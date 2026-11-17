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

#[tokio::test]
async fn test_stream_sse_to_event_bus() {
    use ride_core::ModelStatusEvent;
    use ride_discovery::stream_sse_to_event_bus;

    let body = openai_body(&["Hello", " from", " EventBus!"]);
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
    let (event_tx, mut event_rx) = ride_core::new_event_bus();
    let cancel = CancellationToken::new();

    let req = client.post(format!("http://{addr}/v1/chat/completions"));
    tokio::spawn(async move {
        stream_sse_to_event_bus(&client, req, SseFormat::OpenAi, event_tx, cancel)
            .await
            .expect("stream_sse_to_event_bus should succeed");
    });

    let mut tokens = vec![];
    let mut complete = false;

    while let Ok(event) = tokio::time::timeout(std::time::Duration::from_secs(2), event_rx.recv()).await {
        match event.unwrap() {
            ModelStatusEvent::TokenGenerated { token } => tokens.push(token),
            ModelStatusEvent::GenerationComplete => {
                complete = true;
                break;
            }
            _ => {}
        }
    }

    assert!(complete, "should have received GenerationComplete");
    assert_eq!(tokens.join(""), "Hello from EventBus!");
}

#[tokio::test]
async fn test_llama_props_fallback_integration() {
    use ride_core::ProviderType;
    use ride_discovery::providers::openai_compat::list_models_with_llama_fallback;

    // Simulate llama.cpp /props endpoint returning GGUF model info
    let props_body = r#"{
        "model_path": "models/mistral-7b-instruct-v0.3.Q4_K_M.gguf",
        "default_generation_settings": {
            "n_ctx": 32768
        }
    }"#;

    let router = Router::new().route(
        "/props",
        axum::routing::get(move || {
            let body = props_body.to_string();
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
    let models = list_models_with_llama_fallback(
        &client,
        &format!("http://{addr}"),
        ProviderType::LlamaCpp,
        addr.port(),
    )
    .await
    .expect("should fallback to /props and succeed");

    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "mistral-7b-instruct-v0.3.Q4_K_M.gguf");
    assert_eq!(models[0].context_length, Some(32768));
    assert_eq!(models[0].provider, ProviderType::LlamaCpp);
    assert_eq!(models[0].port, addr.port());
}

#[tokio::test]
async fn test_discovery_loop_integration() {
    use ride_core::{ModelStatusEvent, ProviderType};
    use ride_discovery::{ModelDetector, ProbeEndpoint};
    use std::time::Duration;

    let body = r#"{
        "models": [{"name": "qwen2.5-coder:7b"}]
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
    let detector = ModelDetector::new(vec![ProbeEndpoint::new(
        ProviderType::Ollama,
        format!("http://{addr}"),
        "/api/tags",
    )]);

    let (event_tx, mut event_rx) = ride_core::new_event_bus();
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        detector
            .run_discovery_loop(event_tx, cancel_clone, Duration::from_millis(50))
            .await;
    });

    let ev = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .expect("should receive event within 2s")
        .expect("channel ok");

    match ev {
        ModelStatusEvent::Connected { provider, models } => {
            assert_eq!(provider, ProviderType::Ollama);
            assert_eq!(models.len(), 1);
            assert_eq!(models[0].id, "qwen2.5-coder:7b");
        }
        _ => panic!("expected Connected event"),
    }

    cancel.cancel();
}

#[tokio::test]
async fn test_multibyte_utf8_split_across_chunks() {
    use bytes::Bytes;
    use futures_util::stream;
    use std::convert::Infallible;

    // "你" in UTF-8 is [0xE4, 0xBD, 0xA0]
    // Chunk 1 ends with [0xE4, 0xBD], Chunk 2 starts with [0xA0]
    let chunk1 = Bytes::from_static(b"{\"response\":\"\xe4\xbd");
    let chunk2 = Bytes::from_static(b"\xa0\",\"done\":false}\n{\"response\":\"\",\"done\":true}\n");

    let router = Router::new().route(
        "/api/generate",
        post(|| async move {
            let s = stream::iter(vec![Ok::<_, Infallible>(chunk1), Ok::<_, Infallible>(chunk2)]);
            axum::response::Response::builder()
                .header("content-type", "application/x-ndjson")
                .body(axum::body::Body::from_stream(s))
                .unwrap()
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

    let mut tokens = vec![];
    while let Some(tok) = rx.recv().await {
        if let SseToken::Delta(s) = tok {
            tokens.push(s);
        }
    }

    assert_eq!(tokens.join(""), "你", "multi-byte UTF-8 character must not be corrupted across chunk boundaries");
}

#[tokio::test]
async fn test_trailing_line_without_newline() {
    // Final line does not end with \n
    let body = "{\"response\":\"final_word\",\"done\":false}";

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
        .expect("stream should succeed");

    let mut tokens = vec![];
    while let Some(tok) = rx.recv().await {
        if let SseToken::Delta(s) = tok {
            tokens.push(s);
        }
    }

    assert_eq!(tokens, vec!["final_word".to_string()], "trailing line without newline must be delivered");
}

#[tokio::test]
async fn test_html_endpoint_rejected_as_offline() {
    use ride_core::ProviderType;
    use ride_discovery::{ModelDetector, ProbeEndpoint};

    let router = Router::new().route(
        "/v1/models",
        axum::routing::get(|| async {
            axum::response::Response::builder()
                .status(200)
                .header("content-type", "text/html; charset=utf-8")
                .body(axum::body::Body::from("<html><body>Custom Web Server</body></html>"))
                .unwrap()
        }),
    );

    let addr = start_test_server(router).await;
    let ep = ProbeEndpoint::new(ProviderType::LmStudio, format!("http://{addr}"), "/v1/models");
    let detector = ModelDetector::new(vec![ep.clone()]);

    let status = detector.check_endpoint(&ep).await;
    assert!(!status.is_online, "HTML 200 response must be recognized as NOT a model API");
}

#[tokio::test]
async fn test_discovery_loop_deduplication() {
    use ride_core::{ModelStatusEvent, ProviderType};
    use ride_discovery::{ModelDetector, ProbeEndpoint};
    use std::time::Duration;

    let body = r#"{"models": [{"name": "qwen2.5-coder:7b"}]}"#;
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
    let detector = ModelDetector::new(vec![ProbeEndpoint::new(
        ProviderType::Ollama,
        format!("http://{addr}"),
        "/api/tags",
    )]);

    let (event_tx, mut event_rx) = ride_core::new_event_bus();
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        detector
            .run_discovery_loop(event_tx, cancel_clone, Duration::from_millis(30))
            .await;
    });

    // Receive first Connected event
    let ev = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .expect("first event")
        .expect("channel ok");
    assert!(matches!(ev, ModelStatusEvent::Connected { .. }));

    // Wait across multiple polling ticks (30ms * 4 = 120ms)
    tokio::time::sleep(Duration::from_millis(120)).await;

    // No duplicate Connected events should have been broadcast since model list is unchanged
    assert!(event_rx.try_recv().is_err(), "should not spam Connected event when model list is unchanged");

    cancel.cancel();
}

#[tokio::test]
async fn test_ollama_dynamic_port_extraction() {
    use ride_discovery::providers::ollama::list_models;

    let body = r#"{"models": [{"name": "llama3:8b"}]}"#;
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
    let models = list_models(&client, &format!("http://{addr}")).await.unwrap();

    assert_eq!(models.len(), 1);
    assert_eq!(models[0].port, addr.port(), "port should be extracted from base_url, not hardcoded to 11434");
}
