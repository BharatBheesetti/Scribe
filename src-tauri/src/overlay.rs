use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Manager};

/// Generation counter to prevent timer race conditions.
/// Every show_* call increments this; show_done only hides if the
/// generation hasn't changed during the 800ms wait.
static OVERLAY_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Result of probing for cursor/caret position.
enum CursorProbeResult {
    /// Got the text caret position from the foreground window's GUI thread
    CaretPosition { x: i32, y: i32 },
    /// Fell back to mouse cursor position
    MousePosition { x: i32, y: i32 },
    /// Could not determine any position
    NoPosition,
}

/// Probe for the best position to place the overlay.
/// Tier 1: Win32 GetGUIThreadInfo for text caret (works in Notepad, Word, native apps)
/// Tier 2: GetCursorPos for mouse position (always works)
/// Tier 3: No position available
fn probe_cursor_position() -> CursorProbeResult {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetGUIThreadInfo, GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId,
            GUITHREADINFO,
        };
        use windows::Win32::Foundation::POINT;

        unsafe {
            // Try tier 1: text caret from foreground window
            let fg_window = GetForegroundWindow();
            if fg_window.0 != 0 {
                let thread_id = GetWindowThreadProcessId(fg_window, None);
                if thread_id != 0 {
                    let mut gui_info = GUITHREADINFO {
                        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                        ..Default::default()
                    };
                    if GetGUIThreadInfo(thread_id, &mut gui_info).is_ok() {
                        let caret = gui_info.rcCaret;
                        // Check if caret rect is non-empty (app exposes caret info)
                        if caret.right > caret.left && caret.bottom > caret.top {
                            // caret coords are client-relative; convert to screen coords
                            let hwnd_focus = gui_info.hwndFocus;
                            if hwnd_focus.0 != 0 {
                                let mut pt = POINT {
                                    x: caret.left,
                                    y: caret.bottom,
                                };
                                use windows::Win32::Graphics::Gdi::ClientToScreen;
                                if ClientToScreen(hwnd_focus, &mut pt).as_bool() {
                                    return CursorProbeResult::CaretPosition {
                                        x: pt.x,
                                        y: pt.y,
                                    };
                                }
                            }
                        }
                    }
                }
            }

            // Tier 2: mouse cursor position
            let mut pt = POINT::default();
            if GetCursorPos(&mut pt).is_ok() {
                // Offset below and right to avoid overlapping the cursor
                return CursorProbeResult::MousePosition {
                    x: pt.x + 10,
                    y: pt.y + 20,
                };
            }
        }
    }

    CursorProbeResult::NoPosition
}

/// Get the work area of the monitor containing the given point.
/// Returns (x, y, width, height) of the work area.
fn get_work_area_for_point(px: i32, py: i32) -> (i32, i32, i32, i32) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Gdi::{
            MonitorFromPoint, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        };
        use windows::Win32::Foundation::POINT;

        unsafe {
            let pt = POINT { x: px, y: py };
            let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
                let rc = mi.rcWork;
                return (rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top);
            }
        }
    }

    // Fallback: assume 1920x1080
    (0, 0, 1920, 1080)
}

/// Calculate the overlay position, clamping to screen edges.
/// Returns (logical_x, logical_y, used_fallback) where used_fallback indicates
/// that no cursor position was found and we used the default position.
fn calculate_overlay_position(scale_factor: f64) -> (f64, f64, bool) {
    let overlay_w = 140.0;
    let overlay_h = 36.0;

    let probe = probe_cursor_position();

    match probe {
        CursorProbeResult::CaretPosition { x, y } | CursorProbeResult::MousePosition { x, y } => {
            let (wa_x, wa_y, wa_w, wa_h) = get_work_area_for_point(x, y);

            // Convert physical pixels to logical
            let lx = x as f64 / scale_factor;
            let ly = y as f64 / scale_factor;
            let wa_lx = wa_x as f64 / scale_factor;
            let wa_ly = wa_y as f64 / scale_factor;
            let wa_lw = wa_w as f64 / scale_factor;
            let wa_lh = wa_h as f64 / scale_factor;

            // Clamp to work area
            let mut final_x = lx;
            let mut final_y = ly + 4.0; // Small gap below caret/cursor

            // If overlay would go off right edge, shift left
            if final_x + overlay_w > wa_lx + wa_lw {
                final_x = wa_lx + wa_lw - overlay_w - 8.0;
            }
            // If overlay would go off bottom edge, place above caret
            if final_y + overlay_h > wa_ly + wa_lh {
                final_y = ly - overlay_h - 8.0;
            }
            // Ensure not off left/top
            if final_x < wa_lx {
                final_x = wa_lx + 8.0;
            }
            if final_y < wa_ly {
                final_y = wa_ly + 8.0;
            }

            (final_x, final_y, false)
        }
        CursorProbeResult::NoPosition => {
            // Default top-left
            (20.0, 20.0, true)
        }
    }
}

/// Show the overlay window near the cursor and emit the recording state event.
/// NEVER call set_focus() -- it steals focus from the user's active app,
/// causing text to paste into the overlay instead of their target window.
/// Returns `true` if no cursor position was found (fallback position used),
/// indicating clipboard-only mode should be used.
pub fn show_recording(app: &AppHandle) -> bool {
    OVERLAY_GENERATION.fetch_add(1, Ordering::SeqCst);

    let mut used_fallback = false;

    if let Some(window) = app.get_webview_window("overlay") {
        let scale = window
            .primary_monitor()
            .ok()
            .flatten()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);

        let (x, y, fallback) = calculate_overlay_position(scale);
        used_fallback = fallback;

        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        let _ = window.set_size(tauri::LogicalSize::new(140.0, 36.0));
        let _ = window.show();
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.emit("overlay-show-recording", ());
    }

    used_fallback
}

/// Emit the processing state event (overlay should already be visible).
pub fn show_processing(app: &AppHandle) {
    OVERLAY_GENERATION.fetch_add(1, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("overlay-show-processing", ());
    }
}

/// Emit the done event, then spawn a task to hide the overlay after 800ms.
/// Uses the generation counter to avoid hiding if a new show_* call arrived
/// during the wait (e.g. user starts a new recording within 800ms).
pub fn show_done(app: &AppHandle) {
    let gen = OVERLAY_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("overlay-show-done", ());
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        // Only hide if no new overlay state was shown during the 800ms wait
        if OVERLAY_GENERATION.load(Ordering::SeqCst) == gen {
            hide(&app_handle);
        }
    });
}

/// Hide the overlay window and emit the hide event.
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.emit("overlay-hide", ());
        let _ = window.hide();
    }
}
