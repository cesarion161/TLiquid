//! Tauri commands exposed to the Svelte windows. These are the only entry
//! points from the frontend into the Rust core.

use crate::config::{self, Settings};
use crate::error::{AppError, Result};
use crate::languages;
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

/// Test a provider connection with a candidate key the user just typed (FR-040),
/// before it's saved. `Ok(true)` accepted, `Ok(false)` rejected, `Err` failure.
#[tauri::command]
pub async fn test_provider_key(provider: ProviderId, key: String) -> Result<bool> {
    providers::adapter(provider).validate_key(&key).await
}

/// Test the connection using the provider's already-saved key (FR-040), reading
/// it from the Keychain so the frontend never has to hold a saved key.
#[tauri::command]
pub async fn test_provider_connection(provider: ProviderId) -> Result<bool> {
    let key = secrets::get_key(provider.as_str())?.ok_or_else(|| {
        AppError::Provider(format!("No API key configured for {}.", provider.as_str()))
    })?;
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

/// Translate text: resolve routing → build the provider-neutral prompt → look
/// up the Keychain key → call the provider adapter → assemble the response.
/// Non-streaming in Phase 0 (one `TranslationResponse`); streaming is P1-009.
/// No source or translated text is persisted (FR-019).
#[tauri::command]
pub async fn translate(app: AppHandle, request: TranslationRequest) -> Result<TranslationResponse> {
    let settings = config::load(&app);

    let resolution = languages::resolve(
        &settings,
        request.routing_mode,
        request.explicit_target_language.clone(),
    );
    let target_language = resolution
        .display_target()
        .ok_or_else(|| AppError::Provider("No secondary language is configured.".into()))?;

    let prompt = translation::build_prompt(&resolution, &request.source_text);

    let key = secrets::get_key(request.provider.as_str())?.ok_or_else(|| {
        AppError::Provider(format!(
            "No API key configured for {}.",
            request.provider.as_str()
        ))
    })?;

    let started = std::time::Instant::now();
    let translated_text = providers::adapter(request.provider)
        .translate(&key, &request.model, &prompt)
        .await?;
    let latency_ms = started.elapsed().as_millis() as u64;

    // Strip only the blank lines models tend to wrap answers in — not all
    // whitespace — so a code-block translation keeps its indentation and inner
    // formatting (the prompt promises to preserve it).
    let translated_text = translated_text
        .trim_matches(|c| c == '\n' || c == '\r')
        .to_string();

    Ok(TranslationResponse {
        translated_text,
        detected_source_language: None,
        target_language,
        provider: request.provider,
        model: request.model,
        latency_ms,
    })
}
