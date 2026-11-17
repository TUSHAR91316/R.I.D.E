use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    ColibriMoe,
    RabbitMoe,
    Ollama,
    LmStudio,
    LlamaCpp,
    Vllm,
    Custom(String),
}

impl ProviderType {
    pub fn default_port(&self) -> u16 {
        match self {
            Self::ColibriMoe => 8080,
            Self::RabbitMoe => 5000,
            Self::Ollama => 11434,
            Self::LmStudio => 1234,
            Self::LlamaCpp => 8080,
            Self::Vllm => 8000,
            Self::Custom(_) => 8080,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ColibriMoe => "Colibrì (MoE)",
            Self::RabbitMoe => "Rabbit (MoE Rust)",
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::LlamaCpp => "llama.cpp",
            Self::Vllm => "vLLM",
            Self::Custom(_) => "Custom Provider",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredModel {
    pub id: String,
    pub name: String,
    pub provider: ProviderType,
    pub endpoint: String,
    pub is_moe: bool,
    pub context_length: Option<usize>,
    pub parameter_count: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub provider: ProviderType,
    pub endpoint: String,
    pub is_online: bool,
    pub active_model: Option<DiscoveredModel>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenChunk {
    pub delta: String,
    pub is_final: bool,
    pub tokens_per_second: Option<f32>,
}
