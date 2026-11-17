use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoeStreamStats {
    pub tokens_generated: usize,
    pub experts_swapped: usize,
    pub nvme_bandwidth_mbs: f32,
    pub avg_token_latency_ms: f32,
}

impl Default for MoeStreamStats {
    fn default() -> Self {
        Self {
            tokens_generated: 0,
            experts_swapped: 0,
            nvme_bandwidth_mbs: 0.0,
            avg_token_latency_ms: 0.0,
        }
    }
}
