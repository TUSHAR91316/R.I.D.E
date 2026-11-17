use serde::{Deserialize, Serialize};
use crate::model::ProviderType;

/// Normalized model descriptor shared across all providers.
/// Fields map to the task spec: id, provider, context_length, is_moe, port.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalModel {
    /// Unique model identifier (e.g. "qwen2.5-coder:7b", "gpt-4o-mini")
    pub id: String,
    /// Human-readable name, may be the same as id
    pub name: String,
    /// Which provider serves this model
    pub provider: ProviderType,
    /// Model context window in tokens, if known
    pub context_length: Option<usize>,
    /// True for Mixture-of-Experts models (Colibrì / Rabbit)
    pub is_moe: bool,
    /// Port the provider is listening on
    pub port: u16,
}

/// Real-time event bus events broadcast across all R.I.D.E crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelStatusEvent {
    /// A provider came online and reported its model list.
    Connected {
        provider: ProviderType,
        models: Vec<LocalModel>,
    },
    /// A provider went offline.
    Disconnected {
        provider: ProviderType,
    },
    /// A single streaming token was generated.
    TokenGenerated {
        token: String,
    },
    /// The generation stream finished cleanly.
    GenerationComplete,
    /// Live MoE engine stats snapshot.
    MoeStats {
        /// NVMe read throughput in MB/s
        throughput_mb_s: f32,
        /// Number of currently active MoE experts in RAM
        active_experts: u32,
        /// Total resident RAM used by the MoE process in GB
        ram_gb: f32,
    },
}

/// Capacity for the global broadcast channel.
/// 512 means up to 512 unread events before the sender starts dropping.
pub const EVENT_BUS_CAPACITY: usize = 512;

/// Create a new global event bus (sender + first receiver).
pub fn new_event_bus() -> (
    tokio::sync::broadcast::Sender<ModelStatusEvent>,
    tokio::sync::broadcast::Receiver<ModelStatusEvent>,
) {
    tokio::sync::broadcast::channel(EVENT_BUS_CAPACITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_model_fields() {
        let m = LocalModel {
            id: "llama3:8b".into(),
            name: "LLaMA 3 8B".into(),
            provider: ProviderType::Ollama,
            context_length: Some(8192),
            is_moe: false,
            port: 11434,
        };
        assert_eq!(m.id, "llama3:8b");
        assert!(!m.is_moe);
        assert_eq!(m.port, 11434);
    }

    #[test]
    fn test_model_status_event_connected_serialize() {
        let ev = ModelStatusEvent::Connected {
            provider: ProviderType::Ollama,
            models: vec![LocalModel {
                id: "phi3:mini".into(),
                name: "Phi-3 Mini".into(),
                provider: ProviderType::Ollama,
                context_length: Some(4096),
                is_moe: false,
                port: 11434,
            }],
        };
        let json = serde_json::to_string(&ev).expect("serialize ok");
        assert!(json.contains("phi3:mini"));
    }

    #[tokio::test]
    async fn test_event_bus_roundtrip() {
        let (tx, mut rx) = new_event_bus();
        tx.send(ModelStatusEvent::GenerationComplete).unwrap();
        let ev = rx.recv().await.unwrap();
        matches!(ev, ModelStatusEvent::GenerationComplete);
    }

    #[tokio::test]
    async fn test_event_bus_moe_stats() {
        let (tx, mut rx) = new_event_bus();
        tx.send(ModelStatusEvent::MoeStats {
            throughput_mb_s: 1024.5,
            active_experts: 8,
            ram_gb: 12.3,
        })
        .unwrap();
        let ev = rx.recv().await.unwrap();
        if let ModelStatusEvent::MoeStats { throughput_mb_s, active_experts, ram_gb } = ev {
            assert!((throughput_mb_s - 1024.5).abs() < 0.01);
            assert_eq!(active_experts, 8);
            assert!((ram_gb - 12.3).abs() < 0.01);
        } else {
            panic!("wrong event variant");
        }
    }
}
