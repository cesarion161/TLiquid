//! macOS menu-bar (tray) shell (P0-002).

use crate::windows;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open TLiquid", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit TLiquid", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &separator, &quit])?;

    let icon = app
        .default_window_icon()
        .expect("bundled default window icon")
        .clone();

    TrayIconBuilder::with_id("tliquid")
        .icon(icon)
        .tooltip("TLiquid")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = windows::show_main(app);
            }
            "settings" => {
                let _ = windows::show_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
