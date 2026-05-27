//! macOS selected-text capture (P0-013, PRD §20.1).
//!
//! Approach: write a unique **probe** string to the clipboard, simulate Cmd+C so
//! the frontmost app copies its selection over the probe, poll until the
//! clipboard changes away from the probe, then restore the previous clipboard.
//!
//! Using a probe (rather than diffing against the previous clipboard) avoids two
//! false negatives: a selection that happens to equal the prior clipboard, and a
//! slower app (e.g. Terminal) that hasn't written the pasteboard by a fixed
//! deadline. Polling tolerates that latency; if the clipboard never leaves the
//! probe, nothing was copied (no selection, or copy was blocked).
//!
//! Posting the keystroke needs macOS Accessibility permission; if it's missing
//! the copy is a no-op and we return a [`crate::error::AppError::Capture`] with
//! guidance (FR-018). Restoration is best-effort: non-text clipboard contents
//! (e.g. an image) can't be preserved by this approach (a known §20.1 edge case).

use crate::error::{AppError, Result};
use tauri::AppHandle;

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
pub fn capture_selection(app: &AppHandle) -> Result<String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
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

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        AppError::Capture(format!(
            "Could not initialize input simulation: {e}. Grant TLiquid Accessibility \
             permission in System Settings → Privacy & Security → Accessibility."
        ))
    })?;

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
        return Err(AppError::Capture(format!(
            "Could not simulate copy: {e}. Grant TLiquid Accessibility permission in \
             System Settings → Privacy & Security → Accessibility."
        )));
    }
    interpret(captured)
}

#[cfg(not(target_os = "macos"))]
pub fn capture_selection(_app: &AppHandle) -> Result<String> {
    Err(AppError::Capture(
        "Selected-text capture is only available on macOS in Phase 0.".into(),
    ))
}

/// Map the polled result to a translation input. `Some` means the app wrote a
/// real selection over the probe; `None` means it never did (no selection, or
/// the copy was blocked / Accessibility is off). Pure, so it is unit-tested.
///
/// `cfg`-gated to macOS-or-tests: the non-macOS `capture_selection` stub doesn't
/// call it, so this keeps a non-macOS build warning-free under `-D warnings`.
#[cfg(any(target_os = "macos", test))]
fn interpret(captured: Option<String>) -> Result<String> {
    captured.ok_or_else(|| {
        AppError::Capture(
            "No text was captured. Make sure text is selected, and that TLiquid has \
             Accessibility permission (System Settings → Privacy & Security → Accessibility). \
             If TLiquid isn't listed there, run the installed TLiquid.app — capture can't work \
             when the app is launched from a terminal or `tauri dev`."
                .into(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_text_is_returned() {
        assert_eq!(interpret(Some("hello".into())).unwrap(), "hello");
    }

    #[test]
    fn a_selection_equal_to_the_old_clipboard_still_captures() {
        // The probe approach means an unchanged-looking selection is still real.
        assert_eq!(interpret(Some("same".into())).unwrap(), "same");
    }

    #[test]
    fn nothing_copied_is_a_no_selection_error() {
        assert!(interpret(None).is_err());
    }
}
