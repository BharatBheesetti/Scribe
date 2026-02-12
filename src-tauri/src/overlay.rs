use tauri::{AppHandle, Emitter, Manager};

/// Show the overlay window and emit the recording state event.
pub fn show_recording(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("overlay-show-recording", ());
    }
}

/// Emit the processing state event (overlay should already be visible).
pub fn show_processing(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("overlay-show-processing", ());
    }
}

/// Emit the done event, then spawn a task to hide the overlay after 800ms.
pub fn show_done(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("overlay-show-done", ());
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        hide(&app_handle);
    });
}

/// Hide the overlay window and emit the hide event.
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("overlay-hide", ());
        let _ = window.hide();
    }
}
