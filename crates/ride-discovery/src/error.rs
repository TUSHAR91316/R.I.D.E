use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Stream cancelled")]
    Cancelled,
}
