use thiserror::Error;

#[derive(Error, Debug)]
pub enum RideError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Model discovery error: {0}")]
    Discovery(String),

    #[error("MoE engine error: {0}")]
    Moe(String),

    #[error("Skill error: {0}")]
    Skill(String),

    #[error("Agent tool error: {0}")]
    Agent(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, RideError>;
