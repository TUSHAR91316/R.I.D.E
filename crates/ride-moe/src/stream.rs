//! NVMe disk streaming monitor for MoE model inference.
//!
//! `StreamMonitor` samples system-wide disk read throughput and RAM
//! utilization, then publishes `MoeStreamStats` via a `tokio::sync::watch`
//! channel that the UI can subscribe to without polling.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use ride_core::ModelStatusEvent;

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
/// Optionally broadcasts `ModelStatusEvent::MoeStats` to the global event bus.
pub struct StreamMonitor {
    config: StreamMonitorConfig,
    tx: watch::Sender<MoeStreamStats>,
    rx: watch::Receiver<MoeStreamStats>,
    event_tx: Option<broadcast::Sender<ModelStatusEvent>>,
    tokens_counter: Arc<AtomicUsize>,
    experts_counter: Arc<AtomicUsize>,
}

impl StreamMonitor {
    /// Create a new `StreamMonitor` with the given configuration.
    pub fn new(config: StreamMonitorConfig) -> Self {
        let (tx, rx) = watch::channel(MoeStreamStats::default());
        Self {
            config,
            tx,
            rx,
            event_tx: None,
            tokens_counter: Arc::new(AtomicUsize::new(0)),
            experts_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a new `StreamMonitor` in simulated mode (for testing).
    pub fn simulated() -> Self {
        Self::new(StreamMonitorConfig {
            simulated: true,
            sample_interval: Duration::from_millis(100),
        })
    }

    /// Record newly generated tokens.
    pub fn record_tokens(&self, count: usize) {
        self.tokens_counter.fetch_add(count, Ordering::Relaxed);
    }

    /// Record newly swapped MoE experts.
    pub fn record_expert_swapped(&self, count: usize) {
        self.experts_counter.fetch_add(count, Ordering::Relaxed);
    }

    /// Attach the global `ModelStatusEvent` bus sender to publish `MoeStats` snapshots.
    pub fn with_event_bus(mut self, tx: broadcast::Sender<ModelStatusEvent>) -> Self {
        self.event_tx = Some(tx);
        self
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

        // Persistent system state across iterations to avoid reallocations
        let mut live_sys = if !self.config.simulated {
            let mut s = System::new();
            s.refresh_memory();
            Some(s)
        } else {
            None
        };

        let mut live_disks = if !self.config.simulated {
            Some(Disks::new_with_refreshed_list())
        } else {
            None
        };

        let mut last_sample_time = std::time::Instant::now();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("StreamMonitor cancelled");
                    break;
                }
                _ = tokio::time::sleep(self.config.sample_interval) => {
                    let elapsed = last_sample_time.elapsed().as_secs_f32().max(0.001);
                    last_sample_time = std::time::Instant::now();

                    let stats = if self.config.simulated {
                        sim_counter = sim_counter.wrapping_add(1);
                        // Simulate realistic-looking NVMe streaming numbers
                        let bandwidth = 800.0 + (sim_counter as f32 * 7.3).sin() * 200.0;
                        let ram = 10.5 + (sim_counter as f32 * 0.1).cos() * 0.5;
                        let custom_tokens = self.tokens_counter.load(Ordering::Relaxed);
                        let custom_experts = self.experts_counter.load(Ordering::Relaxed);
                        MoeStreamStats {
                            tokens_generated: if custom_tokens > 0 { custom_tokens } else { (sim_counter * 3) as usize },
                            experts_swapped: if custom_experts > 0 { custom_experts } else { (sim_counter / 4) as usize },
                            nvme_bandwidth_mbs: bandwidth,
                            avg_token_latency_ms: 85.0 + (sim_counter as f32 * 2.1).sin() * 15.0,
                            system_ram_used_gb: ram,
                            vram_used_gb: 0.0,
                        }
                    } else {
                        let sys = live_sys.as_mut().expect("live sys exists");
                        let disks = live_disks.as_mut().expect("live disks exist");

                        sys.refresh_memory();
                        disks.refresh(false);

                        // read_bytes is bytes read since last refresh call
                        let read_bytes: u64 = disks.iter().map(|d| d.usage().read_bytes).sum();
                        let nvme_mbs = (read_bytes as f32 / (1024.0 * 1024.0)) / elapsed;
                        let ram_gb = sys.used_memory() as f32 / (1024.0 * 1024.0 * 1024.0);

                        MoeStreamStats {
                            tokens_generated: self.tokens_counter.load(Ordering::Relaxed),
                            experts_swapped: self.experts_counter.load(Ordering::Relaxed),
                            nvme_bandwidth_mbs: nvme_mbs,
                            avg_token_latency_ms: 0.0,
                            system_ram_used_gb: ram_gb,
                            vram_used_gb: 0.0,
                        }
                    };

                    // Send on watch channel; ignore error (receiver may have been dropped)
                    let _ = self.tx.send(stats.clone());

                    // Send on global event bus if configured
                    if let Some(ref bus) = self.event_tx {
                        let _ = bus.send(ModelStatusEvent::MoeStats {
                            throughput_mb_s: stats.nvme_bandwidth_mbs,
                            active_experts: stats.experts_swapped as u32,
                            ram_gb: stats.system_ram_used_gb,
                            vram_gb: stats.vram_used_gb,
                        });
                    }
                }
            }
        }
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

    #[tokio::test]
    async fn test_stream_monitor_publishes_to_event_bus() {
        let (bus_tx, mut bus_rx) = ride_core::new_event_bus();
        let monitor = StreamMonitor::simulated().with_event_bus(bus_tx);
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        tokio::spawn(monitor.run(cancel_clone));

        let event = tokio::time::timeout(Duration::from_secs(2), bus_rx.recv())
            .await
            .expect("should receive event within 2s")
            .expect("event bus receive ok");

        match event {
            ModelStatusEvent::MoeStats { throughput_mb_s, active_experts: _, ram_gb, vram_gb: _ } => {
                assert!(throughput_mb_s > 0.0);
                assert!(ram_gb > 0.0);
            }
            _ => panic!("expected MoeStats event"),
        }

        cancel.cancel();
    }

    #[tokio::test]
    async fn test_stream_monitor_record_tokens_and_experts() {
        let monitor = StreamMonitor::simulated();
        monitor.record_tokens(42);
        monitor.record_expert_swapped(8);

        let mut rx = monitor.subscribe();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        tokio::spawn(monitor.run(cancel_clone));

        tokio::time::timeout(Duration::from_secs(2), rx.changed())
            .await
            .expect("should receive update")
            .expect("channel ok");

        let stats = rx.borrow().clone();
        assert!(stats.tokens_generated >= 42);
        assert!(stats.experts_swapped >= 8);

        cancel.cancel();
    }
}
