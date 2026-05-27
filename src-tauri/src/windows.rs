//! The single TLiquid panel window (P0-002/P0-003).
//!
//! TLiquid uses ONE window: a frameless menu-bar panel anchored near the tray
//! icon, in the spirit of Raycast, Docker Desktop's tray panel, and JetBrains
//! Toolbox. There is no separate settings/result window — those are views
//! inside this panel (the frontend switches between them). Design choices:
//!
//! - Created once at startup (hidden) so summoning it later is an instant
//!   show/hide instead of a fresh webview load (PRD §13.2 latency/footprint).
//! - `always_on_top` + `visible_on_all_workspaces` + the app's Accessory
//!   activation policy let it float above other apps, including fullscreen
//!   Spaces, so it can be summoned from anywhere (PRD §19.2).
//! - Positioned under the tray icon using the click location reported by the
//!   tray event.

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// Label of the one and only panel window. Also the capability target.
pub const PANEL_LABEL: &str = "main";

const PANEL_WIDTH: f64 = 400.0;
const PANEL_HEIGHT: f64 = 560.0;
/// Gap kept between the panel and the screen edges / menu bar.
const MARGIN: f64 = 8.0;

/// Create the panel up front, hidden. Called once during setup.
pub fn create_panel(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(PANEL_LABEL).is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, PANEL_LABEL, WebviewUrl::App("index.html".into()))
        .title("TLiquid")
        .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
        .resizable(false)
        .decorations(false) // frameless panel; the titlebar is drawn in the UI
        .always_on_top(true) // float above other windows…
        .visible_on_all_workspaces(true) // …including fullscreen Spaces
        .skip_taskbar(true)
        .visible(false) // shown on demand from the tray / hotkey
        .build()?;
    Ok(())
}

/// Show the panel and focus it. When `cursor` is given (a tray click position),
/// the panel is anchored under it; otherwise it opens at its last position.
pub fn show_panel(app: &AppHandle, cursor: Option<PhysicalPosition<f64>>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return Ok(());
    };
    if let Some(cursor) = cursor {
        position_under(&window, cursor)?;
    }
    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// Toggle panel visibility — the tray left-click behavior.
pub fn toggle_panel(app: &AppHandle, cursor: Option<PhysicalPosition<f64>>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return Ok(());
    };
    if window.is_visible().unwrap_or(false) {
        window.hide()
    } else {
        show_panel(app, cursor)
    }
}

/// Place the panel just below `cursor` (the clicked tray icon), horizontally
/// centered on it, clamped to stay fully on the monitor it was summoned from.
fn position_under(window: &WebviewWindow, cursor: PhysicalPosition<f64>) -> tauri::Result<()> {
    let size = window.outer_size()?; // physical pixels
    let mut x = cursor.x - size.width as f64 / 2.0;
    let mut y = cursor.y + MARGIN; // just below where the user clicked in the menu bar

    if let Ok(Some(monitor)) = window.current_monitor() {
        let pos = monitor.position();
        let dim = monitor.size();
        let min_x = pos.x as f64 + MARGIN;
        let max_x = (pos.x + dim.width as i32) as f64 - size.width as f64 - MARGIN;
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.max(pos.y as f64 + MARGIN);
    }

    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}
