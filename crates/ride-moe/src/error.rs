use thiserror::Error;

#[derive(Error, Debug)]
pub enum MoeError {
    #[error("Process already running")]
    AlreadyRunning,

    #[error("Failed to spawn process: {0}")]
    Spawn(String),

    #[error("Shutdown failed: {0}")]
    Shutdown(String),

    #[error("Health check failed")]
    HealthCheckFailed,
}
