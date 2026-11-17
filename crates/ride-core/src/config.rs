use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiLayout {
    VsCode,
    ClaudeCode,
    Codex,
}

impl Default for UiLayout {
    fn default() -> Self {
        Self::VsCode
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub max_editor_ram_mb: usize,
    pub token_stream_buffer_kb: usize,
    pub throttle_indexing_on_generation: bool,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_editor_ram_mb: 60,
            token_stream_buffer_kb: 64,
            throttle_indexing_on_generation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RideConfig {
    pub active_layout: UiLayout,
    pub skills_dir: PathBuf,
    pub memory_limits: MemoryLimits,
    pub preferred_provider: Option<String>,
}

impl Default for RideConfig {
    fn default() -> Self {
        let home = dirs_fallback();
        let config = Self {
            active_layout: UiLayout::default(),
            skills_dir: home.join(".ride").join("skills"),
            memory_limits: MemoryLimits::default(),
            preferred_provider: None,
        };
        tracing::debug!("Loaded default RideConfig with layout {:?}", config.active_layout);
        config
    }
}

fn dirs_fallback() -> PathBuf {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        if !userprofile.trim().is_empty() {
            return PathBuf::from(userprofile);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RideConfig::default();
        assert_eq!(config.active_layout, UiLayout::VsCode);
        assert_eq!(config.memory_limits.max_editor_ram_mb, 60);
        assert!(config.memory_limits.throttle_indexing_on_generation);
    }

    #[test]
    fn test_layout_serialization() {
        let serialized = serde_json::to_string(&UiLayout::ClaudeCode).unwrap();
        assert_eq!(serialized, "\"claude_code\"");
        let deserialized: UiLayout = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, UiLayout::ClaudeCode);
    }
}
