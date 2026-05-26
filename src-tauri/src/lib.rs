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
    // before the rest of the app initializes (FR-004). Desktop only.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        let _ = windows::show_main(app);
    }));

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            // macOS: live in the menu bar and stay out of the Dock while idle
            // (FR-007). The tray icon is the app's primary surface.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::create(app.handle())?;

            // In dev, surface the main window immediately so the UI is visible
            // without clicking the tray. Release stays menu-bar-only.
            #[cfg(debug_assertions)]
            windows::show_main(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::list_providers,
            commands::get_settings,
            commands::save_settings,
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
