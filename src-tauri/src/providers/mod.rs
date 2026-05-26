//! Provider abstraction layer (P0-008, PRD §15.5).
//!
//! Each provider is an interchangeable translation backend. Adapters make
//! direct BYOK HTTP calls (Phase 0) and never leak provider-specific shapes
//! into the UI beyond the metadata in [`ProviderMeta`].

mod anthropic;
mod gemini;
mod ollama;
mod openai;
mod openrouter;

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Openai,
    Anthropic,
    Gemini,
    Openrouter,
    Ollama,
}

impl ProviderId {
    /// Stable string id used for Keychain entries and config keys.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Openai => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Gemini => "gemini",
            ProviderId::Openrouter => "openrouter",
            ProviderId::Ollama => "ollama",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Language {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingMode {
    Primary,
    Secondary,
    Explicit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRequest {
    pub source_text: String,
    pub routing_mode: RoutingMode,
    pub explicit_target_language: Option<Language>,
    pub provider: ProviderId,
    pub model: String,
    #[serde(default)]
    pub preserve_formatting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResponse {
    pub translated_text: String,
    pub detected_source_language: Option<String>,
    pub target_language: Language,
    pub provider: ProviderId,
    pub model: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMeta {
    pub id: ProviderId,
    pub display_name: String,
    /// Selectable in Phase 0. Ollama/local models are Phase 1 (FR-039).
    pub available: bool,
    pub supports_streaming: bool,
}

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn display_name(&self) -> &'static str;
    fn available(&self) -> bool {
        true
    }
    fn supports_streaming(&self) -> bool {
        false
    }
    async fn validate_key(&self, api_key: &str) -> Result<bool>;
    async fn list_models(&self, api_key: &str) -> Result<Vec<String>>;
    async fn translate(
        &self,
        request: &TranslationRequest,
        api_key: &str,
    ) -> Result<TranslationResponse>;
}

/// Construct the adapter for a provider id.
pub fn adapter(id: ProviderId) -> Box<dyn Provider> {
    match id {
        ProviderId::Openai => Box::new(openai::OpenAi),
        ProviderId::Anthropic => Box::new(anthropic::Anthropic),
        ProviderId::Gemini => Box::new(gemini::Gemini),
        ProviderId::Openrouter => Box::new(openrouter::OpenRouter),
        ProviderId::Ollama => Box::new(ollama::Ollama),
    }
}

/// Metadata for every known provider, for the settings UI.
pub fn all() -> Vec<ProviderMeta> {
    [
        ProviderId::Openai,
        ProviderId::Anthropic,
        ProviderId::Gemini,
        ProviderId::Openrouter,
        ProviderId::Ollama,
    ]
    .into_iter()
    .map(|id| {
        let a = adapter(id);
        ProviderMeta {
            id: a.id(),
            display_name: a.display_name().to_string(),
            available: a.available(),
            supports_streaming: a.supports_streaming(),
        }
    })
    .collect()
}
