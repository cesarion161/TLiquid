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

/// Pull the incremental text from one streaming chunk: `choices[0].delta.content`.
/// `None` for role-only/empty/`finish_reason` chunks. Pure, so it is unit-tested.
fn stream_delta(data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    let text = v
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()?;
    (!text.is_empty()).then(|| text.to_string())
}

#[async_trait]
impl Provider for OpenAi {
    fn id(&self) -> ProviderId {
        ProviderId::Openai
    }
    fn display_name(&self) -> &'static str {
        NAME
    }
    fn supports_streaming(&self) -> bool {
        true
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

    async fn translate_stream(
        &self,
        api_key: &str,
        model: &str,
        prompt: &Prompt,
        on_delta: &(dyn Fn(String) + Send + Sync),
    ) -> Result<String> {
        let body = serde_json::json!({
            "model": model,
            "stream": true,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user },
            ],
        });
        let req = http::client()
            .post(format!("{BASE}/chat/completions"))
            .bearer_auth(api_key)
            .json(&body);
        let mut acc = String::new();
        http::stream_sse(NAME, req, |data| {
            if let Some(delta) = stream_delta(data) {
                acc.push_str(&delta);
                on_delta(delta);
            }
            Ok(())
        })
        .await?;
        if acc.is_empty() {
            return Err(AppError::Provider(format!(
                "{NAME}: the model returned no text."
            )));
        }
        Ok(acc)
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

    #[test]
    fn stream_delta_extracts_content_chunks() {
        let chunk = r#"{"choices":[{"delta":{"content":"Ho"}}]}"#;
        assert_eq!(stream_delta(chunk).as_deref(), Some("Ho"));
    }

    #[test]
    fn stream_delta_ignores_role_only_and_final_chunks() {
        // First chunk carries only the role; last carries finish_reason, no content.
        assert_eq!(
            stream_delta(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        );
        assert_eq!(
            stream_delta(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#),
            None
        );
    }
}
