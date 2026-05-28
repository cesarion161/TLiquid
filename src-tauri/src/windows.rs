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
//! - Draggable + position-remembering (Raycast-style): on first run it anchors
//!   under the tray icon; the user can drag it anywhere (via the titlebar drag
//!   region) and the position is remembered across hides and restarts. Once it
//!   has a place, summoning reuses it rather than re-anchoring.
//! - Auto-hides when it loses focus (click outside / switch apps), Spotlight-
//!   style; re-summon via the tray icon or a hotkey.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

/// Label of the one and only panel window. Also the capability target.
pub const PANEL_LABEL: &str = "main";

// Compact utility size; the input and translation areas scroll on overflow.
const PANEL_WIDTH: f64 = 360.0;
const PANEL_HEIGHT: f64 = 270.0;
/// Gap kept between the panel and the screen edges / menu bar.
const MARGIN: f64 = 8.0;

/// Set once the user has chosen a position — either by dragging the panel, or
/// by us restoring a previously-dragged position from disk. While false, every
/// summon re-anchors under the tray; once true, the chosen position sticks and
/// is remembered across restarts.
static USER_POSITIONED: AtomicBool = AtomicBool::new(false);

/// Set once the user has resized the panel; like [`USER_POSITIONED`] but for size
/// (P2-012). While false, the default compact size is used and not persisted.
static USER_SIZED: AtomicBool = AtomicBool::new(false);

/// Persisted panel geometry (physical pixels), stored next to `settings.json`.
/// All fields optional so position and size persist independently and a file
/// written before sizing existed (only `x`/`y`) still loads (P2-012 back-compat).
#[derive(Default, Serialize, Deserialize)]
struct SavedWindow {
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

/// Create the panel up front, hidden. Called once during setup.
pub fn create_panel(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(PANEL_LABEL).is_some() {
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(app, PANEL_LABEL, WebviewUrl::App("index.html".into()))
        .title("TLiquid")
        .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
        // Resizable, but never smaller than the compact default (P2-012): the
        // current size is the minimum; the inner layout grows proportionally.
        .min_inner_size(PANEL_WIDTH, PANEL_HEIGHT)
        .resizable(true)
        .decorations(false) // frameless panel; the titlebar is drawn in the UI
        // Transparent so the optional macOS vibrancy shows through (P2-012). When
        // translucency is off, the webview's opaque CSS background fills it, so it
        // looks like a normal solid panel. Needs `app.macOSPrivateApi` in config.
        .transparent(true)
        .always_on_top(true) // float above other windows…
        .visible_on_all_workspaces(true) // …including fullscreen Spaces
        .skip_taskbar(true)
        .visible(false) // shown on demand from the tray / hotkey
        .build()?;

    // Restore a previously-dragged position and/or resized size, each only if the
    // user chose it (guarding a position against a now-disconnected display, and a
    // size against the minimum). This runs while hidden, so the resulting move/
    // resize events aren't seen as user gestures.
    let saved = load_window(app);
    if let (Some(x), Some(y)) = (saved.x, saved.y) {
        if is_on_some_monitor(&window, x, y) {
            let _ = window.set_position(PhysicalPosition::new(x, y));
            USER_POSITIONED.store(true, Ordering::Relaxed);
        }
    }
    if let (Some(w), Some(h)) = (saved.width, saved.height) {
        let _ = window.set_size(PhysicalSize::new(w.max(1), h.max(1)));
        USER_SIZED.store(true, Ordering::Relaxed);
    }

    // Apply the saved translucency preference (P2-012). No-op off macOS.
    apply_translucency(&window, crate::config::load(app).ui.translucent);

    let panel = window.clone();
    window.on_window_event(move |event| match event {
        // A close gesture (e.g. Cmd+W) must dismiss the panel, not tear it down:
        // closing the only window would let the app exit. Hiding keeps the
        // menu-bar process alive and reuses the warm webview (FR-005, PRD §13.2).
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            save_geometry(&panel);
            let _ = panel.hide();
        }
        // Auto-hide when focus is lost (click outside / switch apps), like
        // Spotlight/Raycast. Remember the spot/size if the user changed them.
        WindowEvent::Focused(false) => {
            save_geometry(&panel);
            let _ = panel.hide();
        }
        // We only reposition/resize the panel while it's hidden (before showing),
        // so a move/resize while it's visible is the user doing it — remember that.
        // (Visibility is a match guard rather than a nested `if` so clippy's
        // `collapsible_match` is happy on newer toolchains; a non-visible move/
        // resize falls through to the no-op arm, same as before.)
        WindowEvent::Moved(_) if panel.is_visible().unwrap_or(false) => {
            USER_POSITIONED.store(true, Ordering::Relaxed);
        }
        WindowEvent::Resized(_) if panel.is_visible().unwrap_or(false) => {
            USER_SIZED.store(true, Ordering::Relaxed);
        }
        _ => {}
    });
    Ok(())
}

/// Summon the panel: show and focus it, anchoring under the tray icon only the
/// first time (until it has a remembered/dragged position). Drag it via the
/// titlebar to move it; the new spot is remembered.
pub fn show_panel(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return Ok(());
    };
    // Until the user drags it somewhere, anchor under the tray on every summon
    // (best-effort — never block the show on a failed anchor). Done while still
    // hidden so the reposition isn't mistaken for a user drag.
    if !USER_POSITIONED.load(Ordering::Relaxed) {
        let _ = position_under_tray(&window);
    }
    window.show()?;
    // (Re-)apply the backdrop blur: the NSWindow's `windowNumber` is `0` until
    // first show, so the create-time call is a no-op. Calling on every show is
    // cheap and idempotent, and it also picks up a toggle made via `set_translucency`.
    apply_translucency(&window, crate::config::load(app).ui.translucent);
    window.set_focus()?;
    Ok(())
}

/// Path of the remembered-position file, beside `settings.json`.
fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("window.json"))
}

/// The last-saved panel geometry (position and/or size), or defaults if absent.
fn load_window(app: &AppHandle) -> SavedWindow {
    state_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Persist the panel's current geometry (best-effort) so it reopens the same —
/// position only if the user dragged it, size only if the user resized it, so a
/// default anchor/size is never saved (P2-012).
fn save_geometry(window: &WebviewWindow) {
    let positioned = USER_POSITIONED.load(Ordering::Relaxed);
    let sized = USER_SIZED.load(Ordering::Relaxed);
    if !positioned && !sized {
        return;
    }
    let app = window.app_handle();
    let Some(path) = state_path(app) else {
        return;
    };
    let mut saved = SavedWindow::default();
    if positioned {
        if let Ok(pos) = window.outer_position() {
            saved.x = Some(pos.x);
            saved.y = Some(pos.y);
        }
    }
    if sized {
        if let Ok(size) = window.inner_size() {
            saved.width = Some(size.width);
            saved.height = Some(size.height);
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(&saved) {
        let _ = std::fs::write(path, json);
    }
}

/// Apply or clear the macOS window-level backdrop blur (P2-012). Uses the private
/// CoreGraphics API `CGSSetWindowBackgroundBlurRadius` — the same mechanism Warp,
/// iTerm2 and Alacritty use for an **adjustable** backdrop blur, since
/// `NSVisualEffectView`'s materials have a fixed blur. Combined with the panel's
/// translucent CSS background (alpha = "window opacity"), this is the
/// frosted-glass-with-tunable-radius look. The private API is well-known and
/// stable, but rules TLiquid out of the App Store (we're direct-distribution).
/// `radius = 0` turns it off. No-op off macOS.
pub fn apply_translucency(window: &WebviewWindow, enabled: bool) {
    #[cfg(target_os = "macos")]
    {
        let radius = if enabled { PANEL_BLUR_RADIUS } else { 0 };
        set_window_background_blur(window, radius);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, enabled);
}

/// Default blur radius for the panel backdrop (P2-012). ~30 reads as a frosted
/// glass; raise for more frost, lower for crisper see-through. Easy to expose as
/// a user setting (Warp-style slider) later.
#[cfg(target_os = "macos")]
const PANEL_BLUR_RADIUS: i32 = 30;

/// Set the window's backdrop blur radius via the private CGS API. The window's
/// `windowNumber` is `0` until it's been ordered in (first `show`), so this is
/// a no-op on a hidden window — `show_panel` re-calls `apply_translucency` after
/// the first show so the blur takes effect then.
#[cfg(target_os = "macos")]
fn set_window_background_blur(window: &WebviewWindow, radius: i32) {
    use objc2::{msg_send, runtime::AnyObject};

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGSMainConnectionID() -> i32;
        fn CGSSetWindowBackgroundBlurRadius(connection: i32, window_id: i32, radius: i32) -> i32;
    }

    let Ok(ptr) = window.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    let ns_window = ptr as *mut AnyObject;
    unsafe {
        let window_number: isize = msg_send![ns_window, windowNumber];
        if window_number <= 0 {
            return;
        }
        let conn = CGSMainConnectionID();
        let _ = CGSSetWindowBackgroundBlurRadius(conn, window_number as i32, radius);
    }
}

/// Apply the translucency preference to the panel window by label (P2-012), used
/// by the `set_translucency` command so the toggle takes effect immediately.
pub fn set_translucency(app: &AppHandle, enabled: bool) {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        apply_translucency(&window, enabled);
    }
}

/// Whether `(x, y)` (a window top-left) falls within some connected monitor, so
/// a restored window isn't stranded off-screen after a display change.
fn is_on_some_monitor(window: &WebviewWindow, x: i32, y: i32) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return false;
    };
    monitors.iter().any(|m| {
        let p = m.position();
        let s = m.size();
        x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
    })
}

/// Place the panel just below the tray icon, horizontally centered on it and
/// clamped to the menu-bar monitor. Uses the icon's screen rect so it works for
/// hotkey summons too (not just tray clicks).
fn position_under_tray(window: &WebviewWindow) -> tauri::Result<()> {
    let app = window.app_handle();
    let Some(tray) = app.tray_by_id(crate::tray::TRAY_ID) else {
        return Ok(());
    };
    let Some(rect) = tray.rect()? else {
        return Ok(());
    };

    // The menu bar lives on the primary monitor; use it for scale + clamping.
    let monitor = window.primary_monitor()?.or(window.current_monitor()?);
    let scale = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0);

    let tpos = rect.position.to_physical::<f64>(scale);
    let tsize = rect.size.to_physical::<f64>(scale);
    // Anchor: centered on the icon, just below its bottom edge.
    let anchor = PhysicalPosition::new(tpos.x + tsize.width / 2.0, tpos.y + tsize.height);

    let size = window.outer_size()?;
    let panel = (size.width as f64, size.height as f64);

    let (x, y) = match monitor {
        Some(m) => {
            let pos = m.position();
            let dim = m.size();
            panel_origin(
                anchor,
                panel,
                (pos.x as f64, pos.y as f64),
                (dim.width as f64, dim.height as f64),
                MARGIN,
            )
        }
        None => (anchor.x - panel.0 / 2.0, anchor.y + MARGIN),
    };

    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

/// Compute the panel's top-left origin so it sits just below `anchor` (the tray
/// icon's bottom-center), horizontally centered on it, and fully on the monitor.
///
/// Pure (no Tauri handles) so the clamping rules can be unit-tested:
/// - `panel`/`monitor_size` are `(width, height)`; `monitor_pos` is the
///   monitor's top-left in the same physical-pixel coordinate space as `anchor`.
/// - `margin` is the gap kept from the screen edges.
///
/// When the monitor is narrower/shorter than the panel plus margins, the lower
/// bound wins so the panel stays pinned to the top-left edge rather than
/// drifting off-screen.
fn panel_origin(
    anchor: PhysicalPosition<f64>,
    panel: (f64, f64),
    monitor_pos: (f64, f64),
    monitor_size: (f64, f64),
    margin: f64,
) -> (f64, f64) {
    let (pw, ph) = panel;
    let (mx, my) = monitor_pos;
    let (mw, mh) = monitor_size;

    let min_x = mx + margin;
    let max_x = (mx + mw - pw - margin).max(min_x);
    let x = (anchor.x - pw / 2.0).clamp(min_x, max_x);

    let min_y = my + margin;
    let max_y = (my + mh - ph - margin).max(min_y);
    let y = (anchor.y + margin).clamp(min_y, max_y);

    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_panel_horizontally_under_cursor() {
        // Click in the middle of a roomy 2000-wide monitor at the origin.
        let (x, _) = panel_origin(
            PhysicalPosition::new(1000.0, 10.0),
            (400.0, 560.0),
            (0.0, 0.0),
            (2000.0, 1200.0),
            MARGIN,
        );
        assert_eq!(x, 1000.0 - 200.0); // cursor.x - width/2
    }

    #[test]
    fn drops_below_the_cursor_by_the_margin() {
        let (_, y) = panel_origin(
            PhysicalPosition::new(1000.0, 10.0),
            (400.0, 560.0),
            (0.0, 0.0),
            (2000.0, 1200.0),
            MARGIN,
        );
        assert_eq!(y, 10.0 + MARGIN);
    }

    #[test]
    fn clamps_to_right_edge_when_cursor_is_near_the_corner() {
        // Tray icons sit at the top-right; the panel must not spill off-screen.
        let (x, _) = panel_origin(
            PhysicalPosition::new(1990.0, 10.0),
            (400.0, 560.0),
            (0.0, 0.0),
            (2000.0, 1200.0),
            MARGIN,
        );
        assert_eq!(x, 2000.0 - 400.0 - MARGIN); // flush to the right margin
    }

    #[test]
    fn clamps_to_left_edge_when_cursor_is_near_zero() {
        let (x, _) = panel_origin(
            PhysicalPosition::new(0.0, 10.0),
            (400.0, 560.0),
            (0.0, 0.0),
            (2000.0, 1200.0),
            MARGIN,
        );
        assert_eq!(x, MARGIN);
    }

    #[test]
    fn respects_a_non_zero_monitor_origin() {
        // Secondary monitor whose top-left is at x=2000 (to the right of the primary).
        let (x, y) = panel_origin(
            PhysicalPosition::new(2050.0, 10.0),
            (400.0, 560.0),
            (2000.0, 0.0),
            (1600.0, 900.0),
            MARGIN,
        );
        assert_eq!(x, MARGIN + 2000.0); // clamped to that monitor's left edge
        assert_eq!(y, 10.0 + MARGIN);
    }

    #[test]
    fn clamps_bottom_edge_so_a_tall_panel_stays_on_screen() {
        // Pathologically short monitor: the panel can't fit, so it pins to the top.
        let (_, y) = panel_origin(
            PhysicalPosition::new(500.0, 400.0),
            (400.0, 560.0),
            (0.0, 0.0),
            (1000.0, 500.0),
            MARGIN,
        );
        assert_eq!(y, MARGIN); // lower bound wins
    }
}
