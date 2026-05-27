//! TLiquid backend: macOS menu-bar shell + translation core.
//!
//! Module responsibilities map to the PRD architecture (§15.3) and TODO epics:
//! - `config`       P0-004  non-secret settings persistence
//! - `secrets`      P0-005  macOS Keychain key storage
//! - `languages`    P0-006  primary/secondary routing engine
//! - `providers`    P0-008  provider abstraction + adapters
//! - `translation`  P0-010  translation orchestrator + prompt templates
//! - `tray`/`windows` P0-002/P0-003  menu-bar shell and on-demand windows

mod commands;
mod config;
mod error;
mod languages;
mod providers;
mod secrets;
mod translation;
mod tray;
mod windows;

pub use error::{AppError, Result};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Single-instance must be registered first so a second launch is caught
    // before the rest of the app initializes (FR-004). Desktop only. A second
    // launch just summons the existing panel.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        let _ = windows::show_panel(app, None);
    }));

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
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

            // Create the panel once, hidden, so summoning it later is an instant
            // show rather than a fresh webview load (PRD §13.2).
            windows::create_panel(app.handle())?;
            tray::create(app.handle())?;

            // In dev, surface the panel immediately so the UI is visible without
            // clicking the tray. Release stays hidden until summoned.
            #[cfg(debug_assertions)]
            windows::show_panel(app.handle(), None)?;

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
            commands::list_provider_models,
            commands::translate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TLiquid");
}
