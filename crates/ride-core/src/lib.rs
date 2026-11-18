pub mod config;
pub mod error;
pub mod events;
pub mod model;

pub use config::{MemoryLimits, RideConfig, UiLayout};
pub use error::{Result, RideError};
pub use events::{
    new_agent_event_bus, new_event_bus, AgentEvent, LocalModel, ModelStatusEvent,
    EVENT_BUS_CAPACITY,
};
pub use model::{DiscoveredModel, ModelStatus, ProviderType, TokenChunk};

pub const RIDE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RIDE_APP_NAME: &str = "R.I.D.E (Rust Intelligent Development Environment)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_and_name() {
        assert_eq!(RIDE_VERSION, "0.1.0");
        assert!(RIDE_APP_NAME.contains("R.I.D.E"));
    }

    #[test]
    fn test_provider_ports() {
        assert_eq!(ProviderType::Ollama.default_port(), 11434);
        assert_eq!(ProviderType::LmStudio.default_port(), 1234);
        assert_eq!(ProviderType::ColibriMoe.default_port(), 8080);
        assert_eq!(ProviderType::RabbitMoe.default_port(), 5000);
        assert_eq!(ProviderType::Vllm.default_port(), 8000);
    }
}
