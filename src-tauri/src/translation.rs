//! Translation orchestrator and prompt templates (P0-010, PRD §15.6).
//!
//! [`plan`] is the orchestrator's pure core: it turns a request (source text +
//! routing mode + language settings) into a [`TranslationPlan`] — the resolved
//! target language plus the provider-neutral [`Prompt`] to send. The async I/O
//! (Keychain lookup, the provider HTTP call, response assembly) lives in
//! `commands::translate`, which calls `plan` then hands the prompt to an adapter.
//!
//! The prompt builders embed the source text in the user message, not the
//! instructions, and instruct the model to return only the translation while
//! preserving formatting/code (FR-014/FR-015). No translation text is persisted
//! anywhere, and the only network calls are direct BYOK provider requests —
//! TLiquid has no server of its own (FR-019/FR-020).

use crate::config::Settings;
use crate::error::{AppError, Result};
use crate::languages::{self, Resolution};
use crate::providers::{Language, Prompt, RoutingMode};

/// The resolved, ready-to-send result of orchestrating a translation request.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslationPlan {
    /// Best-effort target to label the result with (see [`Resolution::display_target`]).
    pub target_language: Language,
    /// The system+user prompt to send to the chosen provider.
    pub prompt: Prompt,
}

/// Orchestrate a request into a [`TranslationPlan`]: resolve routing against the
/// user's language settings and build the matching prompt. Pure (no I/O), so the
/// routing→prompt behavior is unit-tested. Errors only when secondary mode is
/// requested without a secondary language configured.
pub fn plan(
    settings: &Settings,
    mode: RoutingMode,
    explicit: Option<Language>,
    source_text: &str,
) -> Result<TranslationPlan> {
    let resolution = languages::resolve(settings, mode, explicit);
    let target_language = resolution
        .display_target()
        .ok_or_else(|| AppError::Provider("No secondary language is configured.".into()))?;
    let prompt = build_prompt(&resolution, source_text);
    Ok(TranslationPlan {
        target_language,
        prompt,
    })
}

/// Build the system/user prompt for a resolved routing decision. Internal to
/// the orchestrator — callers use [`plan`], which guarantees a real target.
pub(crate) fn build_prompt(resolution: &Resolution, text: &str) -> Prompt {
    match resolution {
        Resolution::Fixed(target) => build_explicit_prompt(target, text),
        Resolution::PrimaryRouted {
            primary,
            secondary,
            fallback,
        } => build_primary_prompt(primary, secondary.as_ref(), fallback, text),
        Resolution::MissingSecondary => Prompt {
            system: String::new(),
            user: text.to_string(),
        },
    }
}

pub(crate) fn build_primary_prompt(
    primary: &Language,
    secondary: Option<&Language>,
    fallback: &Language,
    text: &str,
) -> Prompt {
    let secondary_name = secondary.map(|l| l.name.as_str()).unwrap_or("none");
    let system = format!(
        "You are a translation engine.\n\n\
         Primary language: {primary}\n\
         Secondary language: {secondary}\n\n\
         Detect the source language of the text.\n\n\
         Rules:\n\
         1. If the source language is not the primary language, translate the text into the primary language.\n\
         2. If the source language is the primary language and a secondary language is configured, translate the text into the secondary language.\n\
         3. If the source language is the primary language and no secondary language is configured, translate into {fallback}.\n\
         4. Preserve meaning, tone, formatting, punctuation, markdown, code blocks, and technical terminology.\n\
         5. Return only the translation. Do not explain.",
        primary = primary.name,
        secondary = secondary_name,
        fallback = fallback.name,
    );
    Prompt {
        system,
        user: text.to_string(),
    }
}

pub(crate) fn build_explicit_prompt(target: &Language, text: &str) -> Prompt {
    let system = format!(
        "You are a translation engine.\n\n\
         Detect the source language automatically.\n\
         Translate the text into {target}.\n\
         Preserve meaning, tone, formatting, punctuation, markdown, code blocks, and technical terminology.\n\
         Return only the translation. Do not explain.",
        target = target.name,
    );
    Prompt {
        system,
        user: text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lang(code: &str, name: &str) -> Language {
        Language {
            code: code.into(),
            name: name.into(),
        }
    }

    #[test]
    fn explicit_prompt_names_target_and_carries_text_in_user_message() {
        let prompt = build_explicit_prompt(&lang("fr", "French"), "Hello");
        assert!(prompt.system.contains("Translate the text into French."));
        assert!(prompt.system.contains("Return only the translation."));
        // The source text is the user content, not interpolated into the system.
        assert_eq!(prompt.user, "Hello");
        assert!(!prompt.system.contains("Hello"));
    }

    #[test]
    fn primary_prompt_reports_no_secondary() {
        let prompt =
            build_primary_prompt(&lang("en", "English"), None, &lang("en", "English"), "Hola");
        assert!(prompt.system.contains("Secondary language: none"));
        assert_eq!(prompt.user, "Hola");
    }

    #[test]
    fn primary_prompt_names_secondary_when_configured() {
        let prompt = build_primary_prompt(
            &lang("en", "English"),
            Some(&lang("es", "Spanish")),
            &lang("es", "Spanish"),
            "Hello",
        );
        assert!(prompt.system.contains("Secondary language: Spanish"));
    }

    // ── orchestrator (plan) ────────────────────────────────────────────────

    #[test]
    fn plan_explicit_mode_targets_the_explicit_language() {
        let settings = Settings::default();
        let plan = plan(
            &settings,
            RoutingMode::Explicit,
            Some(lang("de", "German")),
            "Hello",
        )
        .unwrap();
        assert_eq!(plan.target_language, lang("de", "German"));
        assert!(plan
            .prompt
            .system
            .contains("Translate the text into German."));
        assert_eq!(plan.prompt.user, "Hello");
    }

    #[test]
    fn plan_primary_mode_without_secondary_targets_primary_and_routes() {
        let settings = Settings::default(); // primary = English, no secondary
        let plan = plan(&settings, RoutingMode::Primary, None, "Hola").unwrap();
        assert_eq!(plan.target_language.code, "en");
        // Primary-mode prompt encodes the routing rules rather than one target.
        assert!(plan.prompt.system.contains("Detect the source language"));
        assert!(plan.prompt.system.contains("Secondary language: none"));
        assert_eq!(plan.prompt.user, "Hola");
    }

    #[test]
    fn plan_primary_mode_names_secondary_when_configured() {
        let mut settings = Settings::default();
        settings.languages.secondary = Some(lang("es", "Spanish"));
        let plan = plan(&settings, RoutingMode::Primary, None, "Hello").unwrap();
        assert!(plan.prompt.system.contains("Secondary language: Spanish"));
    }

    #[test]
    fn plan_secondary_mode_requires_a_secondary_language() {
        let settings = Settings::default(); // no secondary
        assert!(plan(&settings, RoutingMode::Secondary, None, "Hi").is_err());
    }

    #[test]
    fn plan_secondary_mode_targets_the_secondary_language() {
        let mut settings = Settings::default();
        settings.languages.secondary = Some(lang("fr", "French"));
        let plan = plan(&settings, RoutingMode::Secondary, None, "Hello").unwrap();
        assert_eq!(plan.target_language, lang("fr", "French"));
        assert!(plan
            .prompt
            .system
            .contains("Translate the text into French."));
    }
}
