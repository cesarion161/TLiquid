//! macOS menu-bar (tray) shell (P0-002).
//!
//! Left-clicking the tray summons the single TLiquid panel, anchored under the
//! icon (Raycast / Docker Desktop tray / JetBrains Toolbox style); it auto-hides
//! when it loses focus (see `windows`). Right-click opens a small menu (Open,
//! Settings…, Quit). "Settings…" opens the panel and asks it to switch to its
//! Settings view via the `navigate` event.

use crate::windows;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter,
};

/// Tray icon id, used to look up its screen rect for panel anchoring.
pub const TRAY_ID: &str = "tliquid";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open T·Liquid", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit T·Liquid", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &separator, &quit])?;

    // Dedicated menu-bar glyph — a full-bleed monochrome "T·" mark, not the padded
    // Dock squircle (`default_window_icon`), which renders small and out-of-place
    // among the bar's template glyphs. Marked as a template image so AppKit fills
    // the bar height and tints it for the light/dark menu bar.
    let icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip("T·Liquid")
        .menu(&menu)
        // Left-click summons the panel; the menu opens on right-click.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = windows::show_panel(app);
            }
            "settings" => {
                let _ = windows::show_panel(app);
                // Ask the panel to switch to its Settings view.
                let _ = app.emit_to(windows::PANEL_LABEL, "navigate", "settings");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = windows::show_panel(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
