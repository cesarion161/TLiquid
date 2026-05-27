//! Local diagnostics export (P0-016, FR-064/FR-065, PRD §10.6.6).
//!
//! Produces a plain-text summary the user can copy into a bug report. It is
//! **never uploaded** — there is no TLiquid server. By construction it contains
//! only non-sensitive metadata: app/OS info and settings *shape*. It must never
//! include API keys, translation text, clipboard contents, prompts, or provider
//! responses (the struct has no field for any of those).

use crate::config;
use crate::providers;
use crate::secrets;
use tauri::AppHandle;

/// Non-sensitive snapshot for a bug report. Note the absence of any text/key field.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostics {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub default_provider: String,
    pub default_model: Option<String>,
    pub primary_language: String,
    pub secondary_language: Option<String>,
    pub additional_languages: usize,
    pub shortcuts_enabled: bool,
    /// Provider ids that have a key configured — presence only, never the key.
    pub configured_providers: Vec<String>,
}

/// Gather diagnostics from settings + which providers have a saved key.
pub fn collect(app: &AppHandle) -> Diagnostics {
    let settings = config::load(app);
    let configured_providers = providers::all()
        .into_iter()
        .map(|m| m.id)
        .filter(|id| secrets::get_key(id.as_str()).ok().flatten().is_some())
        .map(|id| id.as_str().to_string())
        .collect();

    Diagnostics {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        default_provider: settings.default_provider.as_str().to_string(),
        default_model: settings.default_model.clone(),
        primary_language: settings.languages.primary.code.clone(),
        secondary_language: settings
            .languages
            .secondary
            .as_ref()
            .map(|l| l.code.clone()),
        additional_languages: settings.languages.additional.len(),
        shortcuts_enabled: settings.shortcuts.enabled,
        configured_providers,
    }
}

impl Diagnostics {
    /// Render a compact, copy-pasteable report.
    pub fn to_report(&self) -> String {
        let configured = if self.configured_providers.is_empty() {
            "none".to_string()
        } else {
            self.configured_providers.join(", ")
        };
        format!(
            "TLiquid diagnostics\n\
             version: {version}\n\
             os: {os} ({arch})\n\
             default provider: {provider}\n\
             default model: {model}\n\
             primary language: {primary}\n\
             secondary language: {secondary}\n\
             additional languages: {additional}\n\
             global shortcuts enabled: {shortcuts}\n\
             providers configured: {configured}",
            version = self.version,
            os = self.os,
            arch = self.arch,
            provider = self.default_provider,
            model = self.default_model.as_deref().unwrap_or("none"),
            primary = self.primary_language,
            secondary = self.secondary_language.as_deref().unwrap_or("none"),
            additional = self.additional_languages,
            shortcuts = self.shortcuts_enabled,
            configured = configured,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Diagnostics {
        Diagnostics {
            version: "0.1.0".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
            default_provider: "openai".into(),
            default_model: Some("gpt-4o".into()),
            primary_language: "en".into(),
            secondary_language: Some("es".into()),
            additional_languages: 2,
            shortcuts_enabled: true,
            configured_providers: vec!["openai".into(), "anthropic".into()],
        }
    }

    #[test]
    fn report_includes_the_key_metadata() {
        let report = sample().to_report();
        assert!(report.contains("version: 0.1.0"));
        assert!(report.contains("os: macos (aarch64)"));
        assert!(report.contains("default model: gpt-4o"));
        assert!(report.contains("providers configured: openai, anthropic"));
    }

    #[test]
    fn report_renders_optional_fields_as_none() {
        let mut d = sample();
        d.default_model = None;
        d.secondary_language = None;
        d.configured_providers = vec![];
        let report = d.to_report();
        assert!(report.contains("default model: none"));
        assert!(report.contains("secondary language: none"));
        assert!(report.contains("providers configured: none"));
    }

    #[test]
    fn diagnostics_struct_has_no_secret_or_text_fields() {
        // A structural guard for FR-052/FR-067: this report can only ever carry
        // the fields below, none of which hold a key, prompt, or translation.
        // If a future field is added, update this list deliberately.
        let report = sample().to_report();
        for line in report.lines().skip(1) {
            let label = line.split(':').next().unwrap_or("");
            assert!(
                matches!(
                    label,
                    "version"
                        | "os"
                        | "default provider"
                        | "default model"
                        | "primary language"
                        | "secondary language"
                        | "additional languages"
                        | "global shortcuts enabled"
                        | "providers configured"
                ),
                "unexpected diagnostics field: {label}"
            );
        }
    }
}
