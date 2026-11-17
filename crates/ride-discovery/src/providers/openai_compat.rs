//! OpenAI-compatible provider integration.
//!
//! Used for: LM Studio (`:1234`), Colibrì/Rabbit (`:5000`/`:8080`),
//!           llama.cpp (`:8080`), vLLM (`:8000`).
//!
//! Discovery: `GET /v1/models` → `.data[].id`
//! Streaming: `POST /v1/chat/completions` with `"stream": true`

use reqwest::Client;
use ride_core::{LocalModel, ProviderType};
use serde_json::Value;
use tracing::debug;

use crate::error::DiscoveryError;

/// List models from an OpenAI-compatible `/v1/models` endpoint.
pub async fn list_models(
    client: &Client,
    base_url: &str,
    provider: ProviderType,
    port: u16,
) -> Result<Vec<LocalModel>, DiscoveryError> {
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    debug!("OpenAI-compat list_models: {url}");

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DiscoveryError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(DiscoveryError::Http(format!(
            "/v1/models returned {}",
            resp.status()
        )));
    }

    let json: Value = resp.json().await.map_err(|e| DiscoveryError::Parse(e.to_string()))?;

    let models = json
        .get("data")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let id = m.get("id").and_then(Value::as_str)?;
                    let context_length = m
                        .get("context_length")
                        .or_else(|| m.get("max_context_length"))
                        .and_then(Value::as_u64)
                        .map(|v| v as usize);

                    // MoE detection heuristic
                    let is_moe = matches!(provider, ProviderType::ColibriMoe | ProviderType::RabbitMoe)
                        || id.to_lowercase().contains("moe")
                        || id.to_lowercase().contains("mixtral")
                        || id.to_lowercase().contains("deepseek-v");

                    Some(LocalModel {
                        id: id.to_owned(),
                        name: id.to_owned(),
                        provider: provider.clone(),
                        context_length,
                        is_moe,
                        port,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Build an OpenAI-compatible streaming chat request body.
pub fn build_chat_request(model: &str, user_message: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            { "role": "user", "content": user_message }
        ],
        "stream": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chat_request() {
        let req = build_chat_request("gpt-4o-mini", "ping");
        assert_eq!(req["model"], "gpt-4o-mini");
        assert_eq!(req["stream"], true);
        let msgs = req["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "ping");
    }

    #[test]
    fn test_moe_provider_flag() {
        // ColibriMoe provider → is_moe should always be true
        let model = LocalModel {
            id: "deepseek-v3".into(),
            name: "deepseek-v3".into(),
            provider: ProviderType::ColibriMoe,
            context_length: Some(131072),
            is_moe: true,
            port: 8080,
        };
        assert!(model.is_moe);
        assert_eq!(model.port, 8080);
    }

    #[test]
    fn test_llama_cpp_non_moe() {
        let model = LocalModel {
            id: "phi-3-mini".into(),
            name: "phi-3-mini".into(),
            provider: ProviderType::LlamaCpp,
            context_length: Some(4096),
            is_moe: false,
            port: 8080,
        };
        assert!(!model.is_moe);
    }
}
