//! SSE (Server-Sent Events) streaming client.
//!
//! Streams token chunks from `/api/generate` (Ollama) and
//! `/v1/chat/completions` (OpenAI-compatible) endpoints into a
//! `tokio::sync::mpsc` channel using zero-copy `Bytes` chunks.
//!
//! Cancellation is supported via `tokio_util::sync::CancellationToken`.

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::error::DiscoveryError;

/// A single token event parsed from an SSE stream.
#[derive(Debug, Clone)]
pub enum SseToken {
    /// A text delta from the model.
    Delta(String),
    /// The stream finished cleanly (`[DONE]` received or `done: true`).
    Done,
}

/// Parse a single SSE `data:` line into an `SseToken` for Ollama's
/// `/api/generate` and `/api/chat` format.
///
/// Ollama sends newline-delimited JSON objects with a `"response"` field
/// (or `"message.content"` for chat) and a `"done"` boolean.
pub fn parse_ollama_chunk(data: &str) -> Option<SseToken> {
    let data = data.trim();
    if data.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(data) {
        Ok(json) => {
            if json.get("done").and_then(Value::as_bool).unwrap_or(false) {
                return Some(SseToken::Done);
            }
            json.get("response")
                .or_else(|| json.pointer("/message/content"))
                .and_then(Value::as_str)
                .map(|s| SseToken::Delta(s.to_owned()))
        }
        Err(e) => {
            warn!("Ollama chunk parse error: {e} | raw: {data:?}");
            None
        }
    }
}

/// Parse a single SSE `data:` line into an `SseToken` for OpenAI-compatible
/// `/v1/chat/completions` streaming format.
///
/// OpenAI sends `data: {...}` lines and `data: [DONE]` to terminate.
pub fn parse_openai_chunk(data: &str) -> Option<SseToken> {
    let data = data.trim();
    if data == "[DONE]" {
        return Some(SseToken::Done);
    }
    if data.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(data) {
        Ok(json) => {
            // choices[0].delta.content or legacy choices[0].text
            json.pointer("/choices/0/delta/content")
                .or_else(|| json.pointer("/choices/0/text"))
                .and_then(Value::as_str)
                .map(|s| SseToken::Delta(s.to_owned()))
        }
        Err(e) => {
            warn!("OpenAI chunk parse error: {e} | raw: {data:?}");
            None
        }
    }
}

/// Helper to parse a single line for the given SSE format.
fn parse_line_for_format(line: &str, format: SseFormat) -> Option<SseToken> {
    match format {
        SseFormat::OpenAi => {
            if let Some(rest) = line.strip_prefix("data:") {
                let rest = rest.trim();
                parse_openai_chunk(rest)
            } else {
                debug!("SSE non-data line: {line:?}");
                None
            }
        }
        SseFormat::Ollama => parse_ollama_chunk(line),
    }
}

/// Which streaming format to use when interpreting SSE chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseFormat {
    /// Ollama `/api/generate` NDJSON format
    Ollama,
    /// OpenAI-compatible `/v1/chat/completions` SSE format
    OpenAi,
}

/// Stream tokens from an SSE endpoint into `tx`.
///
/// The raw HTTP bytes are accumulated in a byte buffer to prevent multi-byte
/// UTF-8 corruption across chunk boundaries. Complete lines delimited by `\n`
/// are parsed as tokens.
///
/// Returns `Ok(())` when the stream ends cleanly or the cancellation token
/// fires. Returns `Err(_)` on HTTP or transport failures.
pub async fn stream_sse(
    _client: &Client,
    request: reqwest::RequestBuilder,
    format: SseFormat,
    tx: mpsc::Sender<SseToken>,
    cancel: CancellationToken,
) -> Result<(), DiscoveryError> {
    let response = tokio::select! {
        res = request.send() => res.map_err(|e| DiscoveryError::Http(e.to_string()))?,
        _ = cancel.cancelled() => return Ok(()),
    };

    if !response.status().is_success() {
        return Err(DiscoveryError::Http(format!(
            "SSE endpoint returned HTTP {}",
            response.status()
        )));
    }

    let mut byte_stream = response.bytes_stream();
    // Raw byte buffer prevents UTF-8 character corruption on chunk boundaries
    let mut byte_buf: Vec<u8> = Vec::new();

    loop {
        tokio::select! {
            maybe_chunk = byte_stream.next() => {
                let chunk: Bytes = match maybe_chunk {
                    None => break,  // stream ended
                    Some(Ok(b)) => b,
                    Some(Err(e)) => {
                        warn!("SSE stream read error: {e}");
                        return Err(DiscoveryError::Http(e.to_string()));
                    }
                };

                byte_buf.extend_from_slice(&chunk);

                // Process all complete lines in the buffer
                while let Some(newline_pos) = byte_buf.iter().position(|&b| b == b'\n') {
                    let line_bytes: Vec<u8> = byte_buf.drain(..=newline_pos).collect();
                    let line = String::from_utf8_lossy(&line_bytes);
                    let line = line.trim_end_matches(['\n', '\r']);

                    if let Some(token) = parse_line_for_format(line, format) {
                        match token {
                            SseToken::Done => {
                                let _ = tx.send(SseToken::Done).await;
                                return Ok(());
                            }
                            tok => {
                                tokio::select! {
                                    res = tx.send(tok) => {
                                        if res.is_err() {
                                            // Receiver dropped — treat as cancellation
                                            return Ok(());
                                        }
                                    }
                                    _ = cancel.cancelled() => {
                                        debug!("SSE stream cancelled during send");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ = cancel.cancelled() => {
                debug!("SSE stream cancelled");
                return Ok(());
            }
        }
    }

    // Process any remaining bytes if stream ended without a trailing newline
    if !byte_buf.is_empty() {
        let line = String::from_utf8_lossy(&byte_buf);
        let line = line.trim_end_matches(['\n', '\r']);
        if let Some(token) = parse_line_for_format(line, format) {
            match token {
                SseToken::Done => {
                    let _ = tx.send(SseToken::Done).await;
                    return Ok(());
                }
                tok => {
                    let _ = tx.send(tok).await;
                }
            }
        }
    }

    // Stream ended without explicit [DONE]; send Done anyway
    let _ = tx.send(SseToken::Done).await;
    Ok(())
}

/// Stream tokens from an SSE endpoint directly to the global `ModelStatusEvent` broadcast channel.
///
/// Converts each `SseToken::Delta(token)` into `ModelStatusEvent::TokenGenerated { token }`
/// and `SseToken::Done` into `ModelStatusEvent::GenerationComplete`.
pub async fn stream_sse_to_event_bus(
    client: &Client,
    request: reqwest::RequestBuilder,
    format: SseFormat,
    event_tx: tokio::sync::broadcast::Sender<ride_core::ModelStatusEvent>,
    cancel: CancellationToken,
) -> Result<(), DiscoveryError> {
    let (tx, mut rx) = mpsc::channel::<SseToken>(64);
    let cancel_stream = cancel.clone();

    let client = client.clone();
    let stream_handle = tokio::spawn(async move {
        stream_sse(&client, request, format, tx, cancel_stream).await
    });

    while let Some(token) = rx.recv().await {
        match token {
            SseToken::Delta(tok) => {
                let _ = event_tx.send(ride_core::ModelStatusEvent::TokenGenerated { token: tok });
            }
            SseToken::Done => {
                let _ = event_tx.send(ride_core::ModelStatusEvent::GenerationComplete);
                break;
            }
        }
    }

    match stream_handle.await {
        Ok(res) => res,
        Err(e) => Err(DiscoveryError::Http(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Ollama chunk parsing ──────────────────────────────────────────────────

    #[test]
    fn test_parse_ollama_delta() {
        let raw = r#"{"model":"llama3","response":" Hello","done":false}"#;
        let tok = parse_ollama_chunk(raw).expect("should parse");
        assert!(matches!(tok, SseToken::Delta(ref s) if s == " Hello"));
    }

    #[test]
    fn test_parse_ollama_chat_delta() {
        let raw = r#"{"model":"llama3","message":{"role":"assistant","content":" Bonjour"},"done":false}"#;
        let tok = parse_ollama_chunk(raw).expect("should parse chat delta");
        assert!(matches!(tok, SseToken::Delta(ref s) if s == " Bonjour"));
    }

    #[test]
    fn test_parse_ollama_done() {
        let raw = r#"{"model":"llama3","response":"","done":true}"#;
        let tok = parse_ollama_chunk(raw).expect("should parse");
        assert!(matches!(tok, SseToken::Done));
    }

    #[test]
    fn test_parse_ollama_malformed_recovery() {
        // Malformed JSON → None (no panic, no crash)
        let result = parse_ollama_chunk("{not valid json");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_ollama_empty() {
        assert!(parse_ollama_chunk("").is_none());
        assert!(parse_ollama_chunk("   ").is_none());
    }

    // ── OpenAI chunk parsing ──────────────────────────────────────────────────

    #[test]
    fn test_parse_openai_delta() {
        let raw = r#"{"id":"chatcmpl-1","choices":[{"delta":{"content":" world"},"index":0}]}"#;
        let tok = parse_openai_chunk(raw).expect("should parse");
        assert!(matches!(tok, SseToken::Delta(ref s) if s == " world"));
    }

    #[test]
    fn test_parse_openai_text_completion() {
        let raw = r#"{"id":"cmpl-1","choices":[{"text":" universe","index":0}]}"#;
        let tok = parse_openai_chunk(raw).expect("should parse legacy completion");
        assert!(matches!(tok, SseToken::Delta(ref s) if s == " universe"));
    }

    #[test]
    fn test_parse_openai_done() {
        assert!(matches!(parse_openai_chunk("[DONE]"), Some(SseToken::Done)));
    }

    #[test]
    fn test_parse_openai_empty_delta() {
        // choices[0].delta exists but content is absent → None
        let raw = r#"{"choices":[{"delta":{},"index":0}]}"#;
        assert!(parse_openai_chunk(raw).is_none());
    }

    #[test]
    fn test_parse_openai_malformed_recovery() {
        assert!(parse_openai_chunk("{bad json!!").is_none());
    }

    #[test]
    fn test_parse_openai_empty() {
        assert!(parse_openai_chunk("").is_none());
    }

    // ── Multi-line data field simulation ─────────────────────────────────────

    #[test]
    fn test_multiline_ollama_sequence() {
        let lines = vec![
            r#"{"response":"Hello","done":false}"#,
            r#"{"response":" world","done":false}"#,
            r#"{"response":"","done":true}"#,
        ];
        let tokens: Vec<_> = lines
            .iter()
            .filter_map(|l| parse_ollama_chunk(l))
            .collect();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], SseToken::Delta(s) if s == "Hello"));
        assert!(matches!(&tokens[1], SseToken::Delta(s) if s == " world"));
        assert!(matches!(&tokens[2], SseToken::Done));
    }

    #[test]
    fn test_multiline_openai_sequence() {
        let lines = vec![
            r#"{"choices":[{"delta":{"content":"Hi"},"index":0}]}"#,
            r#"{"choices":[{"delta":{"content":"!"},"index":0}]}"#,
            "[DONE]",
        ];
        let tokens: Vec<_> = lines
            .iter()
            .filter_map(|l| parse_openai_chunk(l))
            .collect();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], SseToken::Delta(s) if s == "Hi"));
        assert!(matches!(&tokens[1], SseToken::Delta(s) if s == "!"));
        assert!(matches!(&tokens[2], SseToken::Done));
    }
}
