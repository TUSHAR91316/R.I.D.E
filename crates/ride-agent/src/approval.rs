use ride_core::error::{Result, RideError};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot};

/// Security policies governing command execution in the agent terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    /// Allow all commands without user intervention
    AlwaysAllow,
    /// Prompt user approval for every command
    #[default]
    AskEveryTime,
    /// Automatically allow read-only and whitelisted commands; prompt for others
    AllowWhitelisted,
    /// Reject all command execution unconditionally
    Deny,
}

/// Safety categorization of terminal shell commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSafetyLevel {
    /// Non-mutating inspection commands (e.g. `git status`, `dir`, `cargo check`)
    ReadOnly,
    /// Standard non-destructive development commands (e.g. `cargo build`, `cargo test`)
    SafeExecution,
    /// Potentially destructive commands (e.g. `rm`, `del`, `format`, `git reset --hard`)
    PotentiallyDestructive,
    /// Unclassified or complex command invocation
    Unknown,
}

/// Categorizes command safety using pattern matching and token analysis.
pub struct CommandSafetyClassifier;

impl CommandSafetyClassifier {
    pub fn classify(cmd: &str) -> CommandSafetyLevel {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return CommandSafetyLevel::ReadOnly;
        }

        // Check if command is chained with &&, ||, ;, |
        let sub_commands = Self::split_chain_commands(trimmed);
        if sub_commands.len() > 1 {
            let mut highest_risk = CommandSafetyLevel::ReadOnly;
            for sub in sub_commands {
                let level = Self::classify_single(&sub);
                match level {
                    CommandSafetyLevel::PotentiallyDestructive => {
                        return CommandSafetyLevel::PotentiallyDestructive;
                    }
                    CommandSafetyLevel::Unknown => {
                        highest_risk = CommandSafetyLevel::Unknown;
                    }
                    CommandSafetyLevel::SafeExecution => {
                        if highest_risk != CommandSafetyLevel::Unknown {
                            highest_risk = CommandSafetyLevel::SafeExecution;
                        }
                    }
                    CommandSafetyLevel::ReadOnly => {}
                }
            }
            return highest_risk;
        }

        Self::classify_single(trimmed)
    }

    fn split_chain_commands(cmd: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut cur = String::new();
        let mut in_quote: Option<char> = None;

        let chars: Vec<char> = cmd.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];
            if let Some(q) = in_quote {
                cur.push(c);
                if c == q {
                    in_quote = None;
                }
                i += 1;
            } else if c == '\'' || c == '"' {
                cur.push(c);
                in_quote = Some(c);
                i += 1;
            } else if c == ';' || (c == '&' && i + 1 < len && chars[i + 1] == '&') || (c == '|' && i + 1 < len && chars[i + 1] == '|') || c == '|' {
                if !cur.trim().is_empty() {
                    parts.push(cur.trim().to_string());
                    cur.clear();
                }
                if (c == '&' && i + 1 < len && chars[i + 1] == '&') || (c == '|' && i + 1 < len && chars[i + 1] == '|') {
                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                cur.push(c);
                i += 1;
            }
        }

        if !cur.trim().is_empty() {
            parts.push(cur.trim().to_string());
        }

        parts
    }

    fn classify_single(cmd: &str) -> CommandSafetyLevel {
        let lower = cmd.to_lowercase();
        let tokens: Vec<&str> = lower.split_whitespace().collect();
        if tokens.is_empty() {
            return CommandSafetyLevel::ReadOnly;
        }

        let first = tokens[0];

        // 1. Potentially Destructive Commands & Patterns
        if Self::is_destructive_pattern(&lower, &tokens, first) {
            return CommandSafetyLevel::PotentiallyDestructive;
        }

        // 2. Read-Only Commands
        if Self::is_read_only_pattern(&lower, &tokens, first) {
            return CommandSafetyLevel::ReadOnly;
        }

        // 3. Safe Execution (Build & Test)
        if Self::is_safe_execution_pattern(&tokens, first) {
            return CommandSafetyLevel::SafeExecution;
        }

        CommandSafetyLevel::Unknown
    }

    fn is_destructive_pattern(lower: &str, tokens: &[&str], first: &str) -> bool {
        // Obvious delete/remove binaries & cmdlets
        let destructive_binaries = [
            "rm", "del", "erase", "rmdir", "rd", "remove-item", "ri",
            "format", "mkfs", "fdisk", "diskpart", "dd", "shred",
            "kill", "pkill", "taskkill", "stop-process",
            "shutdown", "reboot", "restart-computer",
        ];

        if destructive_binaries.contains(&first) {
            return true;
        }

        // Git destructive subcommands
        if first == "git" && tokens.len() > 1 {
            let sub = tokens[1];
            if sub == "reset" && lower.contains("--hard") {
                return true;
            }
            if sub == "clean" && (lower.contains("-f") || lower.contains("--force")) {
                return true;
            }
            if sub == "branch" && (lower.contains("-d") || lower.contains("-D")) {
                return true;
            }
            if sub == "push" && (lower.contains("--force") || lower.contains("-f")) {
                return true;
            }
        }

        // Pipeline execution from remote: curl | bash / irm | iex
        if lower.contains("curl") && (lower.contains("| sh") || lower.contains("| bash")) {
            return true;
        }
        if lower.contains("irm") && (lower.contains("| iex") || lower.contains("|invoke-expression")) {
            return true;
        }

        // Recursive chmod/chown
        if (first == "chmod" || first == "chown") && lower.contains("-r") {
            return true;
        }

        false
    }

    fn is_read_only_pattern(lower: &str, tokens: &[&str], first: &str) -> bool {
        // Filesystem listing & inspection
        let read_only_bins = [
            "dir", "ls", "pwd", "get-childitem", "gci", "get-location", "gl",
            "cat", "type", "get-content", "gc", "head", "tail", "more", "less",
            "echo", "write-output", "print", "which", "where", "where.exe",
            "get-command", "gcm", "findstr", "grep", "select-string", "sls",
        ];

        if read_only_bins.contains(&first) {
            return true;
        }

        // Git read-only subcommands
        if first == "git" && tokens.len() > 1 {
            let sub = tokens[1];
            let git_read_subs = [
                "status", "diff", "log", "show", "remote", "branch", "tag",
                "rev-parse", "config", "describe", "version",
            ];
            if git_read_subs.contains(&sub) && !lower.contains("-d") && !lower.contains("-D") {
                return true;
            }
        }

        // Cargo read-only
        if first == "cargo" && tokens.len() > 1 {
            let sub = tokens[1];
            let cargo_read_subs = ["check", "clippy", "tree", "metadata", "verify-project"];
            if cargo_read_subs.contains(&sub) {
                return true;
            }
        }

        // Version queries: `--version`, `-v`, `-V`
        if tokens.len() == 2 && (tokens[1] == "--version" || tokens[1] == "-v" || tokens[1] == "-V") {
            return true;
        }

        false
    }

    fn is_safe_execution_pattern(tokens: &[&str], first: &str) -> bool {
        if first == "cargo" && tokens.len() > 1 {
            let sub = tokens[1];
            let safe_cargo = ["build", "test", "run", "bench", "doc"];
            if safe_cargo.contains(&sub) {
                return true;
            }
        }

        if first == "npm" && tokens.len() > 1 {
            let sub = tokens[1];
            let safe_npm = ["test", "build", "run", "lint", "install"];
            if safe_npm.contains(&sub) {
                return true;
            }
        }

        if first == "pytest" || (first == "python" && tokens.len() > 2 && tokens[1] == "-m" && tokens[2] == "pytest") {
            return true;
        }

        false
    }
}

/// Request sent to prompt user approval for command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub command: String,
    pub working_dir: Option<PathBuf>,
    pub safety_level: CommandSafetyLevel,
    pub description: String,
}

/// Result of user approval inquiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Denied { reason: String },
}

pub type ApprovalCallback = Arc<
    dyn Fn(ApprovalRequest) -> Pin<Box<dyn Future<Output = ApprovalDecision> + Send>>
        + Send
        + Sync,
>;

pub type ApprovalChannelMessage = (ApprovalRequest, oneshot::Sender<ApprovalDecision>);

/// Security gate enforcing approval policies before executing terminal commands.
pub struct TerminalApprovalGate {
    policy: Arc<RwLock<ApprovalPolicy>>,
    whitelist: Arc<RwLock<Vec<String>>>,
    channel_tx: Option<mpsc::Sender<ApprovalChannelMessage>>,
    callback: Option<ApprovalCallback>,
    request_counter: AtomicU64,
}

impl TerminalApprovalGate {
    pub fn new(policy: ApprovalPolicy) -> Self {
        Self {
            policy: Arc::new(RwLock::new(policy)),
            whitelist: Arc::new(RwLock::new(vec![
                "git status".into(),
                "git diff".into(),
                "cargo check".into(),
                "cargo test".into(),
                "dir".into(),
                "ls".into(),
            ])),
            channel_tx: None,
            callback: None,
            request_counter: AtomicU64::new(1),
        }
    }

    pub fn with_channel(mut self, tx: mpsc::Sender<ApprovalChannelMessage>) -> Self {
        self.channel_tx = Some(tx);
        self
    }

    pub fn with_callback(mut self, cb: ApprovalCallback) -> Self {
        self.callback = Some(cb);
        self
    }

    pub fn with_whitelist(self, whitelist: Vec<String>) -> Self {
        let mut guard = self.whitelist.write().unwrap();
        *guard = whitelist;
        drop(guard);
        self
    }

    pub fn add_whitelisted(&self, prefix: &str) {
        let mut guard = self.whitelist.write().unwrap();
        guard.push(prefix.to_string());
    }

    pub fn set_policy(&self, policy: ApprovalPolicy) {
        let mut guard = self.policy.write().unwrap();
        *guard = policy;
    }

    pub fn policy(&self) -> ApprovalPolicy {
        *self.policy.read().unwrap()
    }

    /// Evaluates policy and prompts for approval if required.
    pub async fn check_approval(&self, cmd: &str, working_dir: Option<&Path>) -> Result<()> {
        let policy = self.policy();
        match policy {
            ApprovalPolicy::AlwaysAllow => Ok(()),
            ApprovalPolicy::Deny => Err(RideError::Agent(format!(
                "Command '{cmd}' execution denied by Deny policy"
            ))),
            ApprovalPolicy::AllowWhitelisted => {
                let safety = CommandSafetyClassifier::classify(cmd);
                if safety == CommandSafetyLevel::ReadOnly {
                    return Ok(());
                }

                let is_whitelisted = {
                    let guard = self.whitelist.read().unwrap();
                    guard.iter().any(|w| cmd.trim().starts_with(w.trim()))
                };

                if is_whitelisted {
                    Ok(())
                } else {
                    self.prompt_approval(cmd, working_dir, safety).await
                }
            }
            ApprovalPolicy::AskEveryTime => {
                let safety = CommandSafetyClassifier::classify(cmd);
                self.prompt_approval(cmd, working_dir, safety).await
            }
        }
    }

    async fn prompt_approval(
        &self,
        cmd: &str,
        working_dir: Option<&Path>,
        safety: CommandSafetyLevel,
    ) -> Result<()> {
        let req_id = format!("req_{}", self.request_counter.fetch_add(1, Ordering::SeqCst));
        let request = ApprovalRequest {
            id: req_id,
            command: cmd.to_string(),
            working_dir: working_dir.map(|p| p.to_path_buf()),
            safety_level: safety,
            description: match safety {
                CommandSafetyLevel::PotentiallyDestructive => {
                    "Potentially destructive command requiring user confirmation".into()
                }
                CommandSafetyLevel::SafeExecution => {
                    "Build or test execution requiring approval".into()
                }
                CommandSafetyLevel::ReadOnly => "Read-only command requiring approval".into(),
                CommandSafetyLevel::Unknown => "Unverified command requiring approval".into(),
            },
        };

        // 1. Try channel sender (ideal for desktop UI event loop)
        if let Some(tx) = &self.channel_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            if tx.send((request.clone(), resp_tx)).await.is_ok() {
                match resp_rx.await {
                    Ok(ApprovalDecision::Approved) => return Ok(()),
                    Ok(ApprovalDecision::Denied { reason }) => {
                        return Err(RideError::Agent(format!(
                            "Command '{cmd}' denied by user: {reason}"
                        )));
                    }
                    Err(_) => {
                        return Err(RideError::Agent(
                            "Approval channel receiver dropped without response".into(),
                        ));
                    }
                }
            }
        }

        // 2. Try async callback
        if let Some(cb) = &self.callback {
            let decision = cb(request).await;
            match decision {
                ApprovalDecision::Approved => Ok(()),
                ApprovalDecision::Denied { reason } => Err(RideError::Agent(format!(
                    "Command '{cmd}' denied by user: {reason}"
                ))),
            }
        } else {
            // Default to deny if approval is required but no handler is wired
            Err(RideError::Agent(format!(
                "Command '{cmd}' requires user approval, but no approval handler is registered"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_destructive_commands() {
        assert_eq!(
            CommandSafetyClassifier::classify("rm -rf src/"),
            CommandSafetyLevel::PotentiallyDestructive
        );
        assert_eq!(
            CommandSafetyClassifier::classify("del /f /q *.*"),
            CommandSafetyLevel::PotentiallyDestructive
        );
        assert_eq!(
            CommandSafetyClassifier::classify("git reset --hard HEAD~1"),
            CommandSafetyLevel::PotentiallyDestructive
        );
        assert_eq!(
            CommandSafetyClassifier::classify("git clean -fd"),
            CommandSafetyLevel::PotentiallyDestructive
        );
        assert_eq!(
            CommandSafetyClassifier::classify("Remove-Item -Recurse -Force dist"),
            CommandSafetyLevel::PotentiallyDestructive
        );
    }

    #[test]
    fn test_classify_read_only_commands() {
        assert_eq!(
            CommandSafetyClassifier::classify("git status"),
            CommandSafetyLevel::ReadOnly
        );
        assert_eq!(
            CommandSafetyClassifier::classify("git diff HEAD"),
            CommandSafetyLevel::ReadOnly
        );
        assert_eq!(
            CommandSafetyClassifier::classify("cargo check"),
            CommandSafetyLevel::ReadOnly
        );
        assert_eq!(
            CommandSafetyClassifier::classify("dir"),
            CommandSafetyLevel::ReadOnly
        );
        assert_eq!(
            CommandSafetyClassifier::classify("Get-ChildItem -Path ."),
            CommandSafetyLevel::ReadOnly
        );
        assert_eq!(
            CommandSafetyClassifier::classify("echo hello"),
            CommandSafetyLevel::ReadOnly
        );
    }

    #[test]
    fn test_classify_chained_commands() {
        // Chain with one destructive command becomes PotentiallyDestructive
        assert_eq!(
            CommandSafetyClassifier::classify("git status && rm -rf temp"),
            CommandSafetyLevel::PotentiallyDestructive
        );
        // Chain of read-only commands remains ReadOnly
        assert_eq!(
            CommandSafetyClassifier::classify("git status && cargo check"),
            CommandSafetyLevel::ReadOnly
        );
    }

    #[tokio::test]
    async fn test_gate_always_allow_and_deny() {
        let allow_gate = TerminalApprovalGate::new(ApprovalPolicy::AlwaysAllow);
        assert!(allow_gate.check_approval("rm -rf /", None).await.is_ok());

        let deny_gate = TerminalApprovalGate::new(ApprovalPolicy::Deny);
        assert!(deny_gate.check_approval("git status", None).await.is_err());
    }

    #[tokio::test]
    async fn test_gate_allow_whitelisted() {
        let gate = TerminalApprovalGate::new(ApprovalPolicy::AllowWhitelisted);
        // Read-only is automatically allowed
        assert!(gate.check_approval("git status", None).await.is_ok());
        // Whitelisted is automatically allowed
        assert!(gate.check_approval("cargo test", None).await.is_ok());

        // Non-whitelisted destructive command fails without registered handler
        assert!(gate.check_approval("del file.txt", None).await.is_err());
    }

    #[tokio::test]
    async fn test_gate_channel_approval_flow() {
        let (tx, mut rx) = mpsc::channel(4);
        let gate = TerminalApprovalGate::new(ApprovalPolicy::AskEveryTime).with_channel(tx);

        // Spawn simulated user responding "Approved"
        tokio::spawn(async move {
            if let Some((_req, resp_tx)) = rx.recv().await {
                let _ = resp_tx.send(ApprovalDecision::Approved);
            }
        });

        assert!(gate.check_approval("cargo build", None).await.is_ok());
    }

    #[tokio::test]
    async fn test_gate_channel_denial_flow() {
        let (tx, mut rx) = mpsc::channel(4);
        let gate = TerminalApprovalGate::new(ApprovalPolicy::AskEveryTime).with_channel(tx);

        tokio::spawn(async move {
            if let Some((_req, resp_tx)) = rx.recv().await {
                let _ = resp_tx.send(ApprovalDecision::Denied {
                    reason: "Forbidden by user".into(),
                });
            }
        });

        let err = gate.check_approval("format c:", None).await.unwrap_err();
        assert!(err.to_string().contains("Forbidden by user"));
    }
}
