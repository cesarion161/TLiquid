//! Launch-at-login for macOS (P1-001, FR-053/054/055).
//!
//! Thin wrapper over `tauri-plugin-autostart` (a per-user LaunchAgent on macOS).
//! The user's intent lives in `config::Settings.startup.enabled`; this applies
//! it to the OS. The app always starts into menu-bar/Accessory mode (lib.rs), so
//! a login launch is silent — no window, just the tray icon.
//!
//! Note: the LaunchAgent registers the current executable path, so in
//! `pnpm tauri dev` it points at the dev binary; a real login launch is meant
//! for the installed `.app` from a release build.

use tauri::AppHandle;

#[cfg(desktop)]
use tauri_plugin_autostart::ManagerExt;

/// Enable or disable launching TLiquid at login. Errors carry the OS message;
/// they contain no secrets.
#[cfg(desktop)]
pub fn set_enabled(app: &AppHandle, enabled: bool) -> crate::Result<()> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| crate::AppError::Config(format!("launch-at-login: {e}")))
}

/// Whether TLiquid is currently registered to launch at login (the real OS
/// state, not just the stored setting). `false` if it can't be determined.
#[cfg(desktop)]
pub fn is_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// On startup, make the OS launch-at-login state match the saved setting
/// (e.g. after a hand-edit of settings.json, or a first run). Best-effort — a
/// failure must not block launch.
#[cfg(desktop)]
pub fn reconcile(app: &AppHandle) {
    let want = crate::config::load(app).startup.enabled;
    if is_enabled(app) != want {
        if let Err(e) = set_enabled(app, want) {
            log::warn!("could not reconcile launch-at-login: {e}");
        }
    }
}

// Non-desktop (mobile) stubs: launch-at-login is a desktop concept.
#[cfg(not(desktop))]
pub fn set_enabled(_app: &AppHandle, _enabled: bool) -> crate::Result<()> {
    Ok(())
}
#[cfg(not(desktop))]
pub fn is_enabled(_app: &AppHandle) -> bool {
    false
}
#[cfg(not(desktop))]
pub fn reconcile(_app: &AppHandle) {}
