//! Managed process launcher for Rabbit and Colibrì MoE engines.
//!
//! `MoeLauncher` spawns a Rabbit (`ferrumox/rabbit`) or Colibrì subprocess,
//! forwards stdout/stderr to the tracing log sink, tracks process health, and
//! supports graceful shutdown on Windows (`taskkill`) and Unix (`SIGTERM`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use ride_core::ModelStatusEvent;
use crate::error::MoeError;

/// Configuration for launching a Rabbit or Colibrì process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoeLauncherConfig {
    /// Path to the Rabbit or Colibrì binary (e.g. `C:\bin\rabbit.exe`).
    pub binary_path: PathBuf,
    /// Directory containing the MoE model weights.
    pub model_dir: PathBuf,
    /// Port the engine should listen on.
    pub port: u16,
    /// Extra command-line arguments passed verbatim to the engine.
    pub extra_args: Vec<String>,
    /// Which provider this engine represents. If None, inferred from binary name.
    pub provider: Option<ride_core::ProviderType>,
}

impl MoeLauncherConfig {
    /// Resolve the effective provider type (Colibrì or Rabbit).
    pub fn resolved_provider(&self) -> ride_core::ProviderType {
        if let Some(ref p) = self.provider {
            return p.clone();
        }
        let bin_name = self.binary_path.to_string_lossy().to_lowercase();
        if bin_name.contains("colibri") {
            ride_core::ProviderType::ColibriMoe
        } else {
            ride_core::ProviderType::RabbitMoe
        }
    }
}

impl Default for MoeLauncherConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("rabbit"),
            model_dir: PathBuf::from("models"),
            port: 5000,
            extra_args: vec![],
            provider: None,
        }
    }
}

/// Current lifecycle state of the managed process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    Stopped,
    Starting,
    Running { pid: u32 },
    Failed { reason: String },
}

/// Manages the lifecycle of a Rabbit or Colibrì subprocess.
pub struct MoeLauncher {
    config: MoeLauncherConfig,
    state: Arc<Mutex<ProcessState>>,
    child: Arc<Mutex<Option<Child>>>,
    event_tx: broadcast::Sender<ModelStatusEvent>,
}

impl MoeLauncher {
    /// Create a new launcher.
    ///
    /// `event_tx` is the global R.I.D.E event bus sender; the launcher
    /// broadcasts `Connected` / `Disconnected` events on state changes.
    pub fn new(
        config: MoeLauncherConfig,
        event_tx: broadcast::Sender<ModelStatusEvent>,
    ) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ProcessState::Stopped)),
            child: Arc::new(Mutex::new(None)),
            event_tx,
        }
    }

    /// Return the current process state.
    pub async fn state(&self) -> ProcessState {
        self.state.lock().await.clone()
    }

    /// Spawn the engine process and begin streaming its logs.
    ///
    /// Returns the OS PID on success. The process is tracked internally;
    /// call `shutdown()` to terminate it gracefully.
    pub async fn spawn(&self, cancel: CancellationToken) -> Result<u32, MoeError> {
        let mut state = self.state.lock().await;
        if matches!(*state, ProcessState::Running { .. } | ProcessState::Starting) {
            return Err(MoeError::AlreadyRunning);
        }
        *state = ProcessState::Starting;
        drop(state);

        let mut cmd = Command::new(&self.config.binary_path);
        cmd.arg("--model-dir")
            .arg(&self.config.model_dir)
            .arg("--port")
            .arg(self.config.port.to_string());
        for arg in &self.config.extra_args {
            cmd.arg(arg);
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let mut state = self.state.lock().await;
                *state = ProcessState::Failed { reason: e.to_string() };
                return Err(MoeError::Spawn(e.to_string()));
            }
        };
        let pid = child.id().unwrap_or(0);

        info!("MoE engine spawned: pid={pid} binary={:?}", self.config.binary_path);

        // Stream stdout
        if let Some(stdout) = child.stdout.take() {
            let cancel_clone = cancel.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                loop {
                    tokio::select! {
                        line = lines.next_line() => match line {
                            Ok(Some(l)) => info!("[moe-engine stdout] {l}"),
                            Ok(None) => break,
                            Err(e) => { warn!("stdout read error: {e}"); break; }
                        },
                        _ = cancel_clone.cancelled() => break,
                    }
                }
            });
        }

        // Stream stderr
        if let Some(stderr) = child.stderr.take() {
            let cancel_clone = cancel.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                loop {
                    tokio::select! {
                        line = lines.next_line() => match line {
                            Ok(Some(l)) => info!("[moe-engine stderr] {l}"),
                            Ok(None) => break,
                            Err(e) => { warn!("stderr read error: {e}"); break; }
                        },
                        _ = cancel_clone.cancelled() => break,
                    }
                }
            });
        }

        {
            let mut state = self.state.lock().await;
            *state = ProcessState::Running { pid };
        }
        {
            let mut guard = self.child.lock().await;
            *guard = Some(child);
        }

        // Spawn a watcher that updates state when the process exits or cancellation fires
        {
            let state_ref = Arc::clone(&self.state);
            let child_ref = Arc::clone(&self.child);
            let event_tx = self.event_tx.clone();
            let provider = self.config.resolved_provider();
            let cancel_watcher = cancel.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel_watcher.cancelled() => {
                            let mut child_guard = child_ref.lock().await;
                            if let Some(ref mut c) = *child_guard {
                                #[cfg(target_os = "windows")]
                                if let Some(pid) = c.id() {
                                    let _ = std::process::Command::new("taskkill")
                                        .args(["/F", "/PID", &pid.to_string()])
                                        .output();
                                }
                                #[cfg(not(target_os = "windows"))]
                                if let Some(pid) = c.id() {
                                    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                                }
                                let _ = tokio::time::timeout(Duration::from_secs(2), c.wait()).await;
                            }
                            *child_guard = None;
                            let mut s = state_ref.lock().await;
                            let was_running = matches!(*s, ProcessState::Running { .. } | ProcessState::Starting);
                            *s = ProcessState::Stopped;
                            drop(s);
                            if was_running {
                                let _ = event_tx.send(ModelStatusEvent::Disconnected {
                                    provider: provider.clone(),
                                });
                            }
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_millis(250)) => {
                            let mut child_guard = child_ref.lock().await;
                            if let Some(ref mut c) = *child_guard {
                                match c.try_wait() {
                                    Ok(Some(status)) => {
                                        let is_ok = status.success();
                                        let mut s = state_ref.lock().await;
                                        if is_ok {
                                            info!("MoE engine process exited cleanly (status {status})");
                                            *s = ProcessState::Stopped;
                                        } else {
                                            let reason = format!("exited with status {status}");
                                            warn!("MoE engine process died: {reason}");
                                            *s = ProcessState::Failed { reason };
                                        }
                                        drop(s);
                                        *child_guard = None;
                                        let _ = event_tx.send(ModelStatusEvent::Disconnected {
                                            provider: provider.clone(),
                                        });
                                        break;
                                    }
                                    Ok(None) => {} // still running
                                    Err(e) => {
                                        error!("try_wait error: {e}");
                                        break;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            });
        }

        Ok(pid)
    }

    /// Ping the engine's health endpoint. Returns `true` if the engine
    /// responds with HTTP 200.
    pub async fn ping_health(&self, client: &reqwest::Client) -> bool {
        let url = format!("http://127.0.0.1:{}/health", self.config.port);
        client
            .get(&url)
            .timeout(Duration::from_millis(500))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Wait for the engine to respond to `/health`, then broadcast `ModelStatusEvent::Connected`.
    pub async fn wait_until_ready(&self, timeout: Duration, client: &reqwest::Client) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if self.ping_health(client).await {
                let model_name = self
                    .config
                    .model_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("moe-model")
                    .to_string();
                let provider = self.config.resolved_provider();
                let models = vec![ride_core::LocalModel {
                    id: model_name.clone(),
                    name: model_name,
                    provider: provider.clone(),
                    context_length: None,
                    is_moe: true,
                    port: self.config.port,
                }];
                let _ = self.event_tx.send(ModelStatusEvent::Connected {
                    provider,
                    models,
                });
                return true;
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        false
    }

    /// Gracefully shut down the managed engine process.
    ///
    /// On Windows: sends `taskkill /F /PID <pid>`.
    /// On Unix: sends `SIGTERM`.
    pub async fn shutdown(&self) -> Result<(), MoeError> {
        let mut state = self.state.lock().await;
        if *state == ProcessState::Stopped {
            // Already stopped, nothing to do
            return Ok(());
        }
        let was_active = matches!(*state, ProcessState::Running { .. } | ProcessState::Starting);
        *state = ProcessState::Stopped;
        drop(state);

        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            #[cfg(target_os = "windows")]
            {
                if let Some(pid) = child.id() {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .output();
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                use std::os::unix::process::CommandExt;
                if let Some(pid) = child.id() {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }
            // Wait up to 3s for clean exit
            let _ = tokio::time::timeout(Duration::from_secs(3), child.wait()).await;
        }
        *child_guard = None;

        if was_active {
            let _ = self.event_tx.send(ModelStatusEvent::Disconnected {
                provider: self.config.resolved_provider(),
            });
        }
        info!("MoE engine shut down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_launcher() -> (MoeLauncher, broadcast::Receiver<ModelStatusEvent>) {
        let (tx, rx) = ride_core::new_event_bus();
        let cfg = MoeLauncherConfig::default();
        (MoeLauncher::new(cfg, tx), rx)
    }

    #[test]
    fn test_default_config() {
        let cfg = MoeLauncherConfig::default();
        assert_eq!(cfg.port, 5000);
        assert_eq!(cfg.binary_path, PathBuf::from("rabbit"));
    }

    #[tokio::test]
    async fn test_initial_state_stopped() {
        let (launcher, _rx) = make_launcher();
        assert_eq!(launcher.state().await, ProcessState::Stopped);
    }

    #[tokio::test]
    async fn test_spawn_nonexistent_binary_fails() {
        let (tx, _rx) = ride_core::new_event_bus();
        let cfg = MoeLauncherConfig {
            binary_path: PathBuf::from("__nonexistent_binary_ride_test__"),
            ..Default::default()
        };
        let launcher = MoeLauncher::new(cfg, tx);
        let cancel = CancellationToken::new();
        let result = launcher.spawn(cancel).await;
        assert!(result.is_err());
        let state = launcher.state().await;
        assert!(matches!(state, ProcessState::Failed { .. }));
    }

    #[tokio::test]
    async fn test_shutdown_when_stopped_is_ok() {
        let (launcher, mut rx) = make_launcher();
        // Shutdown on a not-yet-started launcher should not panic
        assert!(launcher.shutdown().await.is_ok());
        // And should NOT send a Disconnected event when it was never active
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_resolved_provider_colibri_and_rabbit() {
        let rabbit_cfg = MoeLauncherConfig {
            binary_path: PathBuf::from("C:\\bin\\rabbit.exe"),
            ..Default::default()
        };
        assert_eq!(rabbit_cfg.resolved_provider(), ride_core::ProviderType::RabbitMoe);

        let colibri_cfg = MoeLauncherConfig {
            binary_path: PathBuf::from("/usr/local/bin/colibri"),
            ..Default::default()
        };
        assert_eq!(colibri_cfg.resolved_provider(), ride_core::ProviderType::ColibriMoe);

        let explicit_cfg = MoeLauncherConfig {
            binary_path: PathBuf::from("custom-bin"),
            provider: Some(ride_core::ProviderType::ColibriMoe),
            ..Default::default()
        };
        assert_eq!(explicit_cfg.resolved_provider(), ride_core::ProviderType::ColibriMoe);
    }

    #[tokio::test]
    async fn test_process_lifecycle_with_real_binary() {
        // Use a real binary that exists on Windows: `cmd /C exit 0`
        let (tx, mut rx) = ride_core::new_event_bus();
        let cfg = MoeLauncherConfig {
            binary_path: PathBuf::from("cmd"),
            extra_args: vec!["/C".into(), "ping -n 3 127.0.0.1 > NUL".into()],
            port: 59876, // unlikely to be in use
            ..Default::default()
        };
        let launcher = MoeLauncher::new(cfg, tx);
        let cancel = CancellationToken::new();

        // Spawn — `cmd` exists on every Windows system
        let pid = launcher.spawn(cancel.clone()).await;
        assert!(pid.is_ok(), "cmd.exe should spawn");

        // State should be Running
        let state = launcher.state().await;
        assert!(matches!(state, ProcessState::Running { .. }));

        // Shut down
        launcher.shutdown().await.unwrap();

        // State should be Stopped
        assert_eq!(launcher.state().await, ProcessState::Stopped);

        // Should have received a Disconnected event
        let ev = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(ev.is_ok());
    }

    #[tokio::test]
    async fn test_wait_until_ready_timeout_on_dead_port() {
        let (tx, _rx) = ride_core::new_event_bus();
        let cfg = MoeLauncherConfig {
            port: 65431, // nothing listening
            ..Default::default()
        };
        let launcher = MoeLauncher::new(cfg, tx);
        let client = reqwest::Client::new();
        // Should return false when timeout expires
        let ready = launcher.wait_until_ready(Duration::from_millis(300), &client).await;
        assert!(!ready);
    }

    #[tokio::test]
    async fn test_process_clean_exit_sets_stopped_state() {
        let (tx, mut rx) = ride_core::new_event_bus();
        let cfg = MoeLauncherConfig {
            binary_path: PathBuf::from("cmd"),
            extra_args: vec!["/C".into(), "exit 0".into()],
            port: 59877,
            ..Default::default()
        };
        let launcher = MoeLauncher::new(cfg, tx);
        let cancel = CancellationToken::new();

        launcher.spawn(cancel).await.expect("cmd should spawn");

        // Wait for process to exit cleanly and watcher to update state
        tokio::time::sleep(Duration::from_millis(600)).await;

        let state = launcher.state().await;
        assert_eq!(state, ProcessState::Stopped, "clean exit with status 0 must be Stopped, not Failed");

        // Disconnected event should have fired
        let ev = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(ev.is_ok());
    }
}
