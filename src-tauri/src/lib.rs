//! TLiquid backend: macOS menu-bar shell + translation core.
//!
//! Module responsibilities map to the PRD architecture (§15.3) and TODO epics:
//! - `config`       P0-004  non-secret settings persistence
//! - `secrets`      P0-005  macOS Keychain key storage
//! - `languages`    P0-006  primary/secondary routing engine
//! - `providers`    P0-008  provider abstraction + adapters
//! - `translation`  P0-010  translation orchestrator + prompt templates
//! - `capture`      P0-013  macOS selected-text capture (simulated Cmd+C)
//! - `shortcuts`    P0-007  global shortcut registration
//! - `tray`/`windows` P0-002/P0-003  menu-bar shell and on-demand windows

mod capture;
mod commands;
mod config;
mod diagnostics;
mod error;
mod languages;
mod providers;
mod secrets;
mod shortcuts;
mod startup;
mod translation;
mod tray;
mod updater;
mod windows;

pub use error::{AppError, Result};

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Single-instance must be registered first so a second launch is caught
    // before the rest of the app initializes (FR-004). Desktop only. A second
    // launch just summons the existing panel.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        let _ = windows::show_panel(app);
    }));

    // Launch-at-login (P1-001). macOS LaunchAgent; the actual on/off is driven by
    // settings (reconciled in `setup`). No extra args — we start to the tray.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ));

    // In-app updates (P2-007 manual check/install; P2-013 background poll).
    // Endpoint + minisign public key live in `tauri.conf.json`. Desktop only.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            // Persist logs to a file in the app log dir (P1-007) so the
            // diagnostics bundle can include recent log lines + error counts.
            // The file name is fixed (`tliquid.log`) so `diagnostics` can find it.
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("tliquid".into()),
                    },
                ))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            // macOS: live in the menu bar and stay out of the Dock while idle
            // (FR-007). Accessory mode also lets the panel float over fullscreen
            // apps. The tray icon is the app's primary surface.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Materialize the settings file on first run so its location (shown
            // in the UI) is real and users can hand-edit it (FR-047, FR-048).
            // Best-effort: a write failure must not block launch.
            if let Err(e) = config::ensure_initialized(app.handle()) {
                log::warn!("could not initialize settings file: {e}");
            }

            // Make the OS launch-at-login state match the saved setting (P1-001).
            startup::reconcile(app.handle());

            // Create the panel once, hidden, so summoning it later is an instant
            // show rather than a fresh webview load (PRD §13.2).
            windows::create_panel(app.handle())?;
            tray::create(app.handle())?;

            // Holds the update found by the most recent check so the install
            // command can apply it without re-fetching (P2-007).
            app.manage(updater::PendingUpdate::default());

            // Register the default global shortcuts (FR-028/029/030). Failures
            // (e.g. an accelerator owned by another app) are stored for the
            // Settings UI rather than aborting launch (FR-033).
            app.manage(shortcuts::ShortcutErrors::default());
            let errors = shortcuts::apply(app.handle());
            if !errors.is_empty() {
                log::warn!(
                    "some global shortcuts could not be registered ({} failed)",
                    errors.len()
                );
            }

            // In dev, surface the panel immediately so the UI is visible without
            // clicking the tray. Release stays hidden until summoned.
            #[cfg(debug_assertions)]
            windows::show_panel(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::list_providers,
            commands::get_settings,
            commands::save_settings,
            commands::settings_path,
            commands::set_provider_key,
            commands::delete_provider_key,
            commands::has_provider_key,
            commands::test_provider_key,
            commands::test_provider_connection,
            commands::list_provider_models,
            commands::translate,
            commands::translate_stream,
            commands::apply_shortcuts,
            commands::pause_shortcuts,
            commands::shortcut_errors,
            commands::validate_shortcut,
            commands::open_accessibility_settings,
            commands::diagnostics,
            commands::export_diagnostics,
            commands::set_launch_at_login,
            commands::is_launch_at_login,
            commands::check_for_update,
            commands::download_and_install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TLiquid");
}
