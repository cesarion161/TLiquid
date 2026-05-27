//! Anthropic adapter — direct BYOK calls to the Messages API (P0-008).

use super::http;
use super::{Prompt, Provider, ProviderId};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use serde::Deserialize;

const BASE: &str = "https://api.anthropic.com/v1";
const VERSION: &str = "2023-06-01";
const NAME: &str = "Anthropic";
/// Output cap for one translation. 4096 is supported by every current model;
/// long inputs may truncate (acceptable for Phase 0).
const MAX_TOKENS: u32 = 4096;

pub struct Anthropic;

#[derive(Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

impl MessagesResponse {
    fn into_text(self) -> Result<String> {
        // A message is a list of blocks; concatenate the text ones.
        let text: String = self
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text)
            .collect();
        if text.is_empty() {
            Err(AppError::Provider(format!(
                "{NAME}: the model returned no text."
            )))
        } else {
            Ok(text)
        }
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
    resp.data.into_iter().map(|m| m.id).collect()
}

/// Pull the incremental text from one Anthropic stream event: only
/// `content_block_delta` events with a `text_delta` carry output text. Other
/// event types (`message_start`, `ping`, `message_stop`, …) yield `None`. Pure,
/// so it is unit-tested.
fn stream_delta(data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    if v.get("type")?.as_str()? != "content_block_delta" {
        return None;
    }
    let delta = v.get("delta")?;
    if delta.get("type")?.as_str()? != "text_delta" {
        return None;
    }
    let text = delta.get("text")?.as_str()?;
    (!text.is_empty()).then(|| text.to_string())
}

impl Anthropic {
    fn models_request(api_key: &str) -> reqwest::RequestBuilder {
        http::client()
            .get(format!("{BASE}/models"))
            .header("x-api-key", api_key)
            .header("anthropic-version", VERSION)
    }
}

#[async_trait]
impl Provider for Anthropic {
    fn id(&self) -> ProviderId {
        ProviderId::Anthropic
    }
    fn display_name(&self) -> &'static str {
        NAME
    }
    fn supports_streaming(&self) -> bool {
        true
    }

    async fn validate_key(&self, api_key: &str) -> Result<bool> {
        http::validate_status(NAME, Self::models_request(api_key)).await
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<String>> {
        let resp: ModelsResponse = http::send_json(NAME, Self::models_request(api_key)).await?;
        Ok(model_ids(resp))
    }

    async fn translate(&self, api_key: &str, model: &str, prompt: &Prompt) -> Result<String> {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": MAX_TOKENS,
            "system": prompt.system,
            "messages": [ { "role": "user", "content": prompt.user } ],
        });
        let req = http::client()
            .post(format!("{BASE}/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", VERSION)
            .json(&body);
        let resp: MessagesResponse = http::send_json(NAME, req).await?;
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
            "max_tokens": MAX_TOKENS,
            "stream": true,
            "system": prompt.system,
            "messages": [ { "role": "user", "content": prompt.user } ],
        });
        let req = http::client()
            .post(format!("{BASE}/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", VERSION)
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
    fn extracts_and_concatenates_text_blocks() {
        let json = r#"{"content":[{"type":"text","text":"Bon"},{"type":"text","text":"jour"}]}"#;
        let resp: MessagesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.into_text().unwrap(), "Bonjour");
    }

    #[test]
    fn ignores_non_text_blocks() {
        let json = r#"{"content":[{"type":"thinking","text":"hmm"},{"type":"text","text":"Hi"}]}"#;
        let resp: MessagesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.into_text().unwrap(), "Hi");
    }

    #[test]
    fn no_text_blocks_is_an_error() {
        let resp: MessagesResponse = serde_json::from_str(r#"{"content":[]}"#).unwrap();
        assert!(resp.into_text().is_err());
    }

    #[test]
    fn parses_model_ids() {
        let json = r#"{"data":[{"id":"claude-opus-4-6"},{"id":"claude-haiku-4-5"}]}"#;
        let resp: ModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(model_ids(resp), vec!["claude-opus-4-6", "claude-haiku-4-5"]);
    }

    #[test]
    fn stream_delta_extracts_text_deltas_only() {
        let chunk = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Bon"}}"#;
        assert_eq!(stream_delta(chunk).as_deref(), Some("Bon"));
    }

    #[test]
    fn stream_delta_ignores_other_event_types() {
        assert_eq!(
            stream_delta(r#"{"type":"message_start","message":{}}"#),
            None
        );
        assert_eq!(stream_delta(r#"{"type":"message_stop"}"#), None);
        // A non-text delta (e.g. input_json_delta) is not output text.
        assert_eq!(
            stream_delta(
                r#"{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{"}}"#
            ),
            None
        );
    }
}
