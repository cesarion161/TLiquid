//! Tauri commands exposed to the Svelte windows. These are the only entry
//! points from the frontend into the Rust core.

use crate::config::{self, Settings};
use crate::error::{AppError, Result};
use crate::providers::{self, ProviderId, ProviderMeta, TranslationRequest, TranslationResponse};
use crate::{diagnostics, secrets, shortcuts, translation};
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

/// Re-register the global shortcuts from current settings (e.g. after the user
/// toggles them on/off). Returns messages for any that failed to register (FR-033).
#[tauri::command]
pub fn apply_shortcuts(app: AppHandle) -> Vec<String> {
    shortcuts::apply(&app)
}

/// Registration errors from the most recent shortcut (re)apply, for Settings.
#[tauri::command]
pub fn shortcut_errors(app: AppHandle) -> Vec<String> {
    shortcuts::stored_errors(&app)
}

/// Open macOS System Settings at Privacy & Security → Accessibility, so the user
/// can grant the permission selected-text capture needs (P0-016, FR-018).
#[tauri::command]
pub fn open_accessibility_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map(|_| ())
            .map_err(|e| AppError::Capture(format!("Could not open System Settings: {e}")))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// A local, copy-pasteable diagnostics report for bug reports (FR-065). Contains
/// only non-sensitive metadata and is never uploaded (FR-064).
#[tauri::command]
pub fn diagnostics(app: AppHandle) -> String {
    diagnostics::collect(&app).to_report()
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

/// Resolve routing → build the provider-neutral prompt → look up the Keychain
/// key. The pure orchestration (routing + prompt) lives in `translation`; this
/// adds the I/O (settings load + Keychain lookup) shared by both translate
/// commands.
fn prepare(
    app: &AppHandle,
    request: &TranslationRequest,
) -> Result<(translation::TranslationPlan, String)> {
    let settings = config::load(app);
    let plan = translation::plan(
        &settings,
        request.routing_mode,
        request.explicit_target_language.clone(),
        &request.source_text,
    )?;
    let key = secrets::get_key(request.provider.as_str())?.ok_or_else(|| {
        AppError::Provider(format!(
            "No API key configured for {}.",
            request.provider.as_str()
        ))
    })?;
    Ok((plan, key))
}

/// Assemble the final response from a completed (streamed or not) provider call.
fn finish(
    request: &TranslationRequest,
    plan: translation::TranslationPlan,
    started: std::time::Instant,
    text: String,
) -> TranslationResponse {
    let latency_ms = started.elapsed().as_millis() as u64;
    // Strip only the blank lines models tend to wrap answers in — not all
    // whitespace — so a code-block translation keeps its indentation and inner
    // formatting (the prompt promises to preserve it).
    let translated_text = text.trim_matches(|c| c == '\n' || c == '\r').to_string();
    TranslationResponse {
        translated_text,
        detected_source_language: None,
        target_language: plan.target_language,
        provider: request.provider,
        model: request.model.clone(),
        latency_ms,
    }
}

/// Translate text and return one `TranslationResponse` (non-streaming). The
/// streaming fallback for providers without `supports_streaming`. No source or
/// translated text is persisted (FR-019).
#[tauri::command]
pub async fn translate(app: AppHandle, request: TranslationRequest) -> Result<TranslationResponse> {
    let (plan, key) = prepare(&app, &request)?;
    let started = std::time::Instant::now();
    let text = providers::adapter(request.provider)
        .translate(&key, &request.model, &plan.prompt)
        .await?;
    Ok(finish(&request, plan, started, text))
}

/// One incremental text chunk streamed to the panel (P1-009), sent over the
/// Tauri channel as the provider produces it.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationDelta {
    pub text: String,
}

/// Streaming translation (P1-009): forwards provider deltas to `on_event` as
/// they arrive, then returns the complete `TranslationResponse` so the panel can
/// settle on the trimmed final text (and Enter-to-copy copies the finished
/// result). No translation text is persisted (FR-019).
#[tauri::command]
pub async fn translate_stream(
    app: AppHandle,
    request: TranslationRequest,
    on_event: tauri::ipc::Channel<TranslationDelta>,
) -> Result<TranslationResponse> {
    let (plan, key) = prepare(&app, &request)?;
    let started = std::time::Instant::now();
    let sink = move |delta: String| {
        // A send failure (e.g. the panel closed) shouldn't abort the translation.
        let _ = on_event.send(TranslationDelta { text: delta });
    };
    let text = providers::adapter(request.provider)
        .translate_stream(&key, &request.model, &plan.prompt, &sink)
        .await?;
    Ok(finish(&request, plan, started, text))
}
