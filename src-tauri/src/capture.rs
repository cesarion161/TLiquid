//! macOS selected-text capture (P0-013, PRD §20.1).
//!
//! Approach: remember the current clipboard, simulate Cmd+C so the frontmost app
//! copies its selection, read the freshly copied text, then restore the previous
//! clipboard. This is the most broadly compatible path across macOS apps.
//!
//! Posting the keystroke needs macOS Accessibility permission; if it's missing
//! the copy is a no-op and we return a [`crate::error::AppError::Capture`] with
//! guidance (FR-018). Restoration is best-effort: non-text clipboard contents
//! (e.g. an image) can't be preserved by this approach (a known §20.1 edge case).

use crate::error::{AppError, Result};
use tauri::AppHandle;

/// How long to wait for the frontmost app to write the pasteboard after Cmd+C.
#[cfg(target_os = "macos")]
const COPY_SETTLE_MS: u64 = 120;

/// Capture the current selection from the frontmost app. Must run BEFORE the
/// TLiquid panel takes focus, or Cmd+C would target TLiquid itself.
#[cfg(target_os = "macos")]
pub fn capture_selection(app: &AppHandle) -> Result<String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::{thread::sleep, time::Duration};
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let clipboard = app.clipboard();
    // Text we may be able to restore afterwards (None if the clipboard held
    // non-text content or was empty).
    let previous = clipboard.read_text().ok();

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        AppError::Capture(format!(
            "Could not initialize input simulation: {e}. Grant TLiquid Accessibility \
             permission in System Settings → Privacy & Security → Accessibility."
        ))
    })?;

    let copy_result = (|| -> std::result::Result<(), enigo::InputError> {
        enigo.key(Key::Meta, Direction::Press)?;
        enigo.key(Key::Unicode('c'), Direction::Click)?;
        enigo.key(Key::Meta, Direction::Release)?;
        Ok(())
    })();

    // Let the frontmost app handle the synthetic Cmd+C and write the pasteboard.
    sleep(Duration::from_millis(COPY_SETTLE_MS));
    let captured = clipboard.read_text().unwrap_or_default();

    // Restore the previous clipboard text (best-effort).
    if let Some(prev) = &previous {
        let _ = clipboard.write_text(prev.as_str());
    }

    if let Err(e) = copy_result {
        return Err(AppError::Capture(format!(
            "Could not simulate copy: {e}. Grant TLiquid Accessibility permission in \
             System Settings → Privacy & Security → Accessibility."
        )));
    }
    decide(previous.as_deref(), &captured)
}

#[cfg(not(target_os = "macos"))]
pub fn capture_selection(_app: &AppHandle) -> Result<String> {
    Err(AppError::Capture(
        "Selected-text capture is only available on macOS in Phase 0.".into(),
    ))
}

/// Decide the capture outcome from the prior clipboard text and what Cmd+C
/// produced. Pure, so the no-selection / unchanged-clipboard logic is testable.
///
/// `cfg`-gated to macOS-or-tests: the non-macOS `capture_selection` stub doesn't
/// call it, so this keeps a non-macOS build warning-free under `-D warnings`.
#[cfg(any(target_os = "macos", test))]
fn decide(previous: Option<&str>, captured: &str) -> Result<String> {
    if captured.trim().is_empty() {
        return Err(AppError::Capture(
            "No text was captured. Select some text in another app and try again. If this \
             keeps happening, grant TLiquid Accessibility permission in System Settings → \
             Privacy & Security → Accessibility."
                .into(),
        ));
    }
    if Some(captured) == previous {
        // The clipboard is unchanged — most likely nothing was selected. (A
        // selection identical to the prior clipboard is a deliberate, accepted
        // false negative; the robust alternative — polling NSPasteboard's
        // changeCount — is left to a later capture-reliability pass, P1-006.)
        return Err(AppError::Capture(
            "No new text was captured. Select some text in another app and try again.".into(),
        ));
    }
    Ok(captured.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_text_is_returned() {
        assert_eq!(decide(Some("old"), "hello").unwrap(), "hello");
    }

    #[test]
    fn empty_capture_is_a_no_selection_error() {
        assert!(decide(Some("old"), "   ").is_err());
        assert!(decide(None, "").is_err());
    }

    #[test]
    fn unchanged_clipboard_is_treated_as_no_selection() {
        // Cmd+C didn't change the clipboard → nothing was selected.
        assert!(decide(Some("same"), "same").is_err());
    }

    #[test]
    fn capture_succeeds_when_there_was_no_prior_clipboard() {
        assert_eq!(decide(None, "captured").unwrap(), "captured");
    }
}
