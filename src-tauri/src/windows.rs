//! On-demand window creation (P0-002/P0-003).
//!
//! TLiquid keeps UI windows closed while idle (PRD §13.2). Windows are created
//! lazily from the tray and reused if already open.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show_main(app: &AppHandle) -> tauri::Result<()> {
    show(app, "main", "index.html", "TLiquid", 380.0, 520.0)
}

pub fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    show(
        app,
        "settings",
        "settings.html",
        "TLiquid — Settings",
        560.0,
        640.0,
    )
}

fn show(
    app: &AppHandle,
    label: &str,
    path: &str,
    title: &str,
    width: f64,
    height: f64,
) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(label) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, label, WebviewUrl::App(path.into()))
        .title(title)
        .inner_size(width, height)
        .resizable(true)
        .visible(true)
        .build()?;
    Ok(())
}
