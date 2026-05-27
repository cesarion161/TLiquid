//! Translation orchestrator and prompt templates (P0-010, PRD §15.6).
//!
//! The prompt builders are pure and provider-neutral so adapters can reuse them.
//! They produce a [`Prompt`] (system instruction + user content); the source
//! text goes in the user message rather than being interpolated into the
//! instructions. No translation text is persisted anywhere (PRD FR-019).

use crate::languages::Resolution;
use crate::providers::{Language, Prompt};

/// Build the system/user prompt for a resolved routing decision.
pub fn build_prompt(resolution: &Resolution, text: &str) -> Prompt {
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

pub fn build_primary_prompt(
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

pub fn build_explicit_prompt(target: &Language, text: &str) -> Prompt {
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
}
