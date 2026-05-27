//! Global shortcut registration for macOS (P0-007, PRD §10.6.2).
//!
//! Three default shortcuts are registered from settings: open-panel, primary
//! translation, and secondary translation (FR-028/029/030). A master toggle can
//! disable them all (FR-034). Registration failures (e.g. an accelerator already
//! owned by another app) are collected and surfaced in Settings (FR-033).
//!
//! Phase 0: every shortcut summons the panel and emits a `shortcut` event; the
//! selected-text capture + translate behavior for primary/secondary is wired in
//! P0-013/P0-014/P0-015, which listen for that event.

use crate::{capture, windows};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Registration errors from the last [`apply`], kept for the Settings UI to show.
#[derive(Default)]
pub struct ShortcutErrors(pub Mutex<Vec<String>>);

#[derive(Clone, Copy)]
enum Action {
    OpenPanel,
    Primary,
    Secondary,
}

/// (Re)register the global shortcuts from current settings, replacing any
/// previously registered. Returns human-readable messages for shortcuts that
/// failed to register, and also stores them in [`ShortcutErrors`].
pub fn apply(app: &AppHandle) -> Vec<String> {
    let _ = app.global_shortcut().unregister_all();

    let settings = crate::config::load(app);
    let mut errors = Vec::new();
    if settings.shortcuts.enabled {
        register(
            app,
            &settings.shortcuts.open_manual_popup,
            Action::OpenPanel,
            &mut errors,
        );
        register(
            app,
            &settings.shortcuts.translate_primary,
            Action::Primary,
            &mut errors,
        );
        register(
            app,
            &settings.shortcuts.translate_secondary,
            Action::Secondary,
            &mut errors,
        );
    }

    if let Some(state) = app.try_state::<ShortcutErrors>() {
        *state.0.lock().unwrap() = errors.clone();
    }
    errors
}

/// The registration errors recorded by the most recent [`apply`].
pub fn stored_errors(app: &AppHandle) -> Vec<String> {
    app.try_state::<ShortcutErrors>()
        .map(|s| s.0.lock().unwrap().clone())
        .unwrap_or_default()
}

fn register(app: &AppHandle, accelerator: &str, action: Action, errors: &mut Vec<String>) {
    let result = app
        .global_shortcut()
        .on_shortcut(accelerator, move |app, _shortcut, event| {
            // Fire on key-down only; the plugin also reports key-up.
            if event.state == ShortcutState::Pressed {
                on_trigger(app, action);
            }
        });
    if let Err(e) = result {
        errors.push(format!("{accelerator} could not be registered: {e}"));
    }
}

/// Payload for the `shortcut` event the panel listens for. For the translation
/// shortcuts it carries the captured selection (or a capture error); the panel
/// translates/prefills from it (P0-014/P0-015).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutPayload {
    action: &'static str,
    text: Option<String>,
    error: Option<String>,
}

fn on_trigger(app: &AppHandle, action: Action) {
    let payload = match action {
        Action::OpenPanel => {
            let _ = windows::show_panel(app, None);
            ShortcutPayload {
                action: "open",
                text: None,
                error: None,
            }
        }
        Action::Primary | Action::Secondary => {
            // Capture the selection BEFORE showing our panel, so the simulated
            // Cmd+C targets the app the user is actually in, not TLiquid (P0-013).
            let (text, error) = match capture::capture_selection(app) {
                Ok(t) => (Some(t), None),
                Err(e) => (None, Some(e.to_string())),
            };
            let _ = windows::show_panel(app, None);
            ShortcutPayload {
                action: if matches!(action, Action::Primary) {
                    "primary"
                } else {
                    "secondary"
                },
                text,
                error,
            }
        }
    };
    let _ = app.emit_to(windows::PANEL_LABEL, "shortcut", payload);
}
