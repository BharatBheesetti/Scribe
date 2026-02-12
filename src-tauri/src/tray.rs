use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter,
};

static ICON_IDLE: &[u8] = include_bytes!("../icons/icon-idle.png");
static ICON_RECORDING: &[u8] = include_bytes!("../icons/icon-recording.png");
static ICON_PROCESSING: &[u8] = include_bytes!("../icons/icon-processing.png");

#[derive(Debug, Clone, PartialEq)]
pub enum TrayState {
    Idle,
    Recording,
    Processing,
}

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let record_item = MenuItem::with_id(
        app,
        "record",
        "Start Recording (Ctrl+Shift+Space)",
        true,
        None::<&str>,
    )?;

    let menu = Menu::with_items(app, &[&record_item, &settings_item, &quit_item])?;

    let icon = Image::from_bytes(ICON_IDLE)?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Scribe - Idle")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => {
                app.exit(0);
            }
            "settings" => {
                println!("Settings clicked");
            }
            "record" => {
                app.emit("hotkey-pressed", ()).ok();
            }
            _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                println!("Tray clicked");
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray_state(app: &AppHandle, state: TrayState) -> Result<(), Box<dyn std::error::Error>> {
    let (icon_bytes, tooltip) = match state {
        TrayState::Idle => (ICON_IDLE, "Scribe - Idle"),
        TrayState::Recording => (ICON_RECORDING, "Scribe - Recording..."),
        TrayState::Processing => (ICON_PROCESSING, "Scribe - Processing..."),
    };

    let icon = Image::from_bytes(icon_bytes)?;

    if let Some(tray) = app.tray_by_id("main") {
        tray.set_icon(Some(icon))?;
        tray.set_tooltip(Some(tooltip))?;
        println!("Tray state changed to: {:?}", state);
    } else {
        eprintln!("Warning: tray icon with id 'main' not found");
    }

    Ok(())
}
