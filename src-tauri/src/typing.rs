use enigo::{Enigo, Key, KeyboardControllable};
use clipboard::{ClipboardContext, ClipboardProvider};

pub enum OutputMethod {
    Typed,
    Clipboard,
}

pub fn auto_type_text(text: &str) -> Result<OutputMethod, String> {
    let mut enigo = Enigo::new();

    // Small delay to ensure target app has focus
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Type each character
    for c in text.chars() {
        enigo.key_sequence(&c.to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(OutputMethod::Typed)
}

pub fn copy_to_clipboard(text: &str) -> Result<OutputMethod, String> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    ctx.set_contents(text.to_string())
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;

    Ok(OutputMethod::Clipboard)
}

pub fn auto_output(text: &str) -> Result<OutputMethod, String> {
    // Try auto-typing first
    match auto_type_text(text) {
        Ok(method) => Ok(method),
        Err(_) => {
            // Fallback to clipboard
            copy_to_clipboard(text)
        }
    }
}
