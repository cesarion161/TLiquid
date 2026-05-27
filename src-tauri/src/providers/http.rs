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

/// Stream a Server-Sent Events response (P1-009). Sends `request`, normalizes a
/// non-success status exactly like [`send_json`] (so a 401 streaming request
/// still yields the no-key-leak message), then reads the body incrementally and
/// invokes `on_data` once per `data:` payload line — excluding the `[DONE]`
/// sentinel and empty keep-alive lines. Adapters parse each payload themselves,
/// so this stays provider-neutral.
///
/// Bytes are buffered and only decoded a full line at a time, so a multi-byte
/// UTF-8 character split across two network chunks is never corrupted.
pub async fn stream_sse(
    provider: &str,
    request: reqwest::RequestBuilder,
    mut on_data: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    use futures_util::StreamExt;

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

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| transport_error(provider, e))?;
        buf.extend_from_slice(&chunk);
        // Drain whole lines as they complete; partial trailing bytes stay buffered.
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            if let Some(payload) = sse_data_payload(&line) {
                on_data(payload)?;
            }
        }
        // Guard against a non-conformant endpoint streaming an unbounded line
        // with no newline (only the in-progress partial line is ever buffered).
        if buf.len() > MAX_SSE_LINE_BYTES {
            return Err(AppError::Provider(format!(
                "{provider}: streaming response line exceeded the size limit."
            )));
        }
    }
    // Flush a final line that ended at EOF without a trailing newline, so the
    // last delta isn't lost on a stream that doesn't end with a blank line.
    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        if let Some(payload) = sse_data_payload(&line) {
            on_data(payload)?;
        }
    }
    Ok(())
}

/// Cap on a single un-terminated SSE line held in the read buffer. Translation
/// chunks are tiny; this only bounds a pathological newline-less stream.
const MAX_SSE_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Extract the `data:` payload of a single SSE line, if it carries real data.
/// Returns `None` for `event:`/comment/blank lines, the `[DONE]` sentinel, and
/// empty payloads. Pure, so it is unit-tested.
fn sse_data_payload(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    let data = line.strip_prefix("data:")?.trim();
    if data.is_empty() || data == "[DONE]" {
        None
    } else {
        Some(data)
    }
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

    #[test]
    fn sse_payload_extracts_data_lines() {
        assert_eq!(sse_data_payload("data: {\"a\":1}\n"), Some("{\"a\":1}"));
        // Tolerates a missing space after the colon and CRLF endings.
        assert_eq!(sse_data_payload("data:{\"a\":1}\r\n"), Some("{\"a\":1}"));
        // A trailing line with no newline (the EOF-flush case) still parses.
        assert_eq!(sse_data_payload("data: {\"a\":1}"), Some("{\"a\":1}"));
    }

    #[test]
    fn sse_payload_skips_done_blank_and_non_data_lines() {
        assert_eq!(sse_data_payload("data: [DONE]\n"), None);
        assert_eq!(sse_data_payload("data: \n"), None);
        assert_eq!(sse_data_payload("\n"), None);
        assert_eq!(sse_data_payload("event: message\n"), None);
        assert_eq!(sse_data_payload(": keep-alive comment\n"), None);
    }
}
