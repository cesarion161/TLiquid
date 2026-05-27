//! Ollama / local model adapter. Phase 1 (FR-039); not selectable in Phase 0.

use super::{Prompt, Provider, ProviderId};
use crate::error::{AppError, Result};
use async_trait::async_trait;

pub struct Ollama;

fn pending() -> AppError {
    AppError::Provider("Ollama/local models are a Phase 1 feature (P1-004).".into())
}

#[async_trait]
impl Provider for Ollama {
    fn id(&self) -> ProviderId {
        ProviderId::Ollama
    }
    fn display_name(&self) -> &'static str {
        "Ollama (local)"
    }
    fn available(&self) -> bool {
        false
    }
    async fn validate_key(&self, _api_key: &str) -> Result<bool> {
        Err(pending())
    }
    async fn list_models(&self, _api_key: &str) -> Result<Vec<String>> {
        Err(pending())
    }
    async fn translate(&self, _api_key: &str, _model: &str, _prompt: &Prompt) -> Result<String> {
        Err(pending())
    }
}
