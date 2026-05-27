//! OpenRouter adapter — one key, many model families via an OpenAI-compatible
//! Chat Completions API (P0-008). Key validation uses the dedicated key endpoint
//! (`/key`) since the models list is public and wouldn't reject a bad key.

use super::http;
use super::{Prompt, Provider, ProviderId};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use serde::Deserialize;

const BASE: &str = "https://openrouter.ai/api/v1";
const NAME: &str = "OpenRouter";

pub struct OpenRouter;

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: String,
}

impl ChatResponse {
    fn into_text(self) -> Result<String> {
        self.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| AppError::Provider(format!("{NAME}: the model returned no text.")))
    }
}

#[derive(Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

fn model_ids(resp: ModelsResponse) -> Vec<String> {
    let mut ids: Vec<String> = resp.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    ids
}

#[async_trait]
impl Provider for OpenRouter {
    fn id(&self) -> ProviderId {
        ProviderId::Openrouter
    }
    fn display_name(&self) -> &'static str {
        NAME
    }

    async fn validate_key(&self, api_key: &str) -> Result<bool> {
        let req = http::client()
            .get(format!("{BASE}/key"))
            .bearer_auth(api_key);
        http::validate_status(NAME, req).await
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<String>> {
        let req = http::client()
            .get(format!("{BASE}/models"))
            .bearer_auth(api_key);
        let resp: ModelsResponse = http::send_json(NAME, req).await?;
        Ok(model_ids(resp))
    }

    async fn translate(&self, api_key: &str, model: &str, prompt: &Prompt) -> Result<String> {
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user },
            ],
        });
        let req = http::client()
            .post(format!("{BASE}/chat/completions"))
            .bearer_auth(api_key)
            // Optional attribution headers OpenRouter uses for ranking.
            .header("X-Title", "TLiquid")
            .json(&body);
        let resp: ChatResponse = http::send_json(NAME, req).await?;
        resp.into_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_completion_text() {
        let json = r#"{"choices":[{"message":{"content":"Ciao"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.into_text().unwrap(), "Ciao");
    }

    #[test]
    fn parses_and_sorts_model_ids() {
        let json = r#"{"data":[{"id":"openai/gpt-4o"},{"id":"anthropic/claude-3.5"}]}"#;
        let resp: ModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            model_ids(resp),
            vec!["anthropic/claude-3.5", "openai/gpt-4o"]
        );
    }
}
