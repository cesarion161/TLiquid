//! Ollama / local model adapter (P1-004, FR-039).
//!
//! Ollama is local and **keyless**: it is addressed by an endpoint URL instead
//! of an API key. To keep the [`Provider`] trait unchanged, the orchestrator
//! passes the configured endpoint (`config::Providers::ollama_endpoint`) through
//! the same `api_key` parameter the cloud adapters use for their key — here it
//! is the base URL (e.g. `http://localhost:11434`). No literal host appears in
//! this file; the URL is always supplied at runtime from settings.

use super::http;
use super::{Prompt, Provider, ProviderId};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use serde::Deserialize;

const NAME: &str = "Ollama";

pub struct Ollama;

/// Trim a trailing slash so `{base}/api/...` never doubles up.
fn base(endpoint: &str) -> &str {
    endpoint.trim_end_matches('/')
}

// ── /api/tags (model list) ──────────────────────────────────────────────────
#[derive(Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<TagEntry>,
}

#[derive(Deserialize)]
struct TagEntry {
    name: String,
}

fn model_ids(resp: TagsResponse) -> Vec<String> {
    let mut ids: Vec<String> = resp.models.into_iter().map(|m| m.name).collect();
    ids.sort();
    ids
}

// ── /api/chat (translation) ─────────────────────────────────────────────────
#[derive(Deserialize)]
struct ChatResponse {
    message: Option<ChatMessage>,
    /// Ollama can report a failure as a 200 body `{"error":"..."}` (e.g. a
    /// runtime/model error), not an HTTP error status — surface it (FR-018).
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: String,
}

impl ChatResponse {
    fn into_text(self) -> Result<String> {
        if let Some(err) = self.error {
            return Err(AppError::Provider(format!("{NAME}: {err}")));
        }
        self.message
            .map(|m| m.content)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| AppError::Provider(format!("{NAME}: the model returned no text.")))
    }
}

/// Pull the incremental text from one NDJSON chat chunk: `message.content`.
/// Each line is a full JSON object; the final one has `done: true`. Pure, so it
/// is unit-tested.
fn stream_delta(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let text = v.get("message")?.get("content")?.as_str()?;
    (!text.is_empty()).then(|| text.to_string())
}

/// A top-level `{"error":"..."}` Ollama may emit as a 200 NDJSON line mid-stream
/// (the local server is keyless, so the message is non-secret). Pure/tested.
fn stream_error(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    v.get("error").and_then(|e| e.as_str()).map(str::to_string)
}

fn chat_body(model: &str, prompt: &Prompt, stream: bool) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "stream": stream,
        "messages": [
            { "role": "system", "content": prompt.system },
            { "role": "user", "content": prompt.user },
        ],
    })
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
        true
    }
    fn supports_streaming(&self) -> bool {
        true
    }

    /// "Validate" = the local server is reachable and responds to `/api/tags`.
    /// There is no key to reject, so a reachable server is `Ok(true)` and an
    /// unreachable one is an `Err` (a clear network message), never `Ok(false)`.
    async fn validate_key(&self, endpoint: &str) -> Result<bool> {
        let req = http::client().get(format!("{}/api/tags", base(endpoint)));
        http::validate_status(NAME, req).await
    }

    async fn list_models(&self, endpoint: &str) -> Result<Vec<String>> {
        let req = http::client().get(format!("{}/api/tags", base(endpoint)));
        let resp: TagsResponse = http::send_json(NAME, req).await?;
        Ok(model_ids(resp))
    }

    async fn translate(&self, endpoint: &str, model: &str, prompt: &Prompt) -> Result<String> {
        let req = http::client()
            .post(format!("{}/api/chat", base(endpoint)))
            .json(&chat_body(model, prompt, false));
        let resp: ChatResponse = http::send_json(NAME, req).await?;
        resp.into_text()
    }

    async fn translate_stream(
        &self,
        endpoint: &str,
        model: &str,
        prompt: &Prompt,
        on_delta: &(dyn Fn(String) + Send + Sync),
    ) -> Result<String> {
        let req = http::client()
            .post(format!("{}/api/chat", base(endpoint)))
            .json(&chat_body(model, prompt, true));
        let mut acc = String::new();
        http::stream_ndjson(NAME, req, |line| {
            if let Some(err) = stream_error(line) {
                return Err(AppError::Provider(format!("{NAME}: {err}")));
            }
            if let Some(delta) = stream_delta(line) {
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
    fn trims_trailing_slash_from_endpoint() {
        assert_eq!(base("http://localhost:11434/"), "http://localhost:11434");
        assert_eq!(base("http://localhost:11434"), "http://localhost:11434");
    }

    #[test]
    fn parses_and_sorts_tag_names() {
        let json = r#"{"models":[{"name":"qwen2.5:7b"},{"name":"llama3:latest"}]}"#;
        let resp: TagsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(model_ids(resp), vec!["llama3:latest", "qwen2.5:7b"]);
    }

    #[test]
    fn extracts_non_streaming_chat_text() {
        let json = r#"{"message":{"role":"assistant","content":"Hola"},"done":true}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.into_text().unwrap(), "Hola");
    }

    #[test]
    fn empty_chat_content_is_an_error() {
        let json = r#"{"message":{"role":"assistant","content":""},"done":true}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.into_text().is_err());
    }

    #[test]
    fn stream_delta_extracts_message_content() {
        let line = r#"{"message":{"role":"assistant","content":"He"},"done":false}"#;
        assert_eq!(stream_delta(line).as_deref(), Some("He"));
    }

    #[test]
    fn stream_delta_ignores_empty_final_chunk() {
        // The terminating chunk carries done:true with empty content.
        let line = r#"{"message":{"role":"assistant","content":""},"done":true}"#;
        assert_eq!(stream_delta(line), None);
    }

    #[test]
    fn surfaces_error_field_streaming_and_non_streaming() {
        // 200-status error body: both paths report the cause, not "no text".
        let line = r#"{"error":"model 'foo' not found"}"#;
        assert_eq!(stream_error(line).as_deref(), Some("model 'foo' not found"));
        assert_eq!(stream_error(r#"{"message":{"content":"hi"}}"#), None);

        let resp: ChatResponse = serde_json::from_str(line).unwrap();
        let err = resp.into_text().unwrap_err().to_string();
        assert!(err.contains("model 'foo' not found"));
    }
}
