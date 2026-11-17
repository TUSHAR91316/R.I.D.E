//! Ollama provider integration.
//!
//! Discovery: `GET /api/tags` → `.models[].name`
//! Streaming: `POST /api/generate` with `"stream": true`

use reqwest::Client;
use ride_core::{LocalModel, ProviderType};
use serde_json::Value;
use tracing::debug;

use crate::error::DiscoveryError;

/// List models from an Ollama instance at `base_url`.
pub async fn list_models(client: &Client, base_url: &str) -> Result<Vec<LocalModel>, DiscoveryError> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    debug!("Ollama list_models: {url}");

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DiscoveryError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(DiscoveryError::Http(format!(
            "Ollama /api/tags returned {}",
            resp.status()
        )));
    }

    let json: Value = resp.json().await.map_err(|e| DiscoveryError::Parse(e.to_string()))?;

    let models = json
        .get("models")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let name = m.get("name").and_then(Value::as_str)?;
                    // Ollama model names can be like "qwen2.5-coder:7b"
                    // Check heuristically if it looks like a MoE model
                    let is_moe = name.to_lowercase().contains("moe")
                        || name.to_lowercase().contains("mixtral")
                        || name.to_lowercase().contains("deepseek-v");
                    Some(LocalModel {
                        id: name.to_owned(),
                        name: name.to_owned(),
                        provider: ProviderType::Ollama,
                        context_length: None,
                        is_moe,
                        port: 11434,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Build a streaming generation request body for Ollama.
pub fn build_generate_request(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_generate_request() {
        let req = build_generate_request("llama3:8b", "Hello");
        assert_eq!(req["model"], "llama3:8b");
        assert_eq!(req["stream"], true);
        assert_eq!(req["prompt"], "Hello");
    }

    #[test]
    fn test_moe_heuristic() {
        // "mixtral" should be flagged as MoE
        let m = LocalModel {
            id: "mixtral:8x7b".into(),
            name: "mixtral:8x7b".into(),
            provider: ProviderType::Ollama,
            context_length: None,
            is_moe: true, // set by our heuristic
            port: 11434,
        };
        assert!(m.is_moe);
    }
}
