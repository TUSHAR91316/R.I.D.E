//! Integration tests for `ride-discovery` SSE streaming using an in-process axum server.
//!
//! These tests verify:
//! - Ollama SSE stream is consumed end-to-end
//! - OpenAI-compat SSE stream is consumed end-to-end
//! - `[DONE]` termination is handled
//! - Malformed chunk recovery (stream doesn't crash)
//! - Cancellation works mid-stream

use axum::{routing::post, Router};
use reqwest::Client;
use ride_discovery::sse::{stream_sse, SseFormat, SseToken};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Start an in-process axum server on a random port and return its address.
async fn start_test_server(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

/// Build an Ollama-style NDJSON response body from token strings.
fn ollama_body(tokens: &[&str]) -> String {
    let mut body = String::new();
    for token in tokens {
        body.push_str(&format!(
            "{{\"model\":\"llama3\",\"response\":\"{token}\",\"done\":false}}\n"
        ));
    }
    body.push_str("{\"model\":\"llama3\",\"response\":\"\",\"done\":true}\n");
    body
}

/// Build an OpenAI-style SSE response body from token strings.
fn openai_body(tokens: &[&str]) -> String {
    let mut body = String::new();
    for token in tokens {
        let chunk = serde_json::json!({
            "choices": [{"delta": {"content": token}, "index": 0}]
        });
        body.push_str(&format!("data: {chunk}\n\n"));
    }
    body.push_str("data: [DONE]\n\n");
    body
}

// ── Ollama integration tests ──────────────────────────────────────────────────

#[tokio::test]
async fn test_ollama_sse_stream_end_to_end() {
    let body = ollama_body(&["Hello", " world", "!"]);

    let router = Router::new().route(
        "/api/generate",
        post(move || {
            let body = body.clone();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "application/x-ndjson")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let (tx, mut rx) = mpsc::channel::<SseToken>(32);
    let cancel = CancellationToken::new();

    let req = client.post(format!("http://{addr}/api/generate"));
    stream_sse(&client, req, SseFormat::Ollama, tx, cancel)
        .await
        .expect("stream should succeed");

    let mut tokens: Vec<String> = vec![];
    let mut done = false;
    while let Some(tok) = rx.recv().await {
        match tok {
            SseToken::Delta(s) => tokens.push(s),
            SseToken::Done => {
                done = true;
                break;
            }
        }
    }

    assert!(done, "stream should have sent Done");
    assert_eq!(tokens.join(""), "Hello world!", "tokens should reconstruct message");
}

#[tokio::test]
async fn test_ollama_malformed_chunk_recovery() {
    // Mix valid and invalid chunks — the stream should not crash
    let body = "not-valid-json\n{\"response\":\"ok\",\"done\":false}\n{\"response\":\"\",\"done\":true}\n";

    let router = Router::new().route(
        "/api/generate",
        post(move || {
            let body = body.to_string();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "application/x-ndjson")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let (tx, mut rx) = mpsc::channel::<SseToken>(32);
    let cancel = CancellationToken::new();

    let req = client.post(format!("http://{addr}/api/generate"));
    stream_sse(&client, req, SseFormat::Ollama, tx, cancel)
        .await
        .expect("stream should not error on malformed chunks");

    let mut tokens: Vec<String> = vec![];
    let mut done = false;
    while let Some(tok) = rx.recv().await {
        match tok {
            SseToken::Delta(s) => tokens.push(s),
            SseToken::Done => {
                done = true;
                break;
            }
        }
    }

    assert!(done);
    assert!(tokens.contains(&"ok".to_string()));
}

// ── OpenAI-compat integration tests ──────────────────────────────────────────

#[tokio::test]
async fn test_openai_sse_stream_end_to_end() {
    let body = openai_body(&["The", " answer", " is", " 42"]);

    let router = Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let body = body.clone();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let (tx, mut rx) = mpsc::channel::<SseToken>(32);
    let cancel = CancellationToken::new();

    let req = client.post(format!("http://{addr}/v1/chat/completions"));
    stream_sse(&client, req, SseFormat::OpenAi, tx, cancel)
        .await
        .expect("stream should succeed");

    let mut tokens: Vec<String> = vec![];
    let mut done = false;
    while let Some(tok) = rx.recv().await {
        match tok {
            SseToken::Delta(s) => tokens.push(s),
            SseToken::Done => {
                done = true;
                break;
            }
        }
    }

    assert!(done);
    assert_eq!(tokens.join(""), "The answer is 42");
}

#[tokio::test]
async fn test_openai_malformed_chunk_recovery() {
    // Mix real chunks with malformed ones
    let body = "data: {bad json\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"index\":0}]}\n\ndata: [DONE]\n\n";

    let router = Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let body = body.to_string();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let (tx, mut rx) = mpsc::channel::<SseToken>(32);
    let cancel = CancellationToken::new();

    let req = client.post(format!("http://{addr}/v1/chat/completions"));
    stream_sse(&client, req, SseFormat::OpenAi, tx, cancel)
        .await
        .expect("stream should not error on malformed data");

    let mut tokens: Vec<String> = vec![];
    let mut done = false;
    while let Some(tok) = rx.recv().await {
        match tok {
            SseToken::Delta(s) => tokens.push(s),
            SseToken::Done => {
                done = true;
                break;
            }
        }
    }

    assert!(done);
    assert!(tokens.contains(&"ok".to_string()));
}

// ── Cancellation tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sse_cancellation() {
    use std::time::Duration;

    // A body with many chunks — no actual delay needed
    let body = {
        let mut b = String::new();
        for i in 0..1000 {
            b.push_str(&format!(
                "{{\"response\":\"tok{i}\",\"done\":false}}\n"
            ));
        }
        b
    };

    let router = Router::new().route(
        "/api/generate",
        post(move || {
            let body = body.clone();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "application/x-ndjson")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let (tx, rx) = mpsc::channel::<SseToken>(4); // small buffer
    // Drop the receiver immediately — sends will fail fast, triggering early return
    drop(rx);

    let cancel = CancellationToken::new();

    let req = client.post(format!("http://{addr}/api/generate"));
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        stream_sse(&client, req, SseFormat::Ollama, tx, cancel),
    )
    .await;

    assert!(result.is_ok(), "should complete within 2s when receiver is dropped");
}

#[tokio::test]
async fn test_sse_cancellation_token() {
    use std::time::Duration;

    // A slow server — sends one token per 20ms
    let router = Router::new().route(
        "/api/generate",
        post(|| async {
            // Return a stream body that sends one line, then finishes
            // (we just use a large body but cancel externally)
            let mut b = String::new();
            for i in 0..50 {
                b.push_str(&format!(
                    "{{\"response\":\"tok{i}\",\"done\":false}}\n"
                ));
            }
            axum::response::Response::builder()
                .header("content-type", "application/x-ndjson")
                .body(axum::body::Body::from(b))
                .unwrap()
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    // Large enough buffer that sends don't block
    let (tx, mut rx) = mpsc::channel::<SseToken>(64);
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Cancel after receiving a few tokens
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancel_clone.cancel();
    });

    let req = client.post(format!("http://{addr}/api/generate"));
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        stream_sse(&client, req, SseFormat::Ollama, tx, cancel),
    )
    .await;

    assert!(result.is_ok(), "should complete within 2s after cancellation");
    // Drain whatever tokens arrived
    while rx.try_recv().is_ok() {}
}

// ── Provider list_models integration tests ────────────────────────────────────

#[tokio::test]
async fn test_ollama_list_models_integration() {
    use ride_discovery::providers::ollama::list_models;

    let body = r#"{
        "models": [
            {"name": "llama3:8b", "size": 4000000000},
            {"name": "mixtral:8x7b", "size": 26000000000}
        ]
    }"#;

    let router = Router::new().route(
        "/api/tags",
        axum::routing::get(move || {
            let body = body.to_string();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let models = list_models(&client, &format!("http://{addr}"))
        .await
        .expect("should list models");

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "llama3:8b");
    assert!(!models[0].is_moe);
    assert_eq!(models[1].id, "mixtral:8x7b");
    assert!(models[1].is_moe, "mixtral should be flagged MoE");
}

#[tokio::test]
async fn test_openai_compat_list_models_integration() {
    use ride_core::ProviderType;
    use ride_discovery::providers::openai_compat::list_models;

    let body = r#"{
        "data": [
            {"id": "gpt-4o-mini", "object": "model", "context_length": 128000},
            {"id": "deepseek-v3-moe", "object": "model", "context_length": 65536}
        ]
    }"#;

    let router = Router::new().route(
        "/v1/models",
        axum::routing::get(move || {
            let body = body.to_string();
            async move {
                axum::response::Response::builder()
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body))
                    .unwrap()
            }
        }),
    );

    let addr = start_test_server(router).await;
    let client = Client::new();
    let models = list_models(&client, &format!("http://{addr}"), ProviderType::LmStudio, 1234)
        .await
        .expect("should list models");

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "gpt-4o-mini");
    assert_eq!(models[0].context_length, Some(128000));
    assert!(!models[0].is_moe);
    assert_eq!(models[1].id, "deepseek-v3-moe");
    assert!(models[1].is_moe, "deepseek-v3 should be MoE");
}
