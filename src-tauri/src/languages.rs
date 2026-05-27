//! Language routing engine (P0-006, PRD §15.4).
//!
//! Source-language detection happens inside the LLM call, so primary-mode
//! routing is encoded into the prompt rather than resolved to a single target
//! up front.

use crate::config::Settings;
use crate::providers::{Language, RoutingMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A single fixed target language (explicit or secondary mode).
    Fixed(Language),
    /// Primary mode: the model detects the source and applies the §9.2 rules.
    /// `fallback` is used when the source is the primary language and no
    /// secondary is configured.
    PrimaryRouted {
        primary: Language,
        secondary: Option<Language>,
        fallback: Language,
    },
    /// Secondary mode was requested but no secondary language is configured.
    MissingSecondary,
}

impl Resolution {
    /// Best-effort target to show in the result. In primary mode the real
    /// target depends on the source the model detects, so this returns the
    /// primary as a representative label. `None` only for [`Self::MissingSecondary`].
    pub fn display_target(&self) -> Option<Language> {
        match self {
            Resolution::Fixed(lang) => Some(lang.clone()),
            Resolution::PrimaryRouted { primary, .. } => Some(primary.clone()),
            Resolution::MissingSecondary => None,
        }
    }
}

pub fn resolve(settings: &Settings, mode: RoutingMode, explicit: Option<Language>) -> Resolution {
    match mode {
        RoutingMode::Explicit => {
            Resolution::Fixed(explicit.unwrap_or_else(|| settings.languages.primary.clone()))
        }
        RoutingMode::Secondary => match settings.languages.secondary.clone() {
            Some(secondary) => Resolution::Fixed(secondary),
            None => Resolution::MissingSecondary,
        },
        RoutingMode::Primary => {
            let primary = settings.languages.primary.clone();
            let secondary = settings.languages.secondary.clone();
            let fallback = secondary.clone().unwrap_or_else(|| primary.clone());
            Resolution::PrimaryRouted {
                primary,
                secondary,
                fallback,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    fn lang(code: &str, name: &str) -> Language {
        Language {
            code: code.into(),
            name: name.into(),
        }
    }

    #[test]
    fn explicit_mode_uses_explicit_target() {
        let settings = Settings::default();
        let r = resolve(&settings, RoutingMode::Explicit, Some(lang("de", "German")));
        assert_eq!(r, Resolution::Fixed(lang("de", "German")));
    }

    #[test]
    fn secondary_mode_requires_a_secondary_language() {
        let settings = Settings::default(); // no secondary by default
        assert_eq!(
            resolve(&settings, RoutingMode::Secondary, None),
            Resolution::MissingSecondary
        );
    }

    #[test]
    fn secondary_mode_targets_secondary_when_set() {
        let mut settings = Settings::default();
        settings.languages.secondary = Some(lang("es", "Spanish"));
        assert_eq!(
            resolve(&settings, RoutingMode::Secondary, None),
            Resolution::Fixed(lang("es", "Spanish"))
        );
    }

    #[test]
    fn primary_mode_falls_back_to_primary_without_secondary() {
        let settings = Settings::default(); // primary = English, no secondary
        match resolve(&settings, RoutingMode::Primary, None) {
            Resolution::PrimaryRouted {
                primary,
                secondary,
                fallback,
            } => {
                assert_eq!(primary.code, "en");
                assert!(secondary.is_none());
                assert_eq!(fallback.code, "en");
            }
            other => panic!("unexpected resolution: {other:?}"),
        }
    }
}
