//! Non-secret settings persistence (P0-004).
//!
//! Mirrors the PRD §16 settings model. Stored as `settings.json` in the macOS
//! app config directory. Secrets are NOT stored here — see [`crate::secrets`].

use crate::error::{AppError, Result};
use crate::providers::{Language, ProviderId};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub startup: Startup,
    pub ui: Ui,
    pub languages: Languages,
    pub shortcuts: Shortcuts,
    pub providers: Providers,
    pub default_provider: ProviderId,
    pub default_model: Option<String>,
    pub output: Output,
    pub history: History,
    pub diagnostics: Diagnostics,
    /// In-app update preferences (P2-013). Defaulted so settings files written
    /// before this field still load (FR-049) rather than being treated as corrupt.
    #[serde(default)]
    pub updates: Updates,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Startup {
    pub enabled: bool,
    /// Whether the one-time launch-at-login consent has been shown (P1-001,
    /// FR-054). Defaulted so older settings files load and first-run is detected.
    #[serde(default)]
    pub prompted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ui {
    pub theme: String,
    pub accent_color: String,
    pub open_result_from: String,
    /// macOS translucency/vibrancy behind the panel ("Liquid Glass", P2-012).
    /// Defaulted so older settings files load; respects the system Reduce-
    /// Transparency setting automatically (AppKit renders the material opaque).
    #[serde(default = "default_true")]
    pub translucent: bool,
    /// Multiplier for body/content text size (inputs, output, labels). 1.0 is the
    /// default; the Appearance slider ranges 0.8–1.4. Pure CSS (App.svelte sets
    /// `--tl-fs-scale`), so it persists through `save_settings` like other UI
    /// prefs. Defaulted so settings files written before this field load.
    #[serde(default = "default_font_scale")]
    pub font_scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Languages {
    pub primary: Language,
    pub secondary: Option<Language>,
    pub additional: Vec<AdditionalLanguage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalLanguage {
    pub code: String,
    pub name: String,
    pub enabled: bool,
    /// Optional global shortcut that translates the selection into this language
    /// (P1-002, FR-032). Defaulted so older settings files still load (FR-049).
    #[serde(default)]
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcuts {
    pub translate_primary: String,
    pub translate_secondary: String,
    /// Master switch for all global shortcuts (FR-034). Defaulted (rather than
    /// required) so a settings file written before this field still loads.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_font_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Providers {
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub gemini: ProviderConfig,
    pub openrouter: ProviderConfig,
    pub ollama: ProviderConfig,
}

/// Default Ollama server, used when `providers.ollama.endpoint` is unset/blank.
pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";

impl Providers {
    /// The configured local Ollama endpoint, or the default (P1-004). Unlike the
    /// cloud providers, Ollama is keyless and addressed by URL; this is the
    /// "credential" the orchestrator hands its adapter.
    pub fn ollama_endpoint(&self) -> String {
        self.ollama
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_OLLAMA_ENDPOINT)
            .to_string()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub enabled: bool,
    pub default_model: Option<String>,
    /// Local server URL — Ollama only (other providers leave this `None`).
    /// Defaulted so settings files written before P1-004 still load (FR-049).
    #[serde(default)]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub selected_text_behavior: String,
    pub copy_on_enter: bool,
    pub replace_selection: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Updates {
    /// Automatically poll GitHub for a newer version every few hours and on
    /// startup (P2-013, FR-058/059). **Default ON.** Check-only — a found update
    /// never auto-downloads or installs; the user always clicks to install
    /// (P2-007). Defaulted so older settings files load with auto-check enabled.
    #[serde(default = "default_true")]
    pub auto_check: bool,
}

impl Default for Updates {
    fn default() -> Self {
        Updates { auto_check: true }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            startup: Startup {
                enabled: false,
                prompted: false,
            },
            ui: Ui {
                theme: "system".into(),
                accent_color: "default".into(),
                open_result_from: "menu_bar".into(),
                translucent: true,
                font_scale: 1.0,
            },
            languages: Languages {
                // English is the mandatory default primary language (FR-022).
                primary: Language {
                    code: "en".into(),
                    name: "English".into(),
                },
                secondary: None,
                additional: Vec::new(),
            },
            shortcuts: Shortcuts {
                translate_primary: "Cmd+Shift+T".into(),
                translate_secondary: "Cmd+Shift+Option+T".into(),
                enabled: true,
            },
            providers: Providers {
                // Seed the default Ollama endpoint so the UI shows it from the
                // first run; the cloud providers stay at their derived defaults.
                ollama: ProviderConfig {
                    endpoint: Some(DEFAULT_OLLAMA_ENDPOINT.to_string()),
                    ..ProviderConfig::default()
                },
                ..Providers::default()
            },
            default_provider: ProviderId::Openai,
            default_model: None,
            output: Output {
                selected_text_behavior: "show_popup".into(),
                copy_on_enter: true,
                replace_selection: false,
            },
            history: History { enabled: false },
            diagnostics: Diagnostics { enabled: false },
            updates: Updates::default(),
        }
    }
}

/// Absolute path to the settings file. Shown in the UI so users can edit
/// advanced non-secret settings by hand (FR-047, FR-048).
pub fn config_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Config(e.to_string()))?;
    Ok(dir.join("settings.json"))
}

/// Load settings, falling back to defaults. A corrupt file is backed up rather
/// than discarded silently (PRD §13.3 graceful handling).
pub fn load(app: &AppHandle) -> Settings {
    match config_path(app) {
        Ok(path) => load_from_path(&path),
        Err(_) => Settings::default(),
    }
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<()> {
    let path = config_path(app)?;
    save_to_path(&path, settings)
}

/// Write the defaults file on first run if no settings file exists yet, so the
/// path shown in the UI is real and users can hand-edit it (FR-047, FR-048).
/// Best-effort: an existing (even corrupt) file is left untouched.
pub fn ensure_initialized(app: &AppHandle) -> Result<()> {
    let path = config_path(app)?;
    if !path.exists() {
        save_to_path(&path, &Settings::default())?;
    }
    Ok(())
}

/// Read settings from `path`, falling back to defaults when the file is absent.
/// A present-but-unparseable file is renamed to `*.json.bak` and defaults are
/// used, so a corrupt config is never silently discarded (FR-049, PRD §13.3).
///
/// Split out from [`load`] so the missing/valid/corrupt branches can be tested
/// without a Tauri `AppHandle`.
fn load_from_path(path: &Path) -> Settings {
    let Ok(contents) = std::fs::read_to_string(path) else {
        // Absent (or unreadable) file: first run / fresh defaults.
        return Settings::default();
    };
    match serde_json::from_str::<Settings>(&contents) {
        Ok(settings) => settings,
        Err(e) => {
            log::warn!("settings file unreadable; backing up and using defaults: {e}");
            let _ = std::fs::rename(path, path.with_extension("json.bak"));
            Settings::default()
        }
    }
}

/// Persist `settings` to `path` as pretty JSON, creating parent dirs as needed.
fn save_to_path(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::Config(e.to_string()))?;
    }
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| AppError::Config(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| AppError::Config(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert_eq!(load_from_path(&path), Settings::default());
        // Reading a missing file must not create it.
        assert!(!path.exists());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/settings.json"); // parent created on save

        let mut settings = Settings::default();
        settings.languages.secondary = Some(Language {
            code: "es".into(),
            name: "Spanish".into(),
        });
        settings.default_model = Some("gpt-4.1-mini".into());

        save_to_path(&path, &settings).unwrap();
        assert!(path.exists());
        assert_eq!(load_from_path(&path), settings);
    }

    #[test]
    fn corrupt_file_is_backed_up_and_defaults_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ this is not valid json ").unwrap();

        let loaded = load_from_path(&path);
        assert_eq!(loaded, Settings::default());

        // The corrupt original is preserved as a sibling backup, not deleted.
        let backup = path.with_extension("json.bak");
        assert!(backup.exists(), "corrupt config should be backed up");
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "{ this is not valid json "
        );
    }

    #[test]
    fn shortcuts_without_enabled_field_default_to_enabled() {
        // Forward-compat: a settings file written before `enabled` existed must
        // load with shortcuts enabled (FR-034), not fail and trigger a backup.
        // (A stale `openManualPopup` field from an old config is harmlessly
        // ignored — that shortcut was removed.)
        let json = r#"{
            "translatePrimary": "Cmd+Shift+T",
            "translateSecondary": "Cmd+Shift+Option+T",
            "openManualPopup": "Cmd+Option+T"
        }"#;
        let shortcuts: Shortcuts = serde_json::from_str(json).unwrap();
        assert!(shortcuts.enabled);
    }

    #[test]
    fn ollama_endpoint_falls_back_to_default_when_unset_or_blank() {
        let mut p = Providers::default(); // endpoint None
        assert_eq!(p.ollama_endpoint(), DEFAULT_OLLAMA_ENDPOINT);
        p.ollama.endpoint = Some("   ".into()); // blank → default
        assert_eq!(p.ollama_endpoint(), DEFAULT_OLLAMA_ENDPOINT);
        p.ollama.endpoint = Some("http://10.0.0.2:11434".into());
        assert_eq!(p.ollama_endpoint(), "http://10.0.0.2:11434");
    }

    #[test]
    fn settings_without_updates_field_default_to_auto_check_on() {
        // Forward-compat: a settings file written before P2-013 (no `updates`
        // key) must load with auto-check ON (FR-058 default), not be treated as
        // corrupt. We round-trip the default settings minus the `updates` field.
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("updates");
        let loaded: Settings = serde_json::from_value(value).unwrap();
        assert!(loaded.updates.auto_check);
    }

    #[test]
    fn default_settings_serialize_to_camel_case() {
        // The IPC boundary relies on camelCase (CLAUDE.md); guard against an
        // accidental rename-all regression that would break the frontend.
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("\"defaultProvider\""));
        assert!(json.contains("\"translatePrimary\""));
        assert!(!json.contains("\"default_provider\""));
    }
}
