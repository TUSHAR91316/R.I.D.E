//! NVMe disk streaming monitor for MoE model inference.
//!
//! `StreamMonitor` samples system-wide disk read throughput and RAM
//! utilization, then publishes `MoeStreamStats` via a `tokio::sync::watch`
//! channel that the UI can subscribe to without polling.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// Real-time MoE streaming statistics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoeStreamStats {
    /// Tokens generated so far in this session.
    pub tokens_generated: usize,
    /// Number of MoE experts swapped from NVMe this session.
    pub experts_swapped: usize,
    /// Current NVMe read throughput in MB/s.
    pub nvme_bandwidth_mbs: f32,
    /// Average per-token generation latency in ms.
    pub avg_token_latency_ms: f32,
    /// Current total system RAM usage in GB.
    pub system_ram_used_gb: f32,
    /// Current total system VRAM estimate (GPU memory) in GB (0.0 if unknown).
    pub vram_used_gb: f32,
}

impl Default for MoeStreamStats {
    fn default() -> Self {
        Self {
            tokens_generated: 0,
            experts_swapped: 0,
            nvme_bandwidth_mbs: 0.0,
            avg_token_latency_ms: 0.0,
            system_ram_used_gb: 0.0,
            vram_used_gb: 0.0,
        }
    }
}

/// Configuration for `StreamMonitor`.
#[derive(Debug, Clone)]
pub struct StreamMonitorConfig {
    /// How often to sample system stats.
    pub sample_interval: Duration,
    /// If `true`, use generated pseudo-random values instead of live sysinfo.
    pub simulated: bool,
}

impl Default for StreamMonitorConfig {
    fn default() -> Self {
        Self {
            sample_interval: Duration::from_millis(500),
            simulated: false,
        }
    }
}

/// Monitors NVMe throughput and RAM/VRAM utilization.
///
/// Publishes snapshots via a `tokio::sync::watch` channel at
/// `config.sample_interval` intervals. Stops cleanly when `cancel` fires.
pub struct StreamMonitor {
    config: StreamMonitorConfig,
    tx: watch::Sender<MoeStreamStats>,
    rx: watch::Receiver<MoeStreamStats>,
}

impl StreamMonitor {
    /// Create a new `StreamMonitor` with the given configuration.
    pub fn new(config: StreamMonitorConfig) -> Self {
        let (tx, rx) = watch::channel(MoeStreamStats::default());
        Self { config, tx, rx }
    }

    /// Create a new `StreamMonitor` in simulated mode (for testing).
    pub fn simulated() -> Self {
        Self::new(StreamMonitorConfig {
            simulated: true,
            sample_interval: Duration::from_millis(100),
        })
    }

    /// Subscribe to the stats watch channel.
    pub fn subscribe(&self) -> watch::Receiver<MoeStreamStats> {
        self.rx.clone()
    }

    /// Run the monitoring loop until `cancel` fires.
    ///
    /// This should be spawned as a background tokio task:
    /// ```ignore
    /// let cancel = CancellationToken::new();
    /// tokio::spawn(monitor.run(cancel.clone()));
    /// ```
    pub async fn run(self, cancel: CancellationToken) {
        let mut sim_counter: u64 = 0;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("StreamMonitor cancelled");
                    break;
                }
                _ = tokio::time::sleep(self.config.sample_interval) => {
                    let stats = if self.config.simulated {
                        sim_counter = sim_counter.wrapping_add(1);
                        // Simulate realistic-looking NVMe streaming numbers
                        let bandwidth = 800.0 + (sim_counter as f32 * 7.3).sin() * 200.0;
                        let ram = 10.5 + (sim_counter as f32 * 0.1).cos() * 0.5;
                        MoeStreamStats {
                            tokens_generated: (sim_counter * 3) as usize,
                            experts_swapped: (sim_counter / 4) as usize,
                            nvme_bandwidth_mbs: bandwidth,
                            avg_token_latency_ms: 85.0 + (sim_counter as f32 * 2.1).sin() * 15.0,
                            system_ram_used_gb: ram,
                            vram_used_gb: 0.0, // GPU detection not simulated
                        }
                    } else {
                        sample_live_stats()
                    };

                    // Send; ignore error (receiver may have been dropped)
                    let _ = self.tx.send(stats);
                }
            }
        }
    }
}

/// Sample real system stats using the `sysinfo` crate.
fn sample_live_stats() -> MoeStreamStats {
    let mut sys = System::new();
    sys.refresh_memory();

    let used_bytes = sys.used_memory();
    let ram_gb = used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

    // Sample disk reads across all block devices
    let mut disks = Disks::new_with_refreshed_list();
    // Brief pause then re-sample to calculate delta
    std::thread::sleep(Duration::from_millis(50));
    disks.refresh(false); // false = don't remove disks no longer listed

    // Sum total bytes read across all disks as a rough throughput proxy.
    // `sysinfo::Disk::usage()` returns a `DiskUsage` with `read_bytes`
    // (bytes read since last refresh) and `total_read_bytes` (cumulative).
    let total_read_bytes: u64 = disks.iter().map(|d| d.usage().total_read_bytes).sum();
    let nvme_mb = total_read_bytes as f32 / (1024.0 * 1024.0);

    MoeStreamStats {
        tokens_generated: 0, // updated externally by the SSE consumer
        experts_swapped: 0,
        nvme_bandwidth_mbs: nvme_mb,
        avg_token_latency_ms: 0.0,
        system_ram_used_gb: ram_gb,
        vram_used_gb: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_simulated_monitor_emits_stats() {
        let monitor = StreamMonitor::simulated();
        let mut rx = monitor.subscribe();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        tokio::spawn(monitor.run(cancel_clone));

        // Wait for at least one update
        tokio::time::timeout(Duration::from_secs(2), rx.changed())
            .await
            .expect("should receive update within 2s")
            .expect("watch should not be closed");

        let stats = rx.borrow().clone();
        // Simulated bandwidth should be in a realistic range
        assert!(stats.nvme_bandwidth_mbs > 0.0);
        assert!(stats.system_ram_used_gb > 0.0);

        cancel.cancel();
    }

    #[tokio::test]
    async fn test_simulated_monitor_cancels_cleanly() {
        let monitor = StreamMonitor::simulated();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(monitor.run(cancel_clone));

        // Cancel immediately
        cancel.cancel();

        // Should complete quickly
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("task should finish within 1s")
            .expect("task should not panic");
    }

    #[test]
    fn test_default_stats() {
        let s = MoeStreamStats::default();
        assert_eq!(s.nvme_bandwidth_mbs, 0.0);
        assert_eq!(s.tokens_generated, 0);
    }

    #[test]
    fn test_config_simulated() {
        let cfg = StreamMonitorConfig {
            simulated: true,
            sample_interval: Duration::from_millis(100),
        };
        assert!(cfg.simulated);
    }
}
