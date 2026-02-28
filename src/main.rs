mod app;
mod core;
mod db;
mod theme;
mod tray;
mod ui;

use std::sync::OnceLock;

static TRAY_MENU_IDS: OnceLock<tray::TrayMenuIds> = OnceLock::new();

pub fn get_tray_menu_ids() -> Option<&'static tray::TrayMenuIds> {
    TRAY_MENU_IDS.get()
}

fn main() -> iced::Result {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("ReelName starting...");

    // Create system tray (must be created before Iced event loop)
    // _tray_icon must stay alive for the icon to remain visible
    let _tray_icon = match tray::create_tray() {
        Ok((icon, ids)) => {
            let _ = TRAY_MENU_IDS.set(ids);
            Some(icon)
        }
        Err(e) => {
            tracing::warn!("Failed to create system tray: {e}");
            None
        }
    };

    // Run the Iced application
    iced::application(app::App::new, app::App::update, app::App::view)
        .subscription(app::App::subscription)
        .theme(app::App::theme)
        .window_size((1280.0, 800.0))
        .antialiasing(true)
        .exit_on_close_request(false)
        .run()
}
