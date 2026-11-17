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
/// `/api/generate` format.
///
/// Ollama sends newline-delimited JSON objects with a `"response"` field
/// and a `"done"` boolean.
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
            // choices[0].delta.content
            json.pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
                .map(|s| SseToken::Delta(s.to_owned()))
        }
        Err(e) => {
            warn!("OpenAI chunk parse error: {e} | raw: {data:?}");
            None
        }
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
/// The raw HTTP bytes are iterated without intermediate full-body allocation.
/// Each `Bytes` chunk from reqwest is parsed line-by-line.
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
    // Buffer to accumulate incomplete lines across chunk boundaries
    let mut line_buf = String::new();

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

                // Process the raw bytes as UTF-8 text, tolerating partial chars
                let text = String::from_utf8_lossy(&chunk);
                line_buf.push_str(&text);

                // Process all complete lines in the buffer
                while let Some(newline_pos) = line_buf.find('\n') {
                    let line: String = line_buf.drain(..=newline_pos).collect();
                    let line = line.trim_end_matches(['\n', '\r']);

                    // SSE lines starting with "data:" (OpenAI format)
                    // Ollama uses raw NDJSON (no "data:" prefix)
                    let data = match format {
                        SseFormat::OpenAi => {
                            if let Some(rest) = line.strip_prefix("data:") {
                                rest.trim()
                            } else {
                                // skip comment lines (": ping") and event: lines
                                debug!("SSE non-data line: {line:?}");
                                continue;
                            }
                        }
                        SseFormat::Ollama => line,
                    };

                    let token = match format {
                        SseFormat::Ollama => parse_ollama_chunk(data),
                        SseFormat::OpenAi => parse_openai_chunk(data),
                    };

                    match token {
                        Some(SseToken::Done) => {
                            let _ = tx.send(SseToken::Done).await;
                            return Ok(());
                        }
                        Some(tok) => {
                            // Use select so cancellation can interrupt a blocked send
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
                        None => {}
                    }
                }
            }
            _ = cancel.cancelled() => {
                debug!("SSE stream cancelled");
                return Ok(());
            }
        }
    }

    // Stream ended without explicit [DONE]; send Done anyway
    let _ = tx.send(SseToken::Done).await;
    Ok(())
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
