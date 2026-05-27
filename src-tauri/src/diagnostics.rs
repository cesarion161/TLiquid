//! Local diagnostics export (P0-016 + P1-007, FR-064/FR-065/FR-067, PRD §10.6.6).
//!
//! Produces a plain-text bundle the user can copy or save for a bug report. It
//! is **never uploaded** — there is no TLiquid server. The metadata section
//! contains only non-sensitive settings *shape* (the [`Diagnostics`] struct has
//! no field for any key/text). The bundle also includes a tail of the app log
//! and a recent-error summary (P1-007); this is safe because TLiquid's logging
//! discipline (P0-017 audit) never writes API keys, translation text, clipboard
//! contents, prompts, or provider responses to the log — so the log carries only
//! categories/levels, never the forbidden content (FR-067).

use crate::config;
use crate::providers;
use crate::secrets;
use crate::startup;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// How many trailing log lines the bundle includes.
const LOG_TAIL_LINES: usize = 80;

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
    pub launch_at_login: bool,
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
        launch_at_login: startup::is_enabled(app),
        configured_providers,
    }
}

/// Absolute path to the persisted log file (P1-007), matching the `LogDir`
/// target configured in `lib.rs` (`tliquid.log` in the app log dir).
pub fn log_file_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_log_dir()
        .ok()
        .map(|dir| dir.join("tliquid.log"))
}

/// The last [`LOG_TAIL_LINES`] lines of the log file (P1-007), or an empty vec
/// if there is no log yet. Reading the whole file is fine — the log plugin
/// rotates it, so it stays bounded.
fn recent_log_lines(app: &AppHandle) -> Vec<String> {
    let Some(path) = log_file_path(app) else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(LOG_TAIL_LINES);
    lines[start..].iter().map(|s| s.to_string()).collect()
}

/// Count error/warn lines in a log tail by their level token (P1-007 "recent
/// error categories"). Pure, so it is unit-tested. The level appears as a
/// bracketed/standalone token in the log plugin's format, e.g. `[ERROR]`.
fn count_levels(lines: &[String]) -> (usize, usize) {
    let mut errors = 0;
    let mut warns = 0;
    for line in lines {
        let upper = line.to_uppercase();
        if upper.contains("ERROR") {
            errors += 1;
        } else if upper.contains("WARN") {
            warns += 1;
        }
    }
    (errors, warns)
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
             launch at login: {launch}\n\
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
            launch = self.launch_at_login,
            configured = configured,
        )
    }
}

/// The full diagnostics bundle (P1-007): the non-secret metadata report, a
/// recent-error summary, and the tail of the app log — for a bug report. Local
/// only; never uploaded (FR-064). Contains no keys/text by construction +
/// logging discipline (see the module docs).
pub fn bundle(app: &AppHandle) -> String {
    let report = collect(app).to_report();
    let log = recent_log_lines(app);
    let (errors, warns) = count_levels(&log);

    let mut out = report;
    out.push_str(&format!(
        "\n\nrecent log: {} lines ({errors} error, {warns} warn)",
        log.len()
    ));
    out.push_str("\n\n--- recent log (tail) ---\n");
    if log.is_empty() {
        out.push_str("(no log file yet)");
    } else {
        out.push_str(&log.join("\n"));
    }
    out
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
            launch_at_login: false,
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
                        | "launch at login"
                        | "providers configured"
                ),
                "unexpected diagnostics field: {label}"
            );
        }
    }

    #[test]
    fn count_levels_tallies_errors_and_warns() {
        let lines = vec![
            "[2026-01-01][tliquid][INFO] panel ready".to_string(),
            "[2026-01-01][tliquid][WARN] shortcut not registered".to_string(),
            "[2026-01-01][tliquid][ERROR] provider error: rate limited".to_string(),
            "[2026-01-01][tliquid][ERROR] capture error".to_string(),
        ];
        assert_eq!(count_levels(&lines), (2, 1));
        assert_eq!(count_levels(&[]), (0, 0));
    }
}
