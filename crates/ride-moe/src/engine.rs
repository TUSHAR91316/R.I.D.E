//! Legacy MoE engine manager (Phase 1 stubs, kept for backward compatibility).

use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoeEngineConfig {
    pub model_path: PathBuf,
    pub port: u16,
    pub nvme_cache_gb: f32,
    pub max_active_experts: usize,
    pub use_gpu_attention: bool,
    pub num_threads: usize,
}

impl Default for MoeEngineConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/moe-weights"),
            port: 8080,
            nvme_cache_gb: 16.0,
            max_active_experts: 8,
            use_gpu_attention: true,
            num_threads: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoeEngineStatus {
    pub is_running: bool,
    pub active_model_name: Option<String>,
    pub streamed_experts_count: u64,
    pub disk_read_mbs: f32,
    pub ram_resident_mb: usize,
}

impl Default for MoeEngineStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            active_model_name: None,
            streamed_experts_count: 0,
            disk_read_mbs: 0.0,
            ram_resident_mb: 0,
        }
    }
}

pub struct MoeEngineManager {
    config: MoeEngineConfig,
    status: MoeEngineStatus,
}

impl MoeEngineManager {
    pub fn new(config: MoeEngineConfig) -> Self {
        Self {
            config,
            status: MoeEngineStatus::default(),
        }
    }

    pub fn config(&self) -> &MoeEngineConfig {
        &self.config
    }

    pub fn status(&self) -> &MoeEngineStatus {
        &self.status
    }

    pub fn update_status(&mut self, status: MoeEngineStatus) {
        self.status = status;
    }

    pub fn validate_config(&self) -> Result<()> {
        if self.config.port == 0 {
            return Err(RideError::Moe("MoE port cannot be 0".into()));
        }
        Ok(())
    }

    pub async fn check_health(&self, client: &reqwest::Client) -> bool {
        let url = format!("http://127.0.0.1:{}/health", self.config.port);
        debug!("Checking health of MoE engine at {}", url);
        client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_moe_config() {
        let config = MoeEngineConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_active_experts, 8);
        assert!(config.use_gpu_attention);

        let manager = MoeEngineManager::new(config);
        assert!(manager.validate_config().is_ok());
    }

    #[test]
    fn test_invalid_moe_port() {
        let mut config = MoeEngineConfig::default();
        config.port = 0;
        let manager = MoeEngineManager::new(config);
        assert!(manager.validate_config().is_err());
    }
}
