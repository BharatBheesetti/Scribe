use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub fn setup_hotkeys(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Register Ctrl+Shift+Space for recording (always active)
    let recording_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);

    app.global_shortcut().on_shortcut(recording_shortcut, {
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
    })?;

    // Escape is NOT registered here -- it is registered/unregistered dynamically
    // only while recording is active, to avoid interfering with other applications.

    Ok(())
}

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
