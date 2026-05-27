//! OpenAI adapter — direct BYOK calls to the Chat Completions API (P0-008).

use super::http;
use super::{Prompt, Provider, ProviderId};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use serde::Deserialize;

const BASE: &str = "https://api.openai.com/v1";
const NAME: &str = "OpenAI";

pub struct OpenAi;

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
impl Provider for OpenAi {
    fn id(&self) -> ProviderId {
        ProviderId::Openai
    }
    fn display_name(&self) -> &'static str {
        NAME
    }

    async fn validate_key(&self, api_key: &str) -> Result<bool> {
        let req = http::client()
            .get(format!("{BASE}/models"))
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
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"Hola"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.into_text().unwrap(), "Hola");
    }

    #[test]
    fn empty_choices_is_an_error() {
        let resp: ChatResponse = serde_json::from_str(r#"{"choices":[]}"#).unwrap();
        assert!(resp.into_text().is_err());
    }

    #[test]
    fn parses_and_sorts_model_ids() {
        let json = r#"{"data":[{"id":"gpt-4o"},{"id":"gpt-3.5-turbo"}]}"#;
        let resp: ModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(model_ids(resp), vec!["gpt-3.5-turbo", "gpt-4o"]);
    }
}
