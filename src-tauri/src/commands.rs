//! Tauri commands exposed to the Svelte windows. These are the only entry
//! points from the frontend into the Rust core.

use crate::config::{self, Settings};
use crate::error::{AppError, Result};
use crate::languages::{self, Resolution};
use crate::providers::{self, ProviderId, ProviderMeta, TranslationRequest, TranslationResponse};
use crate::{secrets, translation};
use tauri::AppHandle;

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn list_providers() -> Vec<ProviderMeta> {
    providers::all()
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    config::load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<()> {
    config::save(&app, &settings)
}

/// Absolute path to the non-secret settings file, shown in the UI so users can
/// find/edit advanced settings by hand (FR-047, FR-048).
#[tauri::command]
pub fn settings_path(app: AppHandle) -> Result<String> {
    Ok(config::config_path(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn set_provider_key(provider: ProviderId, key: String) -> Result<()> {
    secrets::set_key(provider.as_str(), &key)
}

#[tauri::command]
pub fn delete_provider_key(provider: ProviderId) -> Result<()> {
    secrets::delete_key(provider.as_str())
}

#[tauri::command]
pub fn has_provider_key(provider: ProviderId) -> Result<bool> {
    Ok(secrets::get_key(provider.as_str())?.is_some())
}

/// Test a provider connection with a candidate key (FR-040). Full validation
/// lands with the provider adapters in P0-008/P0-009.
#[tauri::command]
pub async fn test_provider_key(provider: ProviderId, key: String) -> Result<bool> {
    providers::adapter(provider).validate_key(&key).await
}

/// List the models a configured provider offers, for the model picker (FR-041).
#[tauri::command]
pub async fn list_provider_models(provider: ProviderId) -> Result<Vec<String>> {
    let key = secrets::get_key(provider.as_str())?.ok_or_else(|| {
        AppError::Provider(format!("No API key configured for {}.", provider.as_str()))
    })?;
    providers::adapter(provider).list_models(&key).await
}

/// Translate text. The full provider call lands in P0-010; this wires the
/// routing engine, prompt assembly and Keychain lookup that feed it.
#[tauri::command]
pub async fn translate(app: AppHandle, request: TranslationRequest) -> Result<TranslationResponse> {
    let settings = config::load(&app);

    let resolution = languages::resolve(
        &settings,
        request.routing_mode,
        request.explicit_target_language.clone(),
    );
    if matches!(resolution, Resolution::MissingSecondary) {
        return Err(AppError::Provider(
            "No secondary language is configured.".into(),
        ));
    }

    // Prompt assembly is provider-neutral; adapters consume it in P0-010.
    let _prompt = translation::build_prompt(&resolution, &request.source_text);

    let key = secrets::get_key(request.provider.as_str())?.ok_or_else(|| {
        AppError::Provider(format!(
            "No API key configured for {}.",
            request.provider.as_str()
        ))
    })?;

    providers::adapter(request.provider)
        .translate(&request, &key)
        .await
}
