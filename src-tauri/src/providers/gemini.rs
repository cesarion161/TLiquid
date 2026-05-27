//! Google Gemini adapter — direct BYOK calls to generateContent (P0-008).
//!
//! Auth uses the `x-goog-api-key` header rather than the `?key=` query param,
//! so the key never appears in a URL (and thus never in a transport error).

use super::http;
use super::{Prompt, Provider, ProviderId};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use serde::Deserialize;

const BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const NAME: &str = "Gemini";
const API_KEY_HEADER: &str = "x-goog-api-key";

pub struct Gemini;

#[derive(Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<Content>,
}

#[derive(Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    #[serde(default)]
    text: String,
}

impl GenerateResponse {
    fn into_text(self) -> Result<String> {
        let text: String = self
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content)
            .map(|content| {
                content
                    .parts
                    .into_iter()
                    .map(|p| p.text)
                    .collect::<String>()
            })
            .unwrap_or_default();
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
    models: Vec<ModelEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelEntry {
    /// Resource name, e.g. "models/gemini-2.5-flash".
    name: String,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
}

/// Pull the incremental text from one streaming chunk: concatenate the text of
/// `candidates[0].content.parts[*]`. Pure, so it is unit-tested.
fn stream_delta(data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    let parts = v
        .get("candidates")?
        .get(0)?
        .get("content")?
        .get("parts")?
        .as_array()?;
    let text: String = parts
        .iter()
        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
        .collect();
    (!text.is_empty()).then_some(text)
}

/// Keep only models usable for text generation, stripping the "models/" prefix.
fn model_ids(resp: ModelsResponse) -> Vec<String> {
    resp.models
        .into_iter()
        .filter(|m| {
            m.supported_generation_methods.is_empty()
                || m.supported_generation_methods
                    .iter()
                    .any(|s| s == "generateContent")
        })
        .map(|m| {
            m.name
                .strip_prefix("models/")
                .unwrap_or(&m.name)
                .to_string()
        })
        .collect()
}

#[async_trait]
impl Provider for Gemini {
    fn id(&self) -> ProviderId {
        ProviderId::Gemini
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
            .header(API_KEY_HEADER, api_key);
        http::validate_status(NAME, req).await
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<String>> {
        let req = http::client()
            .get(format!("{BASE}/models"))
            .header(API_KEY_HEADER, api_key);
        let resp: ModelsResponse = http::send_json(NAME, req).await?;
        Ok(model_ids(resp))
    }

    async fn translate(&self, api_key: &str, model: &str, prompt: &Prompt) -> Result<String> {
        let body = serde_json::json!({
            "systemInstruction": { "parts": [ { "text": prompt.system } ] },
            "contents": [ { "role": "user", "parts": [ { "text": prompt.user } ] } ],
        });
        let req = http::client()
            .post(format!("{BASE}/models/{model}:generateContent"))
            .header(API_KEY_HEADER, api_key)
            .json(&body);
        let resp: GenerateResponse = http::send_json(NAME, req).await?;
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
            "systemInstruction": { "parts": [ { "text": prompt.system } ] },
            "contents": [ { "role": "user", "parts": [ { "text": prompt.user } ] } ],
        });
        // `?alt=sse` makes streamGenerateContent emit SSE `data:` frames rather
        // than a single JSON array, so the shared SSE reader can consume it.
        let req = http::client()
            .post(format!(
                "{BASE}/models/{model}:streamGenerateContent?alt=sse"
            ))
            .header(API_KEY_HEADER, api_key)
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
    fn extracts_text_from_candidate_parts() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"Hallo"}]}}]}"#;
        let resp: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.into_text().unwrap(), "Hallo");
    }

    #[test]
    fn empty_candidates_is_an_error() {
        let resp: GenerateResponse = serde_json::from_str(r#"{"candidates":[]}"#).unwrap();
        assert!(resp.into_text().is_err());
    }

    #[test]
    fn stream_delta_concatenates_candidate_parts() {
        let chunk = r#"{"candidates":[{"content":{"parts":[{"text":"Hal"},{"text":"lo"}]}}]}"#;
        assert_eq!(stream_delta(chunk).as_deref(), Some("Hallo"));
    }

    #[test]
    fn stream_delta_ignores_chunks_without_text() {
        // A trailing chunk may carry only finishReason/usage, no parts text.
        assert_eq!(
            stream_delta(r#"{"candidates":[{"finishReason":"STOP"}]}"#),
            None
        );
    }

    #[test]
    fn strips_prefix_and_filters_to_text_models() {
        let json = r#"{"models":[
            {"name":"models/gemini-2.5-flash","supportedGenerationMethods":["generateContent"]},
            {"name":"models/embedding-001","supportedGenerationMethods":["embedContent"]}
        ]}"#;
        let resp: ModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(model_ids(resp), vec!["gemini-2.5-flash"]);
    }
}
