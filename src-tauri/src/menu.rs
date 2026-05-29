//! macOS application menu.
//!
//! TLiquid is a menu-bar accessory app, so this menu is mostly invisible — its
//! job is to own the standard *key equivalents* that a focused window expects:
//! the Edit-menu clipboard shortcuts the translate text fields rely on
//! (Cmd+C/V/X/A, Undo/Redo) and Cmd+W to dismiss the panel.
//!
//! The one deliberate departure from the default Tauri menu: **Cmd+Q hides the
//! panel instead of quitting.** The default menu binds Cmd+Q to the predefined
//! Quit, which sends AppKit `terminate:` — that bypasses Tauri's `ExitRequested`
//! pipeline (tao implements no `applicationShouldTerminate:`), so it cannot be
//! intercepted with `prevent_exit`. Replacing the item with our own Cmd+Q entry
//! is the only way to repurpose it. Keeping the process alive means the global
//! translate hotkey keeps working after Cmd+Q; **real quit lives in the tray
//! menu** ("Quit T·Liquid" → `app.exit`).

#[cfg(target_os = "macos")]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager};

/// Menu id of the Cmd+Q item that hides (rather than quits). Distinct from the
/// tray menu's "quit" id, which still performs a real exit.
pub const CMD_Q_HIDE: &str = "cmd_q_hide";

/// Build and install the macOS application menu. No-op off macOS.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        // App submenu — the system always titles the first submenu with the app
        // name, so the passed title is cosmetic. Mirrors Tauri's default app
        // submenu except the trailing item: a custom Cmd+Q that hides.
        let app_menu = Submenu::with_items(
            app,
            "T·Liquid",
            true,
            &[
                &PredefinedMenuItem::about(app, None, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, CMD_Q_HIDE, "Hide T·Liquid", true, Some("Cmd+Q"))?,
            ],
        )?;

        // Keep Cmd+W dismissing the panel: the predefined close fires a window
        // CloseRequested, which windows.rs turns into a hide (not a teardown).
        let file_menu = Submenu::with_items(
            app,
            "File",
            true,
            &[&PredefinedMenuItem::close_window(app, None)?],
        )?;

        // The translate input/output are real text fields; without these the
        // standard clipboard/selection shortcuts wouldn't work on macOS.
        let edit_menu = Submenu::with_items(
            app,
            "Edit",
            true,
            &[
                &PredefinedMenuItem::undo(app, None)?,
                &PredefinedMenuItem::redo(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::cut(app, None)?,
                &PredefinedMenuItem::copy(app, None)?,
                &PredefinedMenuItem::paste(app, None)?,
                &PredefinedMenuItem::select_all(app, None)?,
            ],
        )?;

        let menu = Menu::with_items(app, &[&app_menu, &file_menu, &edit_menu])?;
        app.set_menu(menu)?;
    }

    #[cfg(not(target_os = "macos"))]
    let _ = app;

    Ok(())
}

/// Global menu-event handler (registered on the builder). Only claims the
/// Cmd+Q-hide item; everything else (incl. the tray menu's own ids) is ignored
/// here and handled by its respective surface.
pub fn on_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    if event.id.as_ref() == CMD_Q_HIDE {
        // Mirror the Esc/Cmd+W dismissal: hide, keeping the menu-bar process (and
        // its global hotkey) alive. Geometry is persisted via the blur handler
        // that fires as the window resigns key (windows.rs).
        if let Some(window) = app.get_webview_window(crate::windows::PANEL_LABEL) {
            let _ = window.hide();
        }
    }
}
