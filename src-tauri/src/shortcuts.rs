//! Global shortcut registration for macOS (P0-007, P1-002; PRD §10.6.2).
//!
//! Two default shortcuts are registered from settings — primary and secondary
//! translation (FR-028/029) — plus an optional per-additional-language shortcut
//! that translates the selection into that language (P1-002, FR-032). All are
//! user-configurable; a master toggle disables them (FR-034). Registration
//! failures (an accelerator owned by another app) and intra-app conflicts (the
//! same accelerator on two actions) are collected and surfaced in Settings
//! (FR-033). The open-panel hotkey was removed post-Phase-0 (see todo §3a).
//!
//! Every shortcut captures the selection first, then summons the panel and emits
//! a `shortcut` event the panel acts on (P0-013/P0-014/P0-015): primary/secondary
//! use the routing rules; an additional-language shortcut carries its explicit
//! target language.

use crate::providers::Language;
use crate::{capture, windows};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Registration/conflict errors from the last [`apply`], for the Settings UI.
#[derive(Default)]
pub struct ShortcutErrors(pub Mutex<Vec<String>>);

#[derive(Clone)]
enum Action {
    Primary,
    Secondary,
    /// Translate the selection into a specific configured language (P1-002).
    Explicit(Language),
}

/// (Re)register the global shortcuts from current settings, replacing any
/// previously registered. Returns human-readable messages for shortcuts that
/// failed to register or conflict with each other, and stores them in
/// [`ShortcutErrors`].
pub fn apply(app: &AppHandle) -> Vec<String> {
    let _ = app.global_shortcut().unregister_all();

    let settings = crate::config::load(app);
    let mut errors = Vec::new();
    if settings.shortcuts.enabled {
        // The configured (accelerator, action, label) entries, in priority order.
        let mut entries: Vec<(String, Action, String)> = vec![
            (
                settings.shortcuts.translate_primary.clone(),
                Action::Primary,
                "Translate selection".to_string(),
            ),
            (
                settings.shortcuts.translate_secondary.clone(),
                Action::Secondary,
                "Translate to secondary".to_string(),
            ),
        ];
        for lang in &settings.languages.additional {
            if let Some(accel) = lang.shortcut.as_deref().map(str::trim) {
                if !accel.is_empty() {
                    entries.push((
                        accel.to_string(),
                        Action::Explicit(Language {
                            code: lang.code.clone(),
                            name: lang.name.clone(),
                        }),
                        format!("Translate to {}", lang.name),
                    ));
                }
            }
        }

        // Register each unique accelerator once; report duplicates as conflicts
        // so two actions can't fight over the same combo (FR-033).
        let accels: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, _, l)| (a.as_str(), l.as_str()))
            .collect();
        let (to_register, conflicts) = resolve_conflicts(&accels);
        errors.extend(conflicts);
        for i in to_register {
            let (accel, action, _) = &entries[i];
            register(app, accel, action.clone(), &mut errors);
        }
    }

    if let Some(state) = app.try_state::<ShortcutErrors>() {
        *state.0.lock().unwrap() = errors.clone();
    }
    errors
}

/// Unregister all global shortcuts without re-registering (P1-002). Used while
/// the Settings UI records a new shortcut so the combo reaches the webview;
/// [`apply`] restores them afterward.
pub fn pause(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}

/// The registration errors recorded by the most recent [`apply`].
pub fn stored_errors(app: &AppHandle) -> Vec<String> {
    app.try_state::<ShortcutErrors>()
        .map(|s| s.0.lock().unwrap().clone())
        .unwrap_or_default()
}

/// Whether `accelerator` is a valid global-shortcut string the plugin can parse
/// (P1-002 — "invalid shortcuts are rejected"). Used by Settings before saving.
pub fn is_valid(accelerator: &str) -> bool {
    accelerator
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .is_ok()
}

/// Given each entry's `(accelerator, label)` in priority order, return the
/// indices to register (the first occurrence of each accelerator) and a conflict
/// message for every later duplicate. Pure, so the conflict rule is unit-tested.
fn resolve_conflicts(entries: &[(&str, &str)]) -> (Vec<usize>, Vec<String>) {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    let mut to_register = Vec::new();
    let mut conflicts = Vec::new();
    for (i, (accel, label)) in entries.iter().enumerate() {
        match seen.get(accel) {
            Some(other) => conflicts.push(format!(
                "{accel} is assigned to both \"{other}\" and \"{label}\" — change one."
            )),
            None => {
                seen.insert(accel, label);
                to_register.push(i);
            }
        }
    }
    (to_register, conflicts)
}

fn register(app: &AppHandle, accelerator: &str, action: Action, errors: &mut Vec<String>) {
    let result = app
        .global_shortcut()
        .on_shortcut(accelerator, move |app, _shortcut, event| {
            // Fire on key-down only; the plugin also reports key-up.
            if event.state == ShortcutState::Pressed {
                on_trigger(app, &action);
            }
        });
    if let Err(e) = result {
        errors.push(format!("{accelerator} could not be registered: {e}"));
    }
}

/// Payload for the `shortcut` event the panel listens for. Carries the captured
/// selection (or a capture error) plus, for an additional-language shortcut, the
/// explicit `target` language; the panel translates/prefills from it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutPayload {
    action: &'static str,
    text: Option<String>,
    error: Option<String>,
    target: Option<Language>,
}

fn on_trigger(app: &AppHandle, action: &Action) {
    // Capture the selection BEFORE showing our panel, so the simulated Cmd+C
    // targets the app the user is actually in, not TLiquid (P0-013).
    let (text, error) = match capture::capture_selection(app) {
        capture::Capture::Text(t) => (Some(t), None),
        capture::Capture::Failed(msg) => (None, Some(msg)),
        // No selection (permission is fine): do nothing — don't even summon the
        // panel. The user asked for a silent no-op in this case.
        capture::Capture::NoSelection => return,
    };

    let _ = windows::show_panel(app);
    let (action_str, target) = match action {
        Action::Primary => ("primary", None),
        Action::Secondary => ("secondary", None),
        Action::Explicit(lang) => ("explicit", Some(lang.clone())),
    };
    let payload = ShortcutPayload {
        action: action_str,
        text,
        error,
        target,
    };
    let _ = app.emit_to(windows::PANEL_LABEL, "shortcut", payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_accelerator_strings() {
        assert!(is_valid("Cmd+Shift+T"));
        assert!(is_valid("Cmd+Shift+Option+T"));
        assert!(!is_valid("")); // empty
        assert!(!is_valid("NotAKey+++")); // garbage
    }

    #[test]
    fn registers_unique_accelerators_and_reports_conflicts() {
        let entries = [
            ("Cmd+Shift+T", "Translate selection"),
            ("Cmd+Shift+Option+T", "Translate to secondary"),
            ("Cmd+Shift+T", "Translate to German"), // duplicate of #0
            ("Cmd+Shift+R", "Translate to French"),
        ];
        let (to_register, conflicts) = resolve_conflicts(&entries);
        // First occurrence of each accelerator registers; the duplicate doesn't.
        assert_eq!(to_register, vec![0, 1, 3]);
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].contains("Translate selection"));
        assert!(conflicts[0].contains("Translate to German"));
    }

    #[test]
    fn no_conflicts_when_all_unique() {
        let entries = [("Cmd+Shift+T", "a"), ("Cmd+Shift+Option+T", "b")];
        let (to_register, conflicts) = resolve_conflicts(&entries);
        assert_eq!(to_register, vec![0, 1]);
        assert!(conflicts.is_empty());
    }
}
