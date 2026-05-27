//! Shared HTTP helpers for the provider adapters (P0-008).
//!
//! One pooled `reqwest` client (system TLS, no OpenSSL — PRD §15.5), plus
//! error normalization so every adapter surfaces compact, actionable messages
//! (PRD §10.2 failure states) without leaking API keys (FR-051): transport
//! errors are stripped of their URL (some providers carry the key in the query
//! string), and auth failures never echo the provider's body.

use crate::error::{AppError, Result};
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use std::time::Duration;

/// Process-wide client, built once. Cloning a `reqwest::Client` shares the
/// connection pool, so adapters call `client()` freely.
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("TLiquid/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build the HTTP client")
    })
}

/// Send `request`, returning the deserialized JSON body on success or a
/// normalized [`AppError::Provider`] otherwise. `provider` is the display name
/// used to prefix messages.
pub async fn send_json<T: DeserializeOwned>(
    provider: &str,
    request: reqwest::RequestBuilder,
) -> Result<T> {
    let resp = request
        .send()
        .await
        .map_err(|e| transport_error(provider, e))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Provider(http_error_message(
            provider,
            status.as_u16(),
            &body,
        )));
    }
    resp.json::<T>()
        .await
        .map_err(|_| AppError::Provider(format!("{provider}: could not parse the response.")))
}

/// Send `request` purely to validate a key: `Ok(true)` on success, `Ok(false)`
/// on 401/403 (key rejected), `Err` for anything else.
pub async fn validate_status(provider: &str, request: reqwest::RequestBuilder) -> Result<bool> {
    let resp = request
        .send()
        .await
        .map_err(|e| transport_error(provider, e))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(true);
    }
    if matches!(status.as_u16(), 401 | 403) {
        return Ok(false);
    }
    let body = resp.text().await.unwrap_or_default();
    Err(AppError::Provider(http_error_message(
        provider,
        status.as_u16(),
        &body,
    )))
}

/// Turn a `reqwest` transport error into a compact message. `without_url()`
/// drops the request URL so a key embedded in a query string can't leak.
fn transport_error(provider: &str, e: reqwest::Error) -> AppError {
    let e = e.without_url();
    let kind = if e.is_timeout() {
        "request timed out"
    } else if e.is_connect() {
        "could not connect (check your network)"
    } else {
        "network error"
    };
    AppError::Provider(format!("{provider}: {kind}."))
}

/// Map an HTTP status (+ optional provider error body) into a compact,
/// actionable message. Pure, so it is unit-tested. For 401/403 the body is
/// intentionally ignored — provider auth errors can echo a (masked) key, and
/// the category alone is already actionable.
pub fn http_error_message(provider: &str, status: u16, body: &str) -> String {
    let category = match status {
        401 | 403 => "invalid or unauthorized API key",
        404 => "model or endpoint not found",
        408 => "request timed out",
        429 => "rate limited — check your plan/quota",
        500..=599 => "provider service error",
        _ => "request failed",
    };
    let detail = if matches!(status, 401 | 403) {
        None
    } else {
        extract_error_detail(body)
    };
    match detail {
        Some(d) => format!("{provider}: {category} ({d})"),
        None => format!("{provider}: {category} (HTTP {status})"),
    }
}

/// Pull a short message out of a provider error body, handling the common
/// `{"error":{"message":…}}`, `{"error":"…"}`, and `{"message":…}` shapes.
/// Collapsed to one line and truncated so it stays compact.
fn extract_error_detail(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let msg = v
        .get("error")
        .and_then(|e| {
            e.get("message")
                .and_then(|m| m.as_str())
                .or_else(|| e.as_str())
        })
        .or_else(|| v.get("message").and_then(|m| m.as_str()))?;
    let one_line = msg.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.is_empty() {
        return None;
    }
    Some(truncate(&one_line, 160))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_errors_use_category_and_never_echo_the_body() {
        // A 401 body can contain a masked key; we must not surface it.
        let body = r#"{"error":{"message":"Incorrect API key provided: sk-abc...xyz"}}"#;
        let msg = http_error_message("OpenAI", 401, body);
        assert!(msg.contains("invalid or unauthorized API key"));
        assert!(!msg.contains("sk-abc"));
    }

    #[test]
    fn non_auth_errors_include_a_short_provider_detail() {
        let body = r#"{"error":{"message":"The model `gpt-foo` does not exist"}}"#;
        let msg = http_error_message("OpenAI", 404, body);
        assert!(msg.contains("model or endpoint not found"));
        assert!(msg.contains("does not exist"));
    }

    #[test]
    fn rate_limit_maps_to_429_category() {
        let msg = http_error_message("Anthropic", 429, "");
        assert!(msg.contains("rate limited"));
        assert!(msg.starts_with("Anthropic:"));
    }

    #[test]
    fn server_errors_map_to_5xx_category() {
        assert!(http_error_message("Gemini", 503, "").contains("provider service error"));
    }

    #[test]
    fn handles_string_error_and_top_level_message_shapes() {
        assert!(http_error_message("X", 400, r#"{"error":"bad request"}"#).contains("bad request"));
        assert!(http_error_message("X", 400, r#"{"message":"nope"}"#).contains("nope"));
    }

    #[test]
    fn unparseable_body_falls_back_to_status_code() {
        let msg = http_error_message("X", 400, "<html>gateway</html>");
        assert!(msg.contains("HTTP 400"));
    }

    #[test]
    fn detail_is_collapsed_to_one_line_and_truncated() {
        let long = "a ".repeat(200);
        let body = format!(r#"{{"error":{{"message":"{long}"}}}}"#);
        let msg = http_error_message("X", 400, &body);
        assert!(!msg.contains('\n'));
        assert!(msg.ends_with('…') || msg.len() < 200 + 40);
    }
}
