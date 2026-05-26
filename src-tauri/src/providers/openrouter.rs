//! OpenRouter adapter (one key, many model families). HTTP wiring lands in P0-008.

use super::{Provider, ProviderId, TranslationRequest, TranslationResponse};
use crate::error::{AppError, Result};
use async_trait::async_trait;

pub struct OpenRouter;

fn pending() -> AppError {
    AppError::Provider("OpenRouter adapter not implemented yet (P0-008).".into())
}

#[async_trait]
impl Provider for OpenRouter {
    fn id(&self) -> ProviderId {
        ProviderId::Openrouter
    }
    fn display_name(&self) -> &'static str {
        "OpenRouter"
    }
    async fn validate_key(&self, _api_key: &str) -> Result<bool> {
        Err(pending())
    }
    async fn list_models(&self, _api_key: &str) -> Result<Vec<String>> {
        Err(pending())
    }
    async fn translate(
        &self,
        _request: &TranslationRequest,
        _api_key: &str,
    ) -> Result<TranslationResponse> {
        Err(pending())
    }
}
