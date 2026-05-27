//! Tauri commands exposed to the Svelte windows. These are the only entry
//! points from the frontend into the Rust core.

use crate::config::{self, Settings};
use crate::error::{AppError, Result};
use crate::providers::{self, ProviderId, ProviderMeta, TranslationRequest, TranslationResponse};
use crate::{diagnostics, secrets, shortcuts, startup, translation};
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
pub fn save_settings(app: AppHandle, mut settings: Settings) -> Result<()> {
    // `startup` (launch-at-login + its one-time consent) is server-authoritative:
    // it is changed only via `set_launch_at_login`. The translate view and the
    // settings view each hold their own loaded `Settings`, so a full-object save
    // from a possibly-stale copy must NOT overwrite startup, or it could undo a
    // just-made launch-at-login choice (P1-001). Preserve the on-disk value.
    settings.startup = config::load(&app).startup;
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

/// Temporarily unregister all global shortcuts (P1-002). The Settings UI calls
/// this while recording a new shortcut, so the combo reaches the webview instead
/// of being consumed by an already-registered global hotkey; it re-applies after.
#[tauri::command]
pub fn pause_shortcuts(app: AppHandle) {
    shortcuts::pause(&app);
}

/// Registration errors from the most recent shortcut (re)apply, for Settings.
#[tauri::command]
pub fn shortcut_errors(app: AppHandle) -> Vec<String> {
    shortcuts::stored_errors(&app)
}

/// Whether `accelerator` is a valid global-shortcut string (P1-002). The
/// Settings UI calls this before saving a recorded shortcut so invalid combos
/// are rejected rather than persisted.
#[tauri::command]
pub fn validate_shortcut(accelerator: String) -> bool {
    shortcuts::is_valid(&accelerator)
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

/// A local, copy-pasteable diagnostics bundle for bug reports (FR-065): the
/// non-sensitive metadata, a recent-error summary, and a log tail (P1-007).
/// Never uploaded (FR-064); contains no keys/text (FR-067).
#[tauri::command]
pub fn diagnostics(app: AppHandle) -> String {
    diagnostics::bundle(&app)
}

/// Write the diagnostics bundle to a file the user can attach to a bug report
/// (P1-007), returning its absolute path. Local only — no upload (FR-064).
#[tauri::command]
pub fn export_diagnostics(app: AppHandle) -> Result<String> {
    let bundle = diagnostics::bundle(&app);
    // Co-locate with the logs, in the app log dir.
    let dir = diagnostics::log_file_path(&app)
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .ok_or_else(|| AppError::Config("could not resolve the log directory".into()))?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        AppError::Config(format!("could not create the diagnostics directory: {e}"))
    })?;
    let path = dir.join("tliquid-diagnostics.txt");
    std::fs::write(&path, bundle)
        .map_err(|e| AppError::Config(format!("could not write the diagnostics file: {e}")))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Enable/disable launching TLiquid at login (P1-001, FR-053/054/055). This is
/// the single authoritative path for the `startup` setting: it persists the
/// intent (`enabled` + marks the one-time consent `prompted`) and applies it to
/// the OS, so neither the consent banner nor the toggle has to round-trip a
/// full (possibly stale) settings object.
#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<()> {
    let mut settings = config::load(&app);
    settings.startup.enabled = enabled;
    settings.startup.prompted = true;
    config::save(&app, &settings)?;
    startup::set_enabled(&app, enabled)
}

/// The real OS launch-at-login state (FR-053). Used by Settings to reflect the
/// actual registration, not just the stored setting.
#[tauri::command]
pub fn is_launch_at_login(app: AppHandle) -> bool {
    startup::is_enabled(&app)
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

/// The connection credential an adapter needs: the Keychain API key for the
/// cloud providers, or the configured endpoint URL for Ollama (local, keyless —
/// P1-004). Keeps the [`providers::Provider`] trait unchanged: Ollama just
/// receives its base URL in the slot the cloud adapters use for the key.
fn provider_credential(app: &AppHandle, provider: ProviderId) -> Result<String> {
    if provider == ProviderId::Ollama {
        Ok(config::load(app).providers.ollama_endpoint())
    } else {
        secrets::get_key(provider.as_str())?.ok_or_else(|| {
            AppError::Provider(format!("No API key configured for {}.", provider.as_str()))
        })
    }
}

/// Test the connection using the provider's saved credential (FR-040): the
/// Keychain key for cloud providers, or the configured Ollama endpoint. Reads it
/// in the backend so the frontend never has to hold a saved key.
#[tauri::command]
pub async fn test_provider_connection(app: AppHandle, provider: ProviderId) -> Result<bool> {
    let credential = provider_credential(&app, provider)?;
    providers::adapter(provider).validate_key(&credential).await
}

/// List the models a configured provider offers, for the model picker (FR-041).
#[tauri::command]
pub async fn list_provider_models(app: AppHandle, provider: ProviderId) -> Result<Vec<String>> {
    let credential = provider_credential(&app, provider)?;
    providers::adapter(provider).list_models(&credential).await
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
    // The "credential" is the Keychain key (cloud) or the Ollama endpoint (local).
    let credential = provider_credential(app, request.provider)?;
    Ok((plan, credential))
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
