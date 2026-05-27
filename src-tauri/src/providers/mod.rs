//! Provider abstraction layer (P0-008, PRD §15.5).
//!
//! Each provider is an interchangeable translation backend. Adapters make
//! direct BYOK HTTP calls (Phase 0) and never leak provider-specific shapes
//! into the UI beyond the metadata in [`ProviderMeta`].

mod anthropic;
mod gemini;
mod http;
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

/// A provider-neutral prompt: a system instruction and the user content (the
/// source text). Built by [`crate::translation`] from the routing decision and
/// consumed by adapters, which map it onto each provider's chat/messages shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub system: String,
    pub user: String,
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
    /// Check whether `api_key` is accepted by the provider. `Ok(true)` valid,
    /// `Ok(false)` rejected (401/403), `Err` for network/other failures.
    async fn validate_key(&self, api_key: &str) -> Result<bool>;
    /// List the model ids the key can use, for the model picker (FR-041).
    async fn list_models(&self, api_key: &str) -> Result<Vec<String>>;
    /// Run one non-streaming translation and return the model's completion
    /// text. The orchestrator ([`crate::translation`]) builds `prompt` and
    /// wraps the result in a [`TranslationResponse`]; adapters stay response-
    /// agnostic so error/latency assembly lives in one place.
    async fn translate(&self, api_key: &str, model: &str, prompt: &Prompt) -> Result<String>;
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

#[cfg(test)]
mod privacy_tests {
    //! Privacy invariant (P0-017; FR-019/FR-020/FR-064/FR-067): TLiquid has no
    //! server of its own. The only network calls are direct BYOK requests to the
    //! user's chosen provider. Two guards here:
    //!  1. `http_client_is_confined_to_the_provider_layer` — the `reqwest` client
    //!     appears only under `src/providers/`, so this module IS the whole HTTP
    //!     surface (nothing in `commands`/`lib`/etc. can make a network call).
    //!  2. `provider_layer_only_contacts_allowed_hosts` — within that surface,
    //!     every host literal is a known provider or local endpoint, never a
    //!     TLiquid/telemetry URL.
    //!
    //! Privacy checklist for changes anywhere in the app:
    //! - Adding a provider? add its API host to `ALLOWED_HOSTS` below.
    //! - Never send translation text, prompts, provider responses, clipboard
    //!   contents, or API keys to a TLiquid/analytics endpoint — there is none.
    //! - No telemetry, analytics, or automatic update-check network calls (and no
    //!   such dependencies in Cargo.toml).
    //! - Keep keys out of logs/errors (guarded by `secrets`/`error`) and out of
    //!   the diagnostics export (guarded by `diagnostics`).
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Collect every `.rs` file under `dir`, recursively.
    fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rs_files(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn http_client_is_confined_to_the_provider_layer() {
        // If `reqwest` is referenced outside `src/providers/`, the audited HTTP
        // surface has leaked — fail so a stray network call is caught.
        let src = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        let mut files = Vec::new();
        rs_files(src, &mut files);

        let mut provider_uses_reqwest = false;
        for path in &files {
            let in_providers = path.components().any(|c| c.as_os_str() == "providers");
            let body = fs::read_to_string(path).unwrap();
            if in_providers {
                provider_uses_reqwest |= body.contains("reqwest");
            } else {
                assert!(
                    !body.contains("reqwest"),
                    "HTTP must stay in the provider layer; found `reqwest` in {path:?}"
                );
            }
        }
        assert!(
            provider_uses_reqwest,
            "expected the provider layer to use reqwest; scan may be broken"
        );
    }

    /// Hosts the app is allowed to contact. Provider REST APIs + local (Ollama).
    const ALLOWED_HOSTS: &[&str] = &[
        "api.openai.com",
        "api.anthropic.com",
        "generativelanguage.googleapis.com",
        "openrouter.ai",
        "localhost",
        "127.0.0.1",
    ];

    /// Extract the host of every `http(s)://…` URL literal in `src`.
    fn hosts_in(src: &str) -> Vec<String> {
        let mut hosts = Vec::new();
        for scheme in ["https://", "http://"] {
            let mut rest = src;
            while let Some(i) = rest.find(scheme) {
                let after = &rest[i + scheme.len()..];
                let end = after
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
                    .unwrap_or(after.len());
                if end > 0 {
                    hosts.push(after[..end].to_string());
                }
                rest = &after[end..];
            }
        }
        hosts
    }

    #[test]
    fn provider_layer_only_contacts_allowed_hosts() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/providers");
        let mut found = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                found.extend(hosts_in(&fs::read_to_string(&path).unwrap()));
            }
        }
        // Guard against the scan silently finding nothing (e.g. if it breaks).
        assert!(
            found.iter().any(|h| h == "api.openai.com"),
            "expected to find provider URLs to validate; scan may be broken"
        );
        for host in &found {
            assert!(
                !host.contains("tliquid"),
                "the provider layer must never contact a TLiquid host: {host}"
            );
            assert!(
                ALLOWED_HOSTS.contains(&host.as_str()),
                "unexpected host in the provider layer: {host} (add it to ALLOWED_HOSTS only if it is a real provider/local endpoint)"
            );
        }
    }
}
