use enigo::{Enigo, Key, Keyboard, Settings, Direction};
use clipboard::{ClipboardContext, ClipboardProvider};

pub enum OutputMethod {
    Typed,
    Clipboard,
}

pub fn auto_type_text(text: &str) -> Result<OutputMethod, String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to create Enigo: {}", e))?;

    // Small delay to ensure target app has focus
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Type entire text at once
    enigo.text(text)
        .map_err(|e| format!("Failed to type text: {}", e))?;

    Ok(OutputMethod::Typed)
}

#[allow(dead_code)]
pub fn copy_to_clipboard(text: &str) -> Result<OutputMethod, String> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    ctx.set_contents(text.to_string())
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;

    Ok(OutputMethod::Clipboard)
}

/// Pastes text via clipboard using Ctrl+V, preserving and restoring the
/// original clipboard content. This is the fastest output method and works
/// reliably across all applications.
pub fn clipboard_paste(text: &str) -> Result<OutputMethod, String> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    // 1. Save original clipboard content
    let original_clipboard = ctx.get_contents().ok();

    // 2. Copy transcribed text to clipboard
    ctx.set_contents(text.to_string())
        .map_err(|e| format!("Failed to set clipboard contents: {}", e))?;

    // 3. Simulate Ctrl+V to paste
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to create Enigo: {}", e))?;

    // Small delay to ensure target app has focus
    std::thread::sleep(std::time::Duration::from_millis(50));

    enigo.key(Key::Control, Direction::Press)
        .map_err(|e| format!("Failed key press: {}", e))?;
    enigo.key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Failed key click: {}", e))?;
    enigo.key(Key::Control, Direction::Release)
        .map_err(|e| format!("Failed key release: {}", e))?;

    // 4. Wait for paste to complete
    std::thread::sleep(std::time::Duration::from_millis(100));

    // 5. Restore original clipboard content
    if let Some(original) = original_clipboard {
        // Re-acquire clipboard context to avoid stale state
        if let Ok(mut restore_ctx) = ClipboardProvider::new() as Result<ClipboardContext, _> {
            let _ = restore_ctx.set_contents(original);
        }
    }

    Ok(OutputMethod::Clipboard)
}

pub fn auto_output(text: &str) -> Result<OutputMethod, String> {
    // Try clipboard paste first (fastest, most reliable)
    match clipboard_paste(text) {
        Ok(method) => Ok(method),
        Err(_) => {
            // Fallback to char-by-char typing
            auto_type_text(text)
        }
    }
}
