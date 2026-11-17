use ride_core::model::{ModelStatus, ProviderType};
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ProbeEndpoint {
    pub provider: ProviderType,
    pub base_url: String,
    pub probe_path: &'static str,
}

impl ProbeEndpoint {
    pub fn new(provider: ProviderType, base_url: impl Into<String>, probe_path: &'static str) -> Self {
        Self {
            provider,
            base_url: base_url.into(),
            probe_path,
        }
    }

    pub fn full_probe_url(&self) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), self.probe_path)
    }
}

pub struct ModelDetector {
    endpoints: Vec<ProbeEndpoint>,
    client: reqwest::Client,
}

impl Default for ModelDetector {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .connect_timeout(Duration::from_millis(500))
            .build()
            .unwrap_or_default();

        let endpoints = vec![
            ProbeEndpoint::new(ProviderType::ColibriMoe, "http://127.0.0.1:8080", "/v1/models"),
            ProbeEndpoint::new(ProviderType::RabbitMoe, "http://127.0.0.1:5000", "/v1/models"),
            ProbeEndpoint::new(ProviderType::Ollama, "http://127.0.0.1:11434", "/api/tags"),
            ProbeEndpoint::new(ProviderType::LmStudio, "http://127.0.0.1:1234", "/v1/models"),
            ProbeEndpoint::new(ProviderType::LlamaCpp, "http://127.0.0.1:8080", "/props"),
            ProbeEndpoint::new(ProviderType::Vllm, "http://127.0.0.1:8000", "/v1/models"),
        ];

        Self { endpoints, client }
    }
}

impl ModelDetector {
    pub fn new(endpoints: Vec<ProbeEndpoint>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .connect_timeout(Duration::from_millis(500))
            .build()
            .unwrap_or_default();
        Self { endpoints, client }
    }

    pub fn endpoints(&self) -> &[ProbeEndpoint] {
        &self.endpoints
    }

    pub async fn check_endpoint(&self, endpoint: &ProbeEndpoint) -> ModelStatus {
        let url = endpoint.full_probe_url();
        debug!("Probing model endpoint at {}", url);
        let start = std::time::Instant::now();
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as u64;
                ModelStatus {
                    provider: endpoint.provider.clone(),
                    endpoint: endpoint.base_url.clone(),
                    is_online: true,
                    active_model: None,
                    latency_ms: Some(latency),
                }
            }
            _ => ModelStatus {
                provider: endpoint.provider.clone(),
                endpoint: endpoint.base_url.clone(),
                is_online: false,
                active_model: None,
                latency_ms: None,
            },
        }
    }

    pub async fn scan_all(&self) -> Vec<ModelStatus> {
        let futures = self.endpoints.iter().map(|ep| self.check_endpoint(ep));
        futures_util::future::join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_defaults() {
        let detector = ModelDetector::default();
        assert_eq!(detector.endpoints().len(), 6);
        let ep = &detector.endpoints()[0];
        assert_eq!(ep.provider, ProviderType::ColibriMoe);
        assert_eq!(ep.full_probe_url(), "http://127.0.0.1:8080/v1/models");
    }

    #[tokio::test]
    async fn test_offline_endpoint_check() {
        let detector = ModelDetector::new(vec![ProbeEndpoint::new(
            ProviderType::Ollama,
            "http://127.0.0.1:65534",
            "/api/tags",
        )]);
        let results = detector.scan_all().await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_online);
    }
}
