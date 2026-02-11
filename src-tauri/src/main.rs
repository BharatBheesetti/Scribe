// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod hotkey;
mod overlay;
mod tray;
mod transcribe;
mod typing;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use audio::AudioRecorder;
use transcribe::PythonService;

struct AppState {
    recorder: Arc<Mutex<AudioRecorder>>,
    python_service: Arc<Mutex<PythonService>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Initialize state
            let state = AppState {
                recorder: Arc::new(Mutex::new(AudioRecorder::new())),
                python_service: Arc::new(Mutex::new(PythonService::new())),
            };

            // Start Python service
            let python_service = state.python_service.clone();
            tauri::async_runtime::spawn(async move {
                let mut service = python_service.lock().unwrap();
                if let Err(e) = service.start().await {
                    eprintln!("Failed to start Python service: {}", e);
                }
            });

            // Setup hotkeys
            if let Err(e) = hotkey::setup_hotkeys(app.handle()) {
                eprintln!("Failed to setup hotkeys: {}", e);
            }

            // Setup system tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("Failed to setup tray: {}", e);
            }

            // Store state
            app.manage(state);

            // Handle hotkey events
            let app_handle = app.handle().clone();
            app.listen("hotkey-pressed", move |_event| {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state: tauri::State<AppState> = app_handle.state();
                    let mut recorder = state.recorder.lock().unwrap();
                    if let Ok(()) = recorder.start_recording() {
                        let _ = tray::update_tray_state(&app_handle, tray::TrayState::Recording);
                        overlay::show_recording(&app_handle);
                        // Register Escape only while recording
                        if let Err(e) = hotkey::register_escape(&app_handle) {
                            eprintln!("Failed to register escape hotkey: {}", e);
                        }
                    }
                });
            });

            let app_handle = app.handle().clone();
            app.listen("hotkey-released", move |_event| {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state: tauri::State<AppState> = app_handle.state();

                    // Unregister Escape now that recording is stopping
                    if let Err(e) = hotkey::unregister_escape(&app_handle) {
                        eprintln!("Failed to unregister escape hotkey: {}", e);
                    }

                    // Stop recording
                    let audio_path = {
                        let mut recorder = state.recorder.lock().unwrap();
                        recorder.stop_recording()
                    };

                    let audio_path = match audio_path {
                        Ok(path) => path,
                        Err(e) => {
                            eprintln!("Recording error: {}", e);
                            let _ = tray::update_tray_state(&app_handle, tray::TrayState::Idle);
                            overlay::hide(&app_handle);
                            return;
                        }
                    };

                    // Update tray to processing
                    let _ = tray::update_tray_state(&app_handle, tray::TrayState::Processing);
                    overlay::show_processing(&app_handle);

                    // Transcribe
                    let result = {
                        let service = state.python_service.lock().unwrap();
                        service.transcribe(audio_path.clone()).await
                    };

                    // Cleanup audio file
                    let _ = std::fs::remove_file(&audio_path);

                    match result {
                        Ok(response) => {
                            // Auto-type the text
                            if let Err(e) = typing::auto_output(&response.text) {
                                eprintln!("Failed to output text: {}", e);
                            }

                            // Show notification
                            let preview = if response.text.len() > 50 {
                                format!("{}...", &response.text[..50])
                            } else {
                                response.text.clone()
                            };

                            app_handle
                                .notification()
                                .builder()
                                .title("Transcribed")
                                .body(preview)
                                .show()
                                .ok();

                            // Show done overlay (auto-hides after 800ms)
                            overlay::show_done(&app_handle);
                        }
                        Err(e) => {
                            eprintln!("Transcription error: {}", e);
                            app_handle
                                .notification()
                                .builder()
                                .title("Transcription Failed")
                                .body("Try again.")
                                .show()
                                .ok();

                            // Hide overlay on error
                            overlay::hide(&app_handle);
                        }
                    }

                    // Return to idle
                    let _ = tray::update_tray_state(&app_handle, tray::TrayState::Idle);
                });
            });

            let app_handle = app.handle().clone();
            app.listen("escape-pressed", move |_event| {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state: tauri::State<AppState> = app_handle.state();

                    // Unregister Escape since recording is being cancelled
                    if let Err(e) = hotkey::unregister_escape(&app_handle) {
                        eprintln!("Failed to unregister escape hotkey: {}", e);
                    }

                    let mut recorder = state.recorder.lock().unwrap();
                    recorder.cancel_recording();
                    let _ = tray::update_tray_state(&app_handle, tray::TrayState::Idle);
                    overlay::hide(&app_handle);

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
