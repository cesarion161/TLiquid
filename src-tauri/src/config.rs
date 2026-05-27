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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Startup {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ui {
    pub theme: String,
    pub accent_color: String,
    pub open_result_from: String,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcuts {
    pub translate_primary: String,
    pub translate_secondary: String,
    pub open_manual_popup: String,
    /// Master switch for all global shortcuts (FR-034). Defaulted (rather than
    /// required) so a settings file written before this field still loads.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub enabled: bool,
    pub default_model: Option<String>,
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

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            startup: Startup { enabled: false },
            ui: Ui {
                theme: "system".into(),
                accent_color: "default".into(),
                open_result_from: "menu_bar".into(),
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
                open_manual_popup: "Cmd+Option+T".into(),
                enabled: true,
            },
            providers: Providers::default(),
            default_provider: ProviderId::Openai,
            default_model: None,
            output: Output {
                selected_text_behavior: "show_popup".into(),
                copy_on_enter: true,
                replace_selection: false,
            },
            history: History { enabled: false },
            diagnostics: Diagnostics { enabled: false },
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
        let json = r#"{
            "translatePrimary": "Cmd+Shift+T",
            "translateSecondary": "Cmd+Shift+Option+T",
            "openManualPopup": "Cmd+Option+T"
        }"#;
        let shortcuts: Shortcuts = serde_json::from_str(json).unwrap();
        assert!(shortcuts.enabled);
    }

    #[test]
    fn default_settings_serialize_to_camel_case() {
        // The IPC boundary relies on camelCase (CLAUDE.md); guard against an
        // accidental rename-all regression that would break the frontend.
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("\"defaultProvider\""));
        assert!(json.contains("\"openManualPopup\""));
        assert!(!json.contains("\"default_provider\""));
    }
}
