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
//! - Anchored under the tray icon (using the icon's screen rect) for every
//!   summon — tray click or hotkey — so it always drops down from the menu bar.
//! - Auto-hides when it loses focus (click outside / switch apps), Spotlight-
//!   style; re-summon via the tray icon or a hotkey.

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

/// Label of the one and only panel window. Also the capability target.
pub const PANEL_LABEL: &str = "main";

// Compact utility size; the input and translation areas scroll on overflow.
const PANEL_WIDTH: f64 = 360.0;
const PANEL_HEIGHT: f64 = 400.0;
/// Gap kept between the panel and the screen edges / menu bar.
const MARGIN: f64 = 8.0;

/// Create the panel up front, hidden. Called once during setup.
pub fn create_panel(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(PANEL_LABEL).is_some() {
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(app, PANEL_LABEL, WebviewUrl::App("index.html".into()))
        .title("TLiquid")
        .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
        .resizable(false)
        .decorations(false) // frameless panel; the titlebar is drawn in the UI
        .always_on_top(true) // float above other windows…
        .visible_on_all_workspaces(true) // …including fullscreen Spaces
        .skip_taskbar(true)
        .visible(false) // shown on demand from the tray / hotkey
        .build()?;

    // TLiquid is an always-running menu-bar utility (FR-005). A close gesture
    // (e.g. Cmd+W) must dismiss the panel, not tear it down: closing the only
    // window would otherwise let the app exit. Hiding keeps the process alive
    // in the background and reuses the warm webview on the next summon (PRD §13.2).
    let panel = window.clone();
    window.on_window_event(move |event| match event {
        // A close gesture (e.g. Cmd+W) must dismiss the panel, not tear it down:
        // closing the only window would let the app exit. Hiding keeps the
        // menu-bar process alive and reuses the warm webview (FR-005, PRD §13.2).
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = panel.hide();
        }
        // Auto-hide when focus is lost (click outside / switch apps), like
        // Spotlight/Raycast. Re-summon via the tray icon or a hotkey (PRD §19.2).
        WindowEvent::Focused(false) => {
            let _ = panel.hide();
        }
        _ => {}
    });
    Ok(())
}

/// Summon the panel: anchor it under the tray icon, show, and focus it.
pub fn show_panel(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return Ok(());
    };
    // Best-effort positioning — never block the show on a failed anchor.
    let _ = position_under_tray(&window);
    window.show()?;
    window.set_focus()?;
    Ok(())
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
