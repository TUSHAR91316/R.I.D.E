use ride_core::model::{ModelStatus, ProviderType};
use ride_core::{LocalModel, ModelStatusEvent};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::error::DiscoveryError;
use crate::providers::{ollama, openai_compat};

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

    pub fn port(&self) -> u16 {
        self.base_url
            .rsplit_once(':')
            .and_then(|(_, p)| p.trim_end_matches('/').parse::<u16>().ok())
            .unwrap_or_else(|| self.provider.default_port())
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

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn check_endpoint(&self, endpoint: &ProbeEndpoint) -> ModelStatus {
        let url = endpoint.full_probe_url();
        debug!("Probing model endpoint at {}", url);
        let start = std::time::Instant::now();
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                // Reject responses that are HTML web pages (e.g. dev servers, router pages)
                let is_html = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|h| h.to_str().ok())
                    .map(|ct| ct.contains("text/html"))
                    .unwrap_or(false);

                if is_html {
                    ModelStatus {
                        provider: endpoint.provider.clone(),
                        endpoint: endpoint.base_url.clone(),
                        is_online: false,
                        active_model: None,
                        latency_ms: None,
                    }
                } else {
                    let latency = start.elapsed().as_millis() as u64;
                    ModelStatus {
                        provider: endpoint.provider.clone(),
                        endpoint: endpoint.base_url.clone(),
                        is_online: true,
                        active_model: None,
                        latency_ms: Some(latency),
                    }
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

    /// Discover normalized `LocalModel` records for a single endpoint.
    pub async fn discover_models(&self, endpoint: &ProbeEndpoint) -> Result<Vec<LocalModel>, DiscoveryError> {
        let port = endpoint.port();
        match endpoint.provider {
            ProviderType::Ollama => {
                ollama::list_models(&self.client, &endpoint.base_url).await
            }
            ProviderType::LlamaCpp => {
                openai_compat::list_models_with_llama_fallback(
                    &self.client,
                    &endpoint.base_url,
                    endpoint.provider.clone(),
                    port,
                ).await
            }
            _ => {
                openai_compat::list_models(
                    &self.client,
                    &endpoint.base_url,
                    endpoint.provider.clone(),
                    port,
                ).await
            }
        }
    }

    /// Discover models across all configured endpoints concurrently.
    /// Returns a flat list of all discovered online `LocalModel` records.
    pub async fn discover_all_models(&self) -> Vec<LocalModel> {
        let futures = self.endpoints.iter().map(|ep| async move {
            self.discover_models(ep).await.unwrap_or_default()
        });
        futures_util::future::join_all(futures)
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    /// Run a background polling loop that periodically scans endpoints and broadcasts
    /// `Connected` and `Disconnected` events to `event_tx`.
    ///
    /// Probes endpoints concurrently and deduplicates events so subscribers are
    /// not spammed with duplicate `Connected` events on every tick.
    pub async fn run_discovery_loop(
        &self,
        event_tx: tokio::sync::broadcast::Sender<ModelStatusEvent>,
        cancel: CancellationToken,
        poll_interval: Duration,
    ) {
        use std::collections::HashMap;
        // Keyed by (ProviderType, endpoint base_url)
        let mut online_endpoints: HashMap<(ProviderType, String), Vec<LocalModel>> = HashMap::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("Model discovery loop cancelled");
                    break;
                }
                _ = tokio::time::sleep(poll_interval) => {
                    // Probe all endpoints concurrently rather than sequentially
                    let probe_futs = self.endpoints.iter().map(|ep| async move {
                        let res = self.discover_models(ep).await;
                        (ep.clone(), res)
                    });
                    let results = futures_util::future::join_all(probe_futs).await;

                    for (endpoint, models_res) in results {
                        let key = (endpoint.provider.clone(), endpoint.base_url.clone());
                        let prev_models = online_endpoints.get(&key);

                        match models_res {
                            Ok(models) => {
                                // Only broadcast Connected if newly online or if model list has changed
                                if prev_models != Some(&models) {
                                    online_endpoints.insert(key, models.clone());
                                    let _ = event_tx.send(ModelStatusEvent::Connected {
                                        provider: endpoint.provider.clone(),
                                        models,
                                    });
                                }
                            }
                            Err(_) => {
                                // If this endpoint was previously online, remove it and broadcast Disconnected
                                if online_endpoints.remove(&key).is_some() {
                                    // Only broadcast Disconnected if no other endpoint for this provider is still online
                                    let provider_still_online = online_endpoints.keys().any(|(p, _)| p == &endpoint.provider);
                                    if !provider_still_online {
                                        let _ = event_tx.send(ModelStatusEvent::Disconnected {
                                            provider: endpoint.provider.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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

    #[test]
    fn test_probe_endpoint_port_extraction() {
        let ep = ProbeEndpoint::new(ProviderType::Ollama, "http://127.0.0.1:11434", "/api/tags");
        assert_eq!(ep.port(), 11434);

        let ep2 = ProbeEndpoint::new(ProviderType::RabbitMoe, "http://localhost:5000/", "/v1/models");
        assert_eq!(ep2.port(), 5000);

        let ep3 = ProbeEndpoint::new(ProviderType::LmStudio, "http://192.168.1.50", "/v1/models");
        assert_eq!(ep3.port(), 1234); // fallback to ProviderType default
    }

    #[tokio::test]
    async fn test_discover_models_offline_endpoint_fails_cleanly() {
        let detector = ModelDetector::new(vec![ProbeEndpoint::new(
            ProviderType::Ollama,
            "http://127.0.0.1:65534",
            "/api/tags",
        )]);
        let ep = &detector.endpoints()[0];
        let result = detector.discover_models(ep).await;
        assert!(result.is_err());

        let all = detector.discover_all_models().await;
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_discovery_loop_cancels_cleanly() {
        let detector = ModelDetector::new(vec![]);
        let (tx, _rx) = ride_core::new_event_bus();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move {
            detector.run_discovery_loop(tx, cancel_clone, Duration::from_millis(50)).await;
        });

        cancel.cancel();
        tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("should terminate on cancellation")
            .expect("task should not panic");
    }
}
