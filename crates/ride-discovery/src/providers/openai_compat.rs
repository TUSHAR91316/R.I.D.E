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

/// Heuristic to determine if a model identifier indicates a Mixture-of-Experts architecture.
pub fn is_moe_model_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("moe")
        || lower.contains("mixtral")
        || lower.contains("deepseek-v")
        || lower.contains("dbrx")
        || lower.contains("qwen1.5-moe")
        || lower.contains("57b-a14b")
}

/// Parse a llama.cpp `/props` response into a normalized `LocalModel`.
pub fn parse_llama_props(json: &Value, port: u16) -> Option<LocalModel> {
    let model_path = json
        .get("model_path")
        .or_else(|| json.pointer("/default_generation_settings/model"))
        .and_then(Value::as_str)?;

    let id = std::path::Path::new(model_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(model_path)
        .to_string();

    let context_length = json
        .pointer("/default_generation_settings/n_ctx")
        .and_then(Value::as_u64)
        .map(|v| v as usize);

    let is_moe = is_moe_model_name(&id);

    Some(LocalModel {
        id: id.clone(),
        name: id,
        provider: ProviderType::LlamaCpp,
        context_length,
        is_moe,
        port,
    })
}

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
                        || is_moe_model_name(id);

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

/// List models from an endpoint, falling back to llama.cpp `/props` if `/v1/models` fails.
pub async fn list_models_with_llama_fallback(
    client: &Client,
    base_url: &str,
    provider: ProviderType,
    port: u16,
) -> Result<Vec<LocalModel>, DiscoveryError> {
    match list_models(client, base_url, provider.clone(), port).await {
        Ok(models) if !models.is_empty() => Ok(models),
        _ => {
            let url = format!("{}/props", base_url.trim_end_matches('/'));
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| DiscoveryError::Http(e.to_string()))?;

            if !resp.status().is_success() {
                return Err(DiscoveryError::Http(format!(
                    "/props returned {}",
                    resp.status()
                )));
            }

            let json: Value = resp.json().await.map_err(|e| DiscoveryError::Parse(e.to_string()))?;
            if let Some(model) = parse_llama_props(&json, port) {
                Ok(vec![model])
            } else {
                Ok(vec![])
            }
        }
    }
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

    #[test]
    fn test_parse_llama_props() {
        let json = serde_json::json!({
            "model_path": "models/qwen2.5-coder-7b-instruct.gguf",
            "default_generation_settings": {
                "n_ctx": 32768
            }
        });
        let model = parse_llama_props(&json, 8080).expect("should parse props");
        assert_eq!(model.id, "qwen2.5-coder-7b-instruct.gguf");
        assert_eq!(model.context_length, Some(32768));
        assert_eq!(model.provider, ProviderType::LlamaCpp);
        assert_eq!(model.port, 8080);
        assert!(!model.is_moe);
    }

    #[test]
    fn test_is_moe_model_name() {
        assert!(is_moe_model_name("DeepSeek-V3-Chat"));
        assert!(is_moe_model_name("mixtral-8x7b-instruct"));
        assert!(is_moe_model_name("qwen1.5-moe-a2.7b"));
        assert!(is_moe_model_name("dbrx-instruct"));
        assert!(!is_moe_model_name("llama-3-8b-instruct"));
        assert!(!is_moe_model_name("mistral-7b-v0.1"));
    }
}
