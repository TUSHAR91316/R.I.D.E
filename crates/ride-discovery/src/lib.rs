pub mod detector;
pub mod error;
pub mod providers;
pub mod sse;

pub use detector::{ModelDetector, ProbeEndpoint};
pub use error::DiscoveryError;
pub use sse::{
    parse_ollama_chunk, parse_openai_chunk, stream_sse, stream_sse_to_event_bus, SseFormat,
    SseToken,
};
