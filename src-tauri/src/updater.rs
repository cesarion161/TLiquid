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

use crate::config;
use crate::error::{AppError, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Event emitted to the panel when a background check finds a newer version
/// (P2-013). The frontend listens for it to light the notification bell.
pub const EVENT_UPDATE_AVAILABLE: &str = "update-available";

/// How often the background poll checks for updates (FR-058). Owner-specified.
const POLL_INTERVAL: Duration = Duration::from_secs(3 * 60 * 60);

/// Delay before the first background check, so startup isn't slowed and the
/// network has a moment to come up (FR-059 — "on startup", just not instantly).
const STARTUP_DELAY: Duration = Duration::from_secs(10);

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

/// Spawn the background auto-update poll (P2-013, FR-058/059). On startup (after
/// a short delay) and then every [`POLL_INTERVAL`], if `updates.auto_check` is
/// enabled it runs [`check`] and, when a newer version exists, emits
/// [`EVENT_UPDATE_AVAILABLE`] to the panel so the bell lights up.
///
/// **Check-only:** it never downloads or installs — the user always clicks
/// "Download & install" (P2-007). The setting is re-read each tick, so toggling
/// auto-check off in Settings stops the polling without a restart. This is the
/// disclosed exception to the Phase-0/1 "no automatic update checks" promise
/// (FR-056); the toggle (default ON) lets users opt out.
pub fn start_auto_check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        loop {
            // Re-read each tick so an off-toggle takes effect without a restart.
            if config::load(&app).updates.auto_check {
                match check(&app).await {
                    Ok(status) if status.available => {
                        // Best-effort: a failed emit (panel gone) just means the
                        // bell isn't lit this tick; the next poll re-emits.
                        let _ = app.emit_to(
                            crate::windows::PANEL_LABEL,
                            EVENT_UPDATE_AVAILABLE,
                            status,
                        );
                    }
                    Ok(_) => {} // up to date — nothing to surface
                    // Network/parse errors are expected (offline, no release yet);
                    // log at warn (no secrets in updater errors) and retry later.
                    Err(e) => log::warn!("background update check failed: {e}"),
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}
