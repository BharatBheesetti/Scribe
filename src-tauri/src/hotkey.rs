use std::str::FromStr;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

/// Default hotkey string used when settings don't specify one or the stored
/// value fails to parse.
const DEFAULT_HOTKEY: &str = "Ctrl+Shift+Space";

/// Managed state that tracks the currently registered recording shortcut.
pub struct HotkeyState {
    /// The currently registered recording shortcut string (e.g., "Ctrl+Shift+Space").
    /// This is the canonical form produced by `Shortcut::into_string()`.
    pub current_shortcut: Mutex<String>,
}

// ---------------------------------------------------------------------------
// Parsing and validation
// ---------------------------------------------------------------------------

/// Parse a hotkey string into a `Shortcut`. Returns a user-friendly error.
pub fn parse_shortcut_string(s: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(s).map_err(|e| format!("Invalid hotkey \"{}\": {}", s, e))
}

/// Parse and validate a hotkey string. Checks:
/// - Must have at least one modifier (Ctrl, Shift, Alt) unless the key is F1-F24.
/// - Key must NOT be Escape (reserved for cancel-recording).
pub fn validate_hotkey(s: &str) -> Result<Shortcut, String> {
    // Length guard (MEDIUM-1)
    if s.len() > 100 {
        return Err("Hotkey string is too long".to_string());
    }

    let shortcut = parse_shortcut_string(s)?;

    // Check for Escape
    if shortcut.key == Code::Escape {
        return Err("Escape is reserved for cancelling recordings. Choose a different key.".to_string());
    }

    // Check for modifier requirement (unless F-key)
    let is_f_key = matches!(
        shortcut.key,
        Code::F1  | Code::F2  | Code::F3  | Code::F4  |
        Code::F5  | Code::F6  | Code::F7  | Code::F8  |
        Code::F9  | Code::F10 | Code::F11 | Code::F12 |
        Code::F13 | Code::F14 | Code::F15 | Code::F16 |
        Code::F17 | Code::F18 | Code::F19 | Code::F20 |
        Code::F21 | Code::F22 | Code::F23 | Code::F24
    );

    if shortcut.mods.is_empty() && !is_f_key {
        return Err(
            "Hotkey must include at least one modifier (Ctrl, Alt, Shift) unless it is an F-key."
                .to_string(),
        );
    }

    Ok(shortcut)
}

// ---------------------------------------------------------------------------
// Handler factory (shared between setup_hotkeys and change_recording_hotkey)
// ---------------------------------------------------------------------------

/// Create the recording shortcut handler closure.
/// Emits "hotkey-pressed" on press and "hotkey-released" on release.
fn make_recording_handler(
    app: &AppHandle,
) -> impl Fn(&AppHandle, &Shortcut, tauri_plugin_global_shortcut::ShortcutEvent) + Send + Sync + 'static
{
    let app_handle = app.clone();
    move |_app, _shortcut, event| {
        match event.state {
            ShortcutState::Pressed => {
                app_handle.emit("hotkey-pressed", ()).ok();
            }
            ShortcutState::Released => {
                app_handle.emit("hotkey-released", ()).ok();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Setup and dynamic registration
// ---------------------------------------------------------------------------

/// Set up the recording hotkey using the given hotkey string from settings.
/// Falls back to the default `Ctrl+Shift+Space` if the provided string fails to parse.
/// Manages `HotkeyState` in Tauri state.
pub fn setup_hotkeys(app: &AppHandle, hotkey_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try to parse the provided string; fall back to default on failure
    let (shortcut, canonical) = match validate_hotkey(hotkey_str) {
        Ok(s) => {
            let canonical = s.into_string();
            (s, canonical)
        }
        Err(e) => {
            eprintln!(
                "Warning: saved hotkey \"{}\" is invalid ({}). Falling back to default.",
                hotkey_str, e
            );
            let s = parse_shortcut_string(DEFAULT_HOTKEY)
                .expect("Default hotkey must be valid");
            let canonical = s.into_string();
            (s, canonical)
        }
    };

    // Register the shortcut with the handler
    app.global_shortcut()
        .on_shortcut(shortcut, make_recording_handler(app))?;

    // Manage the HotkeyState so other parts of the app can access it
    app.manage(HotkeyState {
        current_shortcut: Mutex::new(canonical),
    });

    Ok(())
}

/// Change the recording hotkey at runtime.
///
/// HIGH-1 fix: Registers the NEW hotkey FIRST, then unregisters the old one.
/// This ensures there's never a window where zero hotkeys are registered.
///
/// Returns the canonical (normalized) hotkey string on success.
pub fn change_recording_hotkey(app: &AppHandle, new_hotkey_str: &str) -> Result<String, String> {
    let new_shortcut = validate_hotkey(new_hotkey_str)?;
    let new_canonical = new_shortcut.into_string();

    // Read the current shortcut from state
    let hotkey_state: tauri::State<HotkeyState> = app
        .try_state()
        .ok_or_else(|| "Hotkey state not initialized".to_string())?;

    let old_canonical = hotkey_state
        .current_shortcut
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    // If the new shortcut is the same as the current one, no-op
    if new_canonical == old_canonical {
        return Ok(new_canonical);
    }

    let old_shortcut = parse_shortcut_string(&old_canonical)
        .map_err(|e| format!("Failed to parse current hotkey: {}", e))?;

    // HIGH-1: Register NEW first. If this fails, old remains active.
    if let Err(e) = app
        .global_shortcut()
        .on_shortcut(new_shortcut, make_recording_handler(app))
    {
        return Err(format!(
            "Failed to register new hotkey \"{}\": {}. Current hotkey \"{}\" is still active.",
            new_hotkey_str, e, old_canonical
        ));
    }

    // New is registered. Now unregister the old one.
    if let Err(e) = app.global_shortcut().unregister(old_shortcut) {
        // Non-fatal: the new hotkey is already active. Log and continue.
        eprintln!(
            "Warning: failed to unregister old hotkey \"{}\": {}",
            old_canonical, e
        );
    }

    // Update state
    *hotkey_state
        .current_shortcut
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = new_canonical.clone();

    Ok(new_canonical)
}

/// Get the current recording hotkey string from managed state.
pub fn current_shortcut_string(app: &AppHandle) -> String {
    match app.try_state::<HotkeyState>() {
        Some(state) => state
            .current_shortcut
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
        None => DEFAULT_HOTKEY.to_string(),
    }
}

/// Temporarily unregister the recording hotkey (for capture mode).
/// Returns the canonical string of the unregistered shortcut.
pub fn unregister_recording_hotkey(app: &AppHandle) -> Result<String, String> {
    let current = current_shortcut_string(app);
    let shortcut = parse_shortcut_string(&current)?;

    app.global_shortcut()
        .unregister(shortcut)
        .map_err(|e| format!("Failed to unregister recording hotkey: {}", e))?;

    Ok(current)
}

/// Re-register the recording hotkey after capture mode ends.
pub fn reregister_recording_hotkey(app: &AppHandle) -> Result<(), String> {
    let current = current_shortcut_string(app);
    let shortcut = parse_shortcut_string(&current)?;

    app.global_shortcut()
        .on_shortcut(shortcut, make_recording_handler(app))
        .map_err(|e| format!("Failed to re-register recording hotkey: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Escape key management (unchanged)
// ---------------------------------------------------------------------------

/// Register the Escape key as a global shortcut. Call when recording starts.
pub fn register_escape(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let escape_shortcut = Shortcut::new(None, Code::Escape);

    // Avoid double-registration if already registered
    if app.global_shortcut().is_registered(escape_shortcut) {
        return Ok(());
    }

    app.global_shortcut().on_shortcut(escape_shortcut, {
        let app_handle = app.clone();
        move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                app_handle.emit("escape-pressed", ()).ok();
            }
        }
    })?;

    Ok(())
}

/// Unregister the Escape key global shortcut. Call when recording stops or is cancelled.
pub fn unregister_escape(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let escape_shortcut = Shortcut::new(None, Code::Escape);

    if app.global_shortcut().is_registered(escape_shortcut) {
        app.global_shortcut().unregister(escape_shortcut)?;
    }

    Ok(())
}
