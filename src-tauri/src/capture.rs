//! macOS selected-text capture (P0-013, PRD §20.1).
//!
//! Approach: write a unique **probe** string to the clipboard, simulate Cmd+C so
//! the frontmost app copies its selection over the probe, poll until the
//! clipboard changes away from the probe, then restore the previous clipboard.
//!
//! The result is a three-way [`Capture`] so the caller can tell apart the cases
//! the user asked about:
//! - [`Capture::Text`] — a selection was copied → translate it.
//! - [`Capture::NoSelection`] — Accessibility is granted but nothing was copied
//!   (no selection, or the app blocked copy) → do nothing, silently.
//! - [`Capture::Failed`] — capture couldn't run at all, almost always because
//!   Accessibility permission is missing → surface an actionable message.
//!
//! Permission is detected up front: on macOS `Enigo::new` checks
//! `AXIsProcessTrustedWithOptions` and returns `NoPermission` (also triggering
//! the system prompt) when the app isn't trusted. Restoration is best-effort:
//! non-text clipboard contents (e.g. an image) can't be preserved (§20.1 edge).

use tauri::AppHandle;

/// The outcome of a selected-text capture.
pub enum Capture {
    /// A selection was copied.
    Text(String),
    /// Permission is granted but nothing was copied (no selection / copy blocked).
    NoSelection,
    /// Capture couldn't run (e.g. missing Accessibility permission). The string
    /// is an actionable message for the user.
    Failed(String),
}

#[cfg(target_os = "macos")]
mod imp {
    /// Unlikely-to-be-a-real-selection probe written before the synthetic Cmd+C.
    pub const PROBE: &str = "\u{200b}__tliquid_capture_probe__\u{200b}";
    /// Poll cadence and overall budget for the app to write the pasteboard.
    pub const POLL_MS: u64 = 25;
    pub const MAX_WAIT_MS: u64 = 600;
}

/// Capture the current selection from the frontmost app. Must run BEFORE the
/// TLiquid panel takes focus, or Cmd+C would target TLiquid itself.
#[cfg(target_os = "macos")]
pub fn capture_selection(app: &AppHandle) -> Capture {
    use enigo::{Direction, Enigo, Key, Keyboard, NewConError, Settings};
    use imp::{MAX_WAIT_MS, POLL_MS, PROBE};
    use std::{
        thread::sleep,
        time::{Duration, Instant},
    };
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let clipboard = app.clipboard();
    // Text we may be able to restore afterwards (None if the clipboard held
    // non-text content or was empty).
    let previous = clipboard.read_text().ok();

    // Permission check happens here: `Enigo::new` returns NoPermission (and opens
    // the system prompt) when Accessibility isn't granted.
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(enigo) => enigo,
        Err(NewConError::NoPermission) => {
            return Capture::Failed(
                "TLiquid needs Accessibility permission to read the selection (System Settings → \
                 Privacy & Security → Accessibility). If TLiquid is already listed and enabled, \
                 it's likely a stale entry from a previous build — select it, press “−” to remove \
                 it, then try again and re-grant. (Unsigned builds get a new identity each rebuild, \
                 so the old grant no longer applies. Also: capture only works from the built \
                 TLiquid.app, not a terminal / `tauri dev`.)"
                    .into(),
            );
        }
        Err(e) => return Capture::Failed(format!("Could not initialize input simulation: {e}.")),
    };

    // Seed a probe so we can tell "nothing copied" from "copied text equal to the
    // previous clipboard". Best-effort: if it fails, the poll below still works.
    let _ = clipboard.write_text(PROBE);

    let copy_result = (|| -> std::result::Result<(), enigo::InputError> {
        enigo.key(Key::Meta, Direction::Press)?;
        enigo.key(Key::Unicode('c'), Direction::Click)?;
        enigo.key(Key::Meta, Direction::Release)?;
        Ok(())
    })();

    // Poll until the app overwrites the probe with the copied selection.
    let deadline = Instant::now() + Duration::from_millis(MAX_WAIT_MS);
    let mut captured: Option<String> = None;
    loop {
        sleep(Duration::from_millis(POLL_MS));
        if let Ok(text) = clipboard.read_text() {
            if text != PROBE && !text.trim().is_empty() {
                captured = Some(text);
                break;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
    }

    // Restore the previous clipboard (or clear, removing the probe/selection).
    let _ = clipboard.write_text(previous.as_deref().unwrap_or(""));

    if let Err(e) = copy_result {
        return Capture::Failed(format!("Could not simulate copy: {e}."));
    }
    outcome(captured)
}

#[cfg(not(target_os = "macos"))]
pub fn capture_selection(_app: &AppHandle) -> Capture {
    Capture::Failed("Selected-text capture is only available on macOS in Phase 0.".into())
}

/// Map the polled clipboard result to an outcome (permission already verified):
/// copied text → [`Capture::Text`]; nothing copied → [`Capture::NoSelection`].
/// Pure, so it's unit-tested.
///
/// `cfg`-gated to macOS-or-tests so a non-macOS build stays warning-free.
#[cfg(any(target_os = "macos", test))]
fn outcome(captured: Option<String>) -> Capture {
    match captured {
        Some(text) => Capture::Text(text),
        None => Capture::NoSelection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_text_becomes_text() {
        assert!(matches!(outcome(Some("hello".into())), Capture::Text(t) if t == "hello"));
    }

    #[test]
    fn nothing_copied_is_no_selection() {
        assert!(matches!(outcome(None), Capture::NoSelection));
    }
}
