use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub fn setup_hotkeys(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Register Ctrl+Shift+Space for recording
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

    // Register Escape for cancellation
    let escape_shortcut = Shortcut::new(None, Code::Escape);

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
