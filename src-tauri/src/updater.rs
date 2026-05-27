//! In-app updates (P2-007 manual check + install; shared by P2-013 auto-check).
//!
//! macOS-focused. Uses the Tauri updater plugin against a `latest.json` published
//! on GitHub Releases (endpoint + minisign public key in `tauri.conf.json`):
//! a **full-bundle replace** (`.app.tar.gz`), not binary patches. The download is
//! **minisign-verified** with the updater's own key — separate from the deferred
//! Apple Developer cert (P1-008) — so updates work even while the app is unsigned.
//!
//! Updates are never silent or forced: `check` only reports whether a newer
//! version exists; the user always clicks `download_and_install` (FR-063, PRD
//! §22.6). Both the manual flow (P2-007) and the background poll (P2-013) call
//! [`check`]; only the user-initiated path calls [`download_and_install`].

use crate::error::{AppError, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Holds the most recent update found by [`check`] so [`download_and_install`]
/// can install the exact object without a second network round-trip. `None` when
/// no newer version is pending. Managed Tauri state (registered in `lib.rs`).
#[derive(Default)]
pub struct PendingUpdate(pub Mutex<Option<Update>>);

/// Outcome of an update check, returned to the UI and reused by the auto-poll.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// Whether a newer version than the running one is available.
    pub available: bool,
    /// The currently running app version.
    pub current_version: String,
    /// The available version (set only when `available`).
    pub version: Option<String>,
    /// Release notes from `latest.json` (when the publisher included a body).
    pub notes: Option<String>,
}

/// One download-progress tick, streamed to the panel over a Tauri channel while
/// the update bundle downloads, so the UI can show a determinate progress bar.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes, when the server reported a `Content-Length`.
    pub total: Option<u64>,
}

/// Check the configured endpoint for a newer version (FR-060/061). Returns the
/// status and, when an update exists, stashes the `Update` in [`PendingUpdate`]
/// so a subsequent install needs no re-fetch. A check finding nothing clears any
/// previously stashed update so a stale one can't be installed later.
pub async fn check(app: &AppHandle) -> Result<UpdateStatus> {
    let current_version = app.package_info().version.to_string();
    let updater = app.updater().map_err(|e| AppError::Update(e.to_string()))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let status = UpdateStatus {
                available: true,
                current_version,
                version: Some(update.version.clone()),
                notes: update.body.clone(),
            };
            *app.state::<PendingUpdate>().0.lock().unwrap() = Some(update);
            Ok(status)
        }
        Ok(None) => {
            // Up to date — drop any update stashed by an earlier check.
            *app.state::<PendingUpdate>().0.lock().unwrap() = None;
            Ok(UpdateStatus {
                available: false,
                current_version,
                version: None,
                notes: None,
            })
        }
        Err(e) => Err(AppError::Update(e.to_string())),
    }
}

/// Download, minisign-verify, and install the pending update found by [`check`],
/// streaming progress over `on_progress` (FR-062/063). The caller relaunches the
/// app afterward. Errors if no update is pending (the UI gates the button on a
/// successful check, but a stale window could still call this).
pub async fn download_and_install(
    app: &AppHandle,
    on_progress: Channel<DownloadProgress>,
) -> Result<()> {
    // Take the pending update out of state (so we never hold the lock across the
    // await), and fail clearly if none was found by a prior check.
    let update = app.state::<PendingUpdate>().0.lock().unwrap().take();
    let Some(update) = update else {
        return Err(AppError::Update(
            "No update is ready to install — check for updates first.".into(),
        ));
    };

    // Accumulate downloaded bytes across chunk callbacks. `Arc<AtomicU64>` keeps
    // the closure `Send + 'static` regardless of the plugin's exact `Fn`/`FnMut`
    // bound, and a cloned channel forwards each tick to the panel.
    let downloaded = Arc::new(AtomicU64::new(0));
    let progress = {
        let downloaded = Arc::clone(&downloaded);
        let on_progress = on_progress.clone();
        move |chunk: usize, total: Option<u64>| {
            let so_far = downloaded.fetch_add(chunk as u64, Ordering::Relaxed) + chunk as u64;
            // A send failure (panel closed) must not abort an in-flight install.
            let _ = on_progress.send(DownloadProgress {
                downloaded: so_far,
                total,
            });
        }
    };

    update
        .download_and_install(progress, || {})
        .await
        .map_err(|e| AppError::Update(e.to_string()))?;
    Ok(())
}
