use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TrayState {
    Idle,
    Recording,
    Processing,
}

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
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

    // Use a simple colored circle as placeholder icon (you'll replace with actual icons)
    let icon_bytes = include_bytes!("../icons/icon.png");
    let icon = Image::from_bytes(icon_bytes)?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => {
                app.exit(0);
            }
            "settings" => {
                // TODO: Show settings window
                println!("Settings clicked");
            }
            "record" => {
                app.emit("hotkey-pressed", ()).ok();
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // TODO: Show/hide settings window
                println!("Tray clicked");
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray_state(app: &AppHandle, state: TrayState) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Update icon based on state
    // For now, we'll just log the state change
    println!("Tray state changed to: {:?}", state);

    // In a full implementation, you would load different icons:
    // let icon = match state {
    //     TrayState::Idle => Image::from_bytes(include_bytes!("../icons/icon-idle.png"))?,
    //     TrayState::Recording => Image::from_bytes(include_bytes!("../icons/icon-recording.png"))?,
    //     TrayState::Processing => Image::from_bytes(include_bytes!("../icons/icon-processing.png"))?,
    // };
    //
    // if let Some(tray) = app.tray_by_id("main") {
    //     tray.set_icon(Some(icon))?;
    // }

    Ok(())
}
