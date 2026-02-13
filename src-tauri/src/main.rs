// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod history;
mod hotkey;
mod inference;
mod model_manager;
mod overlay;
mod post_process;
mod settings;
mod sounds;
mod state_machine;
mod tray;
mod typing;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_notification::NotificationExt;

use audio::AudioRecorder;
use inference::InferenceEngine;
use state_machine::{RecordingState, HotkeyAction, PostRecordingAction, PostTranscriptionAction};

struct AppState {
    recorder: Arc<Mutex<AudioRecorder>>,
    inference: Arc<Mutex<Option<InferenceEngine>>>,
    recording_state: Arc<Mutex<RecordingState>>,
    active_model: Arc<Mutex<String>>,
    settings: Arc<Mutex<settings::Settings>>,
    history: Arc<Mutex<history::History>>,
    audio_level: Arc<AtomicU32>,  // Shared with AudioRecorder, lock-free VU meter
    sounds: sounds::SoundEffects, // Pre-generated WAV buffers, immutable after init
}

// ---------------------------------------------------------------------------
// Tauri commands (called from frontend via invoke)
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_app_info(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let model = state
        .active_model
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let ready = state
        .inference
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some();
    let models = model_manager::list_models(&model);

    serde_json::json!({
        "model_loaded": ready,
        "active_model": model,
        "models": models,
    })
}

#[tauri::command]
async fn download_model_cmd(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    // Download model from HuggingFace (emits progress events)
    let path = model_manager::download_model(&app, &name).await?;

    // Load model into whisper-rs
    let path_str = path
        .to_str()
        .ok_or("Model path contains invalid characters")?
        .to_string();

    let engine = InferenceEngine::new(path_str).await?;

    {
        *state
            .inference
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(engine);
    }
    {
        *state
            .active_model
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = name.clone();
    }
    {
        *state
            .recording_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = RecordingState::Idle;
    }

    let _ = app.emit("model-ready", ());

    app.notification()
        .builder()
        .title("Scribe")
        .body("Model loaded! Press Ctrl+Shift+Space to start recording.")
        .show()
        .ok();

    Ok(serde_json::json!({ "status": "ok", "model": name }))
}

#[tauri::command]
async fn switch_model_cmd(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    let path = model_manager::path_for_model(&name)?;
    if !path.exists() {
        return Err(format!("Model '{}' is not downloaded", name));
    }

    let path_str = path
        .to_str()
        .ok_or("Model path contains invalid characters")?
        .to_string();

    let engine = InferenceEngine::new(path_str).await?;

    {
        *state
            .inference
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(engine);
    }
    {
        *state
            .active_model
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = name.clone();
    }
    {
        *state
            .recording_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = RecordingState::Idle;
    }

    Ok(serde_json::json!({ "status": "ok", "model": name }))
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
    serde_json::to_value(&*settings).unwrap_or(serde_json::json!({}))
}

#[tauri::command]
fn save_settings(
    state: tauri::State<'_, AppState>,
    new_settings: settings::Settings,
) -> Result<(), String> {
    // HIGH-1 fix: Merge auto_start from current in-memory state instead of
    // accepting it from the frontend. Only set_auto_start can change auto_start.
    let current_auto_start = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .auto_start;

    let mut merged = new_settings;
    merged.auto_start = current_auto_start;

    merged.save()?;
    *state.settings.lock().unwrap_or_else(|e| e.into_inner()) = merged;
    Ok(())
}

#[tauri::command]
fn set_auto_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let autolaunch = app.autolaunch();

    if enabled {
        autolaunch
            .enable()
            .map_err(|e| format!("Failed to enable auto-start: {}", e))?;
    } else {
        autolaunch
            .disable()
            .map_err(|e| format!("Failed to disable auto-start: {}", e))?;
    }

    // Persist to settings file
    let mut settings = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    settings.auto_start = enabled;
    settings.save()?;
    *state.settings.lock().unwrap_or_else(|e| e.into_inner()) = settings;

    Ok(())
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

#[tauri::command]
fn get_history(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let history = state.history.lock().unwrap_or_else(|e| e.into_inner());
    serde_json::to_value(&*history).unwrap_or(serde_json::json!({"entries": []}))
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut history = state.history.lock().unwrap_or_else(|e| e.into_inner());
    history.clear();
    history.save()
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        // Single-instance MUST be first plugin registered
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second instance tried to launch — focus our existing window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--auto-started"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            download_model_cmd,
            switch_model_cmd,
            get_settings,
            save_settings,
            get_history,
            clear_history,
            set_auto_start,
        ])
        .setup(|app| {
            let mut loaded_settings = settings::Settings::load();
            let loaded_history = history::History::load();

            // Detect auto-start launch (--auto-started flag appended by autostart plugin)
            let auto_started = std::env::args().any(|a| a == "--auto-started");
            if auto_started {
                println!("Scribe launched via auto-start");
            }

            // HIGH-2 fix: Sync autostart registry with persisted setting, logging errors
            // and correcting settings to match registry reality on failure.
            {
                use tauri_plugin_autostart::ManagerExt;
                let autolaunch = app.handle().autolaunch();
                let is_enabled = autolaunch.is_enabled().unwrap_or(false);

                if loaded_settings.auto_start && !is_enabled {
                    // Setting says enabled but registry missing — re-enable
                    if let Err(e) = autolaunch.enable() {
                        eprintln!("Failed to re-enable auto-start in registry: {}", e);
                        // Correct settings to match reality
                        loaded_settings.auto_start = false;
                        let _ = loaded_settings.save();
                    }
                } else if !loaded_settings.auto_start && is_enabled {
                    // Setting says disabled but registry present — clean up
                    if let Err(e) = autolaunch.disable() {
                        eprintln!("Failed to disable auto-start in registry: {}", e);
                        // Correct settings to match reality
                        loaded_settings.auto_start = true;
                        let _ = loaded_settings.save();
                    }
                }
            }

            let recorder = AudioRecorder::new();
            let audio_level = recorder.audio_level_arc(); // Get Arc BEFORE Mutex wrap
            let sound_effects = sounds::SoundEffects::new();

            let state = AppState {
                recorder: Arc::new(Mutex::new(recorder)),
                inference: Arc::new(Mutex::new(None)),
                recording_state: Arc::new(Mutex::new(RecordingState::Initializing)),
                active_model: Arc::new(Mutex::new(String::new())),
                settings: Arc::new(Mutex::new(loaded_settings)),
                history: Arc::new(Mutex::new(loaded_history)),
                audio_level,
                sounds: sound_effects,
            };

            // Setup hotkeys
            if let Err(e) = hotkey::setup_hotkeys(app.handle()) {
                eprintln!("Failed to setup hotkeys: {}", e);
                app.handle()
                    .notification()
                    .builder()
                    .title("Scribe")
                    .body("Failed to register hotkeys. Try restarting.")
                    .show()
                    .ok();
            }

            // Setup system tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("Failed to setup tray: {}", e);
            }

            // Store state before spawning async tasks that reference it
            app.manage(state);

            // Try to load default model (or open settings if first run)
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state: tauri::State<AppState> = app_handle.state();

                match model_manager::default_model_path() {
                    Ok(Some(path)) => {
                        // Model exists on disk — load it
                        println!("Loading default model: {:?}", path);
                        let path_str = path.to_str().unwrap_or_default().to_string();

                        match InferenceEngine::new(path_str).await {
                            Ok(engine) => {
                                *state
                                    .inference
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner()) = Some(engine);
                                *state
                                    .active_model
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner()) =
                                    model_manager::DEFAULT_MODEL.to_string();

                                // Model loaded successfully — transition to Idle
                                state_machine::on_model_loaded(&state.recording_state);

                                let _ = app_handle.emit("model-ready", ());

                                app_handle
                                    .notification()
                                    .builder()
                                    .title("Scribe")
                                    .body("Ready! Press Ctrl+Shift+Space to start recording.")
                                    .show()
                                    .ok();
                            }
                            Err(e) => {
                                eprintln!("Failed to load model: {}", e);
                                app_handle
                                    .notification()
                                    .builder()
                                    .title("Model Load Failed")
                                    .body("Could not load speech model. Please re-download from Settings.")
                                    .show()
                                    .ok();
                                // Set to Idle so user can retry after downloading
                                state_machine::on_model_loaded(&state.recording_state);
                                // MEDIUM-2 fix: Always show settings on model failure,
                                // even when auto-started. A broken model is not transient
                                // and will fail every boot, creating a silent degradation loop.
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // First run — no model downloaded. Always show settings window.
                        println!("First run: no model found, opening settings");
                        // Set to Idle so user isn't stuck in Initializing
                        state_machine::on_model_loaded(&state.recording_state);
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    Err(e) => {
                        eprintln!("Error checking for default model: {}", e);
                        app_handle
                            .notification()
                            .builder()
                            .title("Scribe")
                            .body("Could not check for models. Open Settings to download one.")
                            .show()
                            .ok();
                        // Set to Idle so user isn't stuck in Initializing
                        state_machine::on_model_loaded(&state.recording_state);
                        // Show settings so user can resolve the issue
                        if !auto_started {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                }
            });

            // Toggle recording on hotkey press
            let app_handle = app.handle().clone();
            app.listen("hotkey-pressed", move |_event| {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state: tauri::State<AppState> = app_handle.state();

                    // Check if model is loaded (brief lock, released before state transition)
                    let model_loaded = state
                        .inference
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .is_some();

                    // Pure state transition — no nested locks
                    let action = state_machine::on_hotkey_pressed(
                        &state.recording_state,
                        model_loaded,
                    );

                    match action {
                        HotkeyAction::RejectInitializing => {
                            app_handle
                                .notification()
                                .builder()
                                .title("Scribe")
                                .body("Still loading model. Please wait.")
                                .show()
                                .ok();
                            return;
                        }
                        HotkeyAction::RejectProcessing => {
                            println!("Ignoring hotkey: still processing");
                            return;
                        }
                        HotkeyAction::RejectNoModel => {
                            app_handle
                                .notification()
                                .builder()
                                .title("Scribe")
                                .body("No model loaded. Open Settings to download one.")
                                .show()
                                .ok();
                            if let Some(window) =
                                app_handle.get_webview_window("main")
                            {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            return;
                        }
                        HotkeyAction::StartRecording => {
                            // START recording (state already set to Recording)
                            let mut recorder = state
                                .recorder
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            match recorder.start_recording() {
                                Ok(()) => {
                                    // Play start sound if enabled (read setting with brief lock)
                                    {
                                        let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                                        if settings.sound_effects {
                                            state.sounds.play_start_sound();
                                        }
                                    }
                                    let _ = tray::update_tray_state(
                                        &app_handle,
                                        tray::TrayState::Recording,
                                    );
                                    overlay::show_recording(&app_handle);
                                    if let Err(e) = hotkey::register_escape(&app_handle) {
                                        eprintln!("Failed to register escape hotkey: {}", e);
                                    }
                                    println!("Toggle: recording STARTED");

                                    // Start audio level polling (10Hz VU meter updates)
                                    {
                                        let app_for_level = app_handle.clone();
                                        let level_atom = Arc::clone(&state.audio_level);
                                        let rs_for_level = Arc::clone(&state.recording_state);

                                        tauri::async_runtime::spawn(async move {
                                            let mut interval = tokio::time::interval(
                                                std::time::Duration::from_millis(100),
                                            );
                                            loop {
                                                interval.tick().await;

                                                // Exit when no longer recording.
                                                // Catches: manual stop, auto-stop (60s timer),
                                                // escape cancel, and any error that transitions
                                                // state away from Recording.
                                                {
                                                    let rs = rs_for_level
                                                        .lock()
                                                        .unwrap_or_else(|e| e.into_inner());
                                                    if *rs != RecordingState::Recording {
                                                        break;
                                                    }
                                                }

                                                let level = f32::from_bits(
                                                    level_atom.load(Ordering::Relaxed)
                                                );
                                                let _ = app_for_level.emit("audio-level", level);
                                            }
                                        });
                                    }
                                }
                                Err(e) => {
                                    // Revert state to Idle on failure
                                    state_machine::on_recording_start_failed(
                                        &state.recording_state,
                                    );
                                    eprintln!("Failed to start recording: {}", e);
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Recording Failed")
                                        .body(format!("{}", e))
                                        .show()
                                        .ok();
                                }
                            }
                        }
                        HotkeyAction::StopAndTranscribe => {
                            // STOP recording and transcribe (state already set to Processing)
                            println!("Toggle: recording STOPPED, starting transcription");

                            // Unregister Escape
                            if let Err(e) = hotkey::unregister_escape(&app_handle) {
                                eprintln!("Failed to unregister escape hotkey: {}", e);
                            }

                            // Stop recording and get 16kHz mono samples
                            // The mic stream is CLOSED after this returns (audio thread joined).
                            let samples_result = {
                                let mut recorder = state
                                    .recorder
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner());
                                recorder.stop_recording()
                            };

                            // Play stop sound AFTER mic is closed -- prevents feedback loop.
                            // Safe: PlaySound uses MME output, completely separate from WASAPI input.
                            {
                                let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                                if settings.sound_effects {
                                    state.sounds.play_stop_sound();
                                }
                            }

                            // Pure state evaluation — sets Idle on error/short/empty
                            let post_action = state_machine::evaluate_recording(
                                &state.recording_state,
                                &samples_result,
                            );

                            let samples = match post_action {
                                PostRecordingAction::Transcribe => {
                                    // Safe to unwrap: evaluate_recording returns Transcribe only for Ok with enough samples
                                    samples_result.unwrap()
                                }
                                PostRecordingAction::EmptyRecording => {
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Empty Recording")
                                        .body("No audio was captured.")
                                        .show()
                                        .ok();
                                    let _ = tray::update_tray_state(
                                        &app_handle,
                                        tray::TrayState::Idle,
                                    );
                                    overlay::hide(&app_handle);
                                    return;
                                }
                                PostRecordingAction::TooShort => {
                                    println!("Recording too short");
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Recording Too Short")
                                        .body("Hold longer and speak. Minimum 0.5 seconds.")
                                        .show()
                                        .ok();
                                    let _ = tray::update_tray_state(
                                        &app_handle,
                                        tray::TrayState::Idle,
                                    );
                                    overlay::hide(&app_handle);
                                    return;
                                }
                                PostRecordingAction::RecordingError(e) => {
                                    eprintln!("Recording error: {}", e);
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Recording Error")
                                        .body(format!("{}", e))
                                        .show()
                                        .ok();
                                    let _ = tray::update_tray_state(
                                        &app_handle,
                                        tray::TrayState::Idle,
                                    );
                                    overlay::hide(&app_handle);
                                    return;
                                }
                            };

                            // Update tray to processing
                            let _ = tray::update_tray_state(
                                &app_handle,
                                tray::TrayState::Processing,
                            );
                            overlay::show_processing(&app_handle);

                            // Get a clone of the inference engine (brief lock)
                            let engine = {
                                state
                                    .inference
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .clone()
                            };

                            // Read language setting (brief lock)
                            let language = {
                                let s = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                                let lang = s.language.clone();
                                if lang == "auto" { None } else { Some(lang) }
                            };

                            // Capture sample count before moving samples into transcribe
                            let samples_len = samples.len();

                            // Transcribe
                            let result = match engine {
                                Some(engine) => {
                                    engine
                                        .transcribe(samples, language)
                                        .await
                                }
                                None => Err("No model loaded".to_string()),
                            };

                            // Pure state evaluation — always transitions to Idle
                            let post_action = state_machine::evaluate_transcription(
                                &state.recording_state,
                                &result,
                            );

                            match post_action {
                                PostTranscriptionAction::OutputText(ref text) => {
                                    println!("Raw transcription: {:?}", text);

                                    // Read settings for filler removal, language, and output mode (single brief lock)
                                    let (filler_removal, language, output_mode) = {
                                        let s = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                                        (s.filler_removal, s.language.clone(), s.output_mode.clone())
                                    };

                                    // Post-process: filler removal + text cleanup
                                    let cleaned = post_process::clean_transcription(text, filler_removal, &language);
                                    // If cleaning produced empty string (all content was filler), fall back to raw
                                    let final_text = if cleaned.is_empty() { text.clone() } else { cleaned };

                                    println!("After cleanup: {:?}", final_text);

                                    // Auto-paste text into the active app
                                    if let Err(e) = typing::auto_output(&final_text, &output_mode) {
                                        eprintln!("Failed to output text: {}", e);
                                        app_handle
                                            .notification()
                                            .builder()
                                            .title("Paste Failed")
                                            .body("Text copied to clipboard. Paste manually with Ctrl+V.")
                                            .show()
                                            .ok();
                                    }

                                    // Show notification with preview (safe UTF-8 truncation)
                                    let preview: String = if final_text.chars().count() > 50 {
                                        let truncated: String =
                                            final_text.chars().take(50).collect();
                                        format!("{}...", truncated)
                                    } else {
                                        final_text.clone()
                                    };

                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Transcribed")
                                        .body(preview)
                                        .show()
                                        .ok();

                                    overlay::show_done(&app_handle);

                                    // Save to history -- uses cleaned text
                                    {
                                        let mut hist = state.history.lock().unwrap_or_else(|e| e.into_inner());
                                        let model_name = state.active_model.lock().unwrap_or_else(|e| e.into_inner()).clone();
                                        let lang = state.settings.lock().unwrap_or_else(|e| e.into_inner()).language.clone();
                                        let duration_secs = samples_len as f64 / 16000.0;
                                        hist.add_entry(history::HistoryEntry {
                                            timestamp: current_timestamp(),
                                            text: final_text.clone(),
                                            duration_seconds: duration_secs,
                                            model: model_name,
                                            language: lang,
                                        });
                                        let _ = hist.save();
                                    }
                                }
                                PostTranscriptionAction::NoSpeechDetected => {
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("No Speech Detected")
                                        .body(
                                            "Try speaking louder or check your microphone.",
                                        )
                                        .show()
                                        .ok();
                                    overlay::hide(&app_handle);
                                }
                                PostTranscriptionAction::TranscriptionError(ref e) => {
                                    eprintln!("Transcription error: {}", e);
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Transcription Failed")
                                        .body(format!("{}", e))
                                        .show()
                                        .ok();
                                    overlay::hide(&app_handle);
                                }
                            }

                            // Tray back to idle (state already set by evaluate_transcription)
                            let _ = tray::update_tray_state(
                                &app_handle,
                                tray::TrayState::Idle,
                            );
                        }
                    }
                });
            });

            // Cancel recording via Escape
            let app_handle = app.handle().clone();
            app.listen("escape-pressed", move |_event| {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state: tauri::State<AppState> = app_handle.state();

                    // Only cancel if currently recording
                    let cancelled = state_machine::on_escape_pressed(
                        &state.recording_state,
                    );
                    if !cancelled {
                        return;
                    }

                    if let Err(e) = hotkey::unregister_escape(&app_handle) {
                        eprintln!("Failed to unregister escape hotkey: {}", e);
                    }

                    let mut recorder = state
                        .recorder
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    recorder.cancel_recording();
                    let _ = tray::update_tray_state(&app_handle, tray::TrayState::Idle);
                    overlay::hide(&app_handle);

                    println!("Recording cancelled via Escape");

                    app_handle
                        .notification()
                        .builder()
                        .title("Recording Cancelled")
                        .body("")
                        .show()
                        .ok();
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
