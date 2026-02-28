// System tray integration using tray-icon (which re-exports muda).

use tray_icon::menu::{Menu, MenuEvent, MenuItem, MenuId};
use tray_icon::{Icon, TrayIconBuilder};

/// Holds the MenuIds so we can match events from any thread.
pub struct TrayMenuIds {
    pub show_id: MenuId,
    pub quit_id: MenuId,
}

/// Actions that can be triggered from the system tray.
pub enum TrayAction {
    ShowWindow,
    Quit,
}

/// Create the system tray icon and context menu.
/// Returns (TrayIcon, TrayMenuIds). The TrayIcon must be kept alive (not dropped)
/// for the tray icon to remain visible.
pub fn create_tray() -> Result<(tray_icon::TrayIcon, TrayMenuIds), String> {
    // Load icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_bytes)
        .map_err(|e| format!("Failed to load tray icon: {e}"))?
        .into_rgba8();
    let (w, h) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), w, h)
        .map_err(|e| format!("Failed to create tray icon: {e:?}"))?;

    // Build menu
    let show_item = MenuItem::new("Show Window", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let ids = TrayMenuIds {
        show_id: show_item.id().clone(),
        quit_id: quit_item.id().clone(),
    };

    let menu = Menu::new();
    menu.append(&show_item)
        .map_err(|e| format!("Menu error: {e}"))?;
    menu.append(&quit_item)
        .map_err(|e| format!("Menu error: {e}"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("ReelName")
        .build()
        .map_err(|e| format!("Failed to build tray icon: {e}"))?;

    tracing::info!("System tray created");
    Ok((tray_icon, ids))
}

/// Poll for tray menu events. Returns an action if a menu item was clicked.
pub fn poll_tray_event(ids: &TrayMenuIds) -> Option<TrayAction> {
    if let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id == ids.show_id {
            Some(TrayAction::ShowWindow)
        } else if event.id == ids.quit_id {
            Some(TrayAction::Quit)
        } else {
            None
        }
    } else {
        None
    }
}
