/// Tauri IPC commands for the R.I.D.E desktop application.
///
/// These commands are exposed to the frontend via the Tauri IPC bridge and
/// delegate to the underlying ride-* library crates.
use ride_agent::{
    AgentSession, AgentSessionConfig, ApprovalDecision, ApprovalPolicy, DiffEngine, DiffManager,
    TerminalApprovalGate, UnifiedDiff,
};
use ride_core::UiLayout;
use ride_discovery::ModelDetector;
use ride_moe::{MoeEngineConfig, MoeEngineManager, MoeEngineStatus};
use ride_skills::SkillRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tauri::State;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

// ─── Shared application state ────────────────────────────────────────────────

/// A serialisable summary of a provider and its connected models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub base_url: String,
    pub is_online: bool,
    pub latency_ms: Option<u64>,
    pub models: Vec<ModelSummary>,
}

/// Lightweight model summary exposed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSummary {
    pub id: String,
    pub name: String,
    pub is_moe: bool,
    pub context_length: Option<usize>,
}

/// Serialisable skill summary for the SkillsManager UI panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub enabled: bool,
}

/// A staged diff proposal ready for UI review, apply, or rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedDiff {
    pub path: String,
    pub original_content: String,
    pub proposed_content: String,
    pub diff: UnifiedDiff,
}

/// Payload returned by `get_diff_preview`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPayload {
    pub path: String,
    pub diff_text: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Shell-command approvals the agent is blocked on, keyed by request id.
pub type PendingApprovals = Arc<Mutex<HashMap<String, oneshot::Sender<ApprovalDecision>>>>;

/// Per-window mutable desktop state kept behind a Tauri-managed Mutex.
pub struct DesktopState {
    pub active_layout: UiLayout,
    pub enabled_skills: Vec<String>,
    pub pending_diffs: Vec<StagedDiff>,
    pub diff_manager: Arc<DiffManager>,
    pub moe_manager: MoeEngineManager,
    pub pending_approvals: PendingApprovals,
    /// Cancellation handle of the agent turn in flight, if any.
    pub active_turn: Option<CancellationToken>,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            active_layout: UiLayout::default(),
            enabled_skills: Vec::new(),
            pending_diffs: Vec::new(),
            diff_manager: Arc::new(DiffManager::new(3)),
            moe_manager: MoeEngineManager::new(MoeEngineConfig::default()),
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            active_turn: None,
        }
    }
}

/// Resolve a pending approval. Returns `false` if no request has that id.
pub fn resolve_approval(approvals: &PendingApprovals, request_id: &str, decision: ApprovalDecision) -> bool {
    let sender = approvals.lock().ok().and_then(|mut m| m.remove(request_id));
    match sender {
        Some(tx) => {
            let _ = tx.send(decision);
            true
        }
        None => false,
    }
}

/// Deny every pending approval (used when a turn is cancelled or ends).
pub fn deny_all_approvals(approvals: &PendingApprovals, reason: &str) -> usize {
    let drained: Vec<_> = match approvals.lock() {
        Ok(mut m) => m.drain().map(|(_, tx)| tx).collect(),
        Err(_) => return 0,
    };
    let n = drained.len();
    for tx in drained {
        let _ = tx.send(ApprovalDecision::Denied { reason: reason.to_string() });
    }
    n
}

impl DesktopState {
    pub fn stage_diff(&mut self, path: &str, original: &str, modified: &str) -> DiffPayload {
        let diff = DiffEngine::create_diff(path, original, modified, 3);
        let payload = DiffPayload {
            path: diff.path.to_string_lossy().to_string(),
            diff_text: diff.diff_text.clone(),
            additions: diff.additions,
            deletions: diff.deletions,
        };
        self.pending_diffs.retain(|d| d.path != path);
        self.pending_diffs.push(StagedDiff {
            path: path.to_string(),
            original_content: original.to_string(),
            proposed_content: modified.to_string(),
            diff,
        });
        payload
    }

    pub fn apply_diff_path(&mut self, path: &str) -> Result<(), String> {
        let pos = self
            .pending_diffs
            .iter()
            .position(|d| d.path == path || d.diff.path.to_string_lossy() == path)
            .ok_or_else(|| format!("No staged diff found for path '{}'", path))?;

        let staged = self.pending_diffs.remove(pos);
        self.diff_manager
            .apply_write_with_backup(&staged.path, &staged.proposed_content)
            .map_err(|e| format!("Failed to apply diff for '{}': {}", staged.path, e))?;
        Ok(())
    }

    pub fn rollback_diff_path(&mut self, path: &str) -> Result<String, String> {
        let was_pending = self
            .pending_diffs
            .iter()
            .any(|d| d.path == path || d.diff.path.to_string_lossy() == path);
        self.pending_diffs
            .retain(|d| d.path != path && d.diff.path.to_string_lossy() != path);

        match self.diff_manager.rollback_latest_for_path(path) {
            Ok(snap) => Ok(format!("Rolled back {} (restored snapshot #{})", path, snap.id)),
            Err(e) => {
                if was_pending {
                    Ok(format!("Discarded staged changes for {}", path))
                } else {
                    Err(format!("Rollback failed for {}: {}", path, e))
                }
            }
        }
    }
}

// ─── IPC Commands ─────────────────────────────────────────────────────────────

/// Return the list of local providers with online status and their available models.
#[tauri::command]
pub async fn get_model_status() -> Result<Vec<ProviderStatus>, String> {
    let detector = ModelDetector::default();
    let statuses = detector.scan_all().await;
    let models = detector.discover_all_models().await;

    let mut result: Vec<ProviderStatus> = statuses
        .iter()
        .map(|s| ProviderStatus {
            provider: s.provider.display_name().to_string(),
            base_url: s.endpoint.clone(),
            is_online: s.is_online,
            latency_ms: s.latency_ms,
            models: models
                .iter()
                .filter(|m| m.provider == s.provider)
                .map(|m| ModelSummary {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    is_moe: m.is_moe,
                    context_length: m.context_length,
                })
                .collect(),
        })
        .collect();

    result.sort_by(|a, b| b.is_online.cmp(&a.is_online));
    Ok(result)
}

/// Send a user message to the agent session and return the full response text.
/// Streaming events are emitted via the `agent://event` Tauri event channel.
#[tauri::command]
pub async fn send_agent_message(
    app: tauri::AppHandle,
    state: State<'_, Mutex<DesktopState>>,
    message: String,
    model_name: Option<String>,
) -> Result<String, String> {
    let cancel = CancellationToken::new();
    let (enabled_skills, diff_manager, approvals) = {
        let mut st = state.lock().map_err(|e| e.to_string())?;
        if st.active_turn.is_some() {
            return Err("The agent is already working on a turn.".into());
        }
        st.active_turn = Some(cancel.clone());
        (
            st.enabled_skills.clone(),
            Arc::clone(&st.diff_manager),
            Arc::clone(&st.pending_approvals),
        )
    };

    let result = run_agent_turn(&app, &message, model_name, enabled_skills, diff_manager, &approvals, cancel).await;

    // Whatever happened, nothing may stay blocked on this turn.
    deny_all_approvals(&approvals, "Turn ended");
    if let Ok(mut st) = state.lock() {
        st.active_turn = None;
    }
    result
}

/// Cancel the agent turn in flight. Pending approvals are denied so a turn
/// blocked on a shell-command prompt unwinds too. Returns whether a turn was running.
#[tauri::command]
pub fn cancel_agent_turn(state: State<Mutex<DesktopState>>) -> Result<bool, String> {
    let st = state.lock().map_err(|e| e.to_string())?;
    let Some(token) = st.active_turn.as_ref() else {
        return Ok(false);
    };
    token.cancel();
    deny_all_approvals(&st.pending_approvals, "Turn cancelled by user");
    Ok(true)
}

/// Answer a shell-command approval the agent is waiting on.
#[tauri::command]
pub async fn respond_approval(
    state: State<'_, Mutex<DesktopState>>,
    request_id: String,
    approved: bool,
) -> Result<(), String> {
    let approvals = {
        let st = state.lock().map_err(|e| e.to_string())?;
        Arc::clone(&st.pending_approvals)
    };
    let decision = || {
        if approved {
            ApprovalDecision::Approved
        } else {
            ApprovalDecision::Denied { reason: "Denied in the R.I.D.E UI".into() }
        }
    };
    // The `ApprovalRequired` event is emitted just before the request reaches
    // the forwarding task, so allow a short window for it to be registered.
    for _ in 0..25 {
        if resolve_approval(&approvals, &request_id, decision()) {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Err(format!("No pending approval with id '{request_id}' (the turn may have ended)"))
}

async fn run_agent_turn(
    app: &tauri::AppHandle,
    message: &str,
    model_name: Option<String>,
    enabled_skills: Vec<String>,
    diff_manager: Arc<DiffManager>,
    approvals: &PendingApprovals,
    cancel: CancellationToken,
) -> Result<String, String> {
    use ride_agent::{HttpLlmClient, LlmClient, MockLlmClient};
    use tauri::Emitter;

    let ride_config = ride_core::RideConfig::default();
    let mut registry = SkillRegistry::new();
    let _ = registry.load_from_dir(&ride_config.skills_dir);
    let active_manifests: Vec<_> = registry
        .list()
        .into_iter()
        .filter(|s| enabled_skills.contains(&s.metadata.name))
        .cloned()
        .collect();

    let mut dyn_tools = ride_skills::DynamicToolRegistry::new();
    for s in &active_manifests {
        let base_dir = s
            .base_dir
            .clone()
            .or_else(|| {
                s.source_path
                    .as_ref()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            })
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        for t in &s.tools {
            dyn_tools.register(ride_skills::DynamicSkillTool::new(t.clone(), &base_dir));
        }
    }

    let detector = ModelDetector::default();
    let statuses = detector.scan_all().await;
    let all_models = detector.discover_all_models().await;

    let chosen_model = model_name.unwrap_or_else(|| "local-model".into());

    // Try matching the provider that actually hosts chosen_model, or fallback to first online provider
    let online_provider = all_models
        .iter()
        .find(|m| m.name == chosen_model || m.id == chosen_model)
        .and_then(|m| statuses.iter().find(|s| s.is_online && s.provider == m.provider))
        .cloned()
        .or_else(|| statuses.into_iter().find(|s| s.is_online));

    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(64);

    let llm: Arc<dyn LlmClient> = if let Some(provider_status) = online_provider {
        Arc::new(
            HttpLlmClient::new(
                &provider_status.endpoint,
                provider_status.provider,
                &chosen_model,
            )
            .with_event_sender(event_tx.clone())
            .with_streaming(true),
        )
    } else {
        Arc::new(MockLlmClient::new_with_handler(move |_msgs| {
            "R.I.D.E is in Offline Mode: no local model detected on localhost ports (11434, 1234, 5000, 8080, 8000).\n\n\
             You can use R.I.D.E as a standalone code editor (edit, Ctrl+S to save, Ctrl+O to open).\n\
             To activate AI capabilities, start Ollama (`ollama serve`) or LM Studio — R.I.D.E will auto-connect dynamically."
                .to_string()
        }))
    };

    // Read-only and whitelisted commands run directly; anything else (and
    // anything destructive) waits for an answer from the UI.
    let policy = ApprovalPolicy::AllowWhitelisted;
    let config = AgentSessionConfig {
        model_name: chosen_model,
        max_turns: 5,
        approval_policy: policy,
        ..Default::default()
    };

    let (approval_tx, mut approval_rx) = mpsc::channel(8);
    let gate = TerminalApprovalGate::new(policy)
        .with_channel(approval_tx)
        .with_event_sender(event_tx.clone());

    // `with_event_sender` rebuilds the session's gate, so install ours after it.
    let mut session = AgentSession::new(config, llm)
        .with_skills(active_manifests)
        .with_dynamic_tools(dyn_tools)
        .with_diff_manager(diff_manager)
        .with_event_sender(event_tx)
        .with_approval_gate(Arc::new(gate));

    let pending = Arc::clone(approvals);
    let approval_forwarder = tokio::spawn(async move {
        while let Some((request, responder)) = approval_rx.recv().await {
            if let Ok(mut m) = pending.lock() {
                m.insert(request.id, responder);
            }
        }
    });

    let app_clone = app.clone();
    let event_listener = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let _ = app_clone.emit("agent://event", serde_json::to_value(&event).ok());
        }
    });

    let result = session.execute_turn(message, cancel).await;

    // Drop the session (and with it every event sender and the gate) so the
    // listener can drain already-buffered events instead of losing them to an abort.
    drop(session);
    approval_forwarder.abort();
    if tokio::time::timeout(std::time::Duration::from_millis(500), event_listener)
        .await
        .is_err()
    {
        tracing::warn!("agent event listener did not drain within 500ms");
    }

    let result = result.map_err(|e| e.to_string())?;
    Ok(result.final_response)
}

/// Return the list of installed skills with their enabled/disabled state.
#[tauri::command]
pub fn get_active_skills(state: State<Mutex<DesktopState>>) -> Result<Vec<SkillInfo>, String> {
    let st = state.lock().map_err(|e| e.to_string())?;
    let ride_config = ride_core::RideConfig::default();
    let mut registry = SkillRegistry::new();
    let _ = registry.load_from_dir(&ride_config.skills_dir);

    let skills = registry
        .list()
        .iter()
        .map(|s| SkillInfo {
            name: s.metadata.name.clone(),
            description: s.metadata.description.clone(),
            triggers: s.metadata.triggers.clone(),
            enabled: st.enabled_skills.contains(&s.metadata.name),
        })
        .collect();

    Ok(skills)
}

/// Toggle a skill on or off for the current workspace.
#[tauri::command]
pub fn toggle_skill(
    name: String,
    enabled: bool,
    state: State<Mutex<DesktopState>>,
) -> Result<(), String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    if enabled {
        if !st.enabled_skills.contains(&name) {
            st.enabled_skills.push(name);
        }
    } else {
        st.enabled_skills.retain(|s| s != &name);
    }
    Ok(())
}

/// Retrieve pending diffs queued for review.
#[tauri::command]
pub fn get_diff_preview(state: State<Mutex<DesktopState>>) -> Result<Vec<DiffPayload>, String> {
    let st = state.lock().map_err(|e| e.to_string())?;
    let payloads = st
        .pending_diffs
        .iter()
        .map(|d| DiffPayload {
            path: d.path.clone(),
            diff_text: d.diff.diff_text.clone(),
            additions: d.diff.additions,
            deletions: d.diff.deletions,
        })
        .collect();
    Ok(payloads)
}

/// Stage a diff for preview in the UI.
#[tauri::command]
pub fn stage_diff_preview(
    path: String,
    original: String,
    modified: String,
    state: State<Mutex<DesktopState>>,
) -> Result<DiffPayload, String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    Ok(st.stage_diff(&path, &original, &modified))
}

/// Apply a pending diff by file path (writes changes to disk and records backup snapshot).
#[tauri::command]
pub fn apply_diff(path: String, state: State<Mutex<DesktopState>>) -> Result<(), String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    st.apply_diff_path(&path)
}

/// Rollback a diff by file path, removing it from pending and attempting snapshot restoration.
#[tauri::command]
pub fn rollback_diff(path: String, state: State<Mutex<DesktopState>>) -> Result<String, String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    st.rollback_diff_path(&path)
}

/// Read the currently active UI layout setting.
#[tauri::command]
pub fn get_active_layout(state: State<Mutex<DesktopState>>) -> Result<String, String> {
    let st = state.lock().map_err(|e| e.to_string())?;
    let layout_str = match st.active_layout {
        UiLayout::VsCode => "vs_code",
        UiLayout::ClaudeCode => "claude_code",
        UiLayout::Codex => "codex",
    };
    Ok(layout_str.to_string())
}

/// Write the active UI layout setting.
#[tauri::command]
pub fn set_active_layout(
    layout: String,
    state: State<Mutex<DesktopState>>,
) -> Result<(), String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    st.active_layout = match layout.as_str() {
        "vs_code" | "vscode" => UiLayout::VsCode,
        "claude_code" | "claudecode" => UiLayout::ClaudeCode,
        "codex" | "zen" => UiLayout::Codex,
        other => {
            return Err(format!(
                "Unknown layout '{}'. Expected: vs_code, claude_code, codex",
                other
            ))
        }
    };
    Ok(())
}

/// Get the status of the local MoE engine.
#[tauri::command]
pub fn get_moe_status(state: State<Mutex<DesktopState>>) -> Result<MoeEngineStatus, String> {
    let st = state.lock().map_err(|e| e.to_string())?;
    Ok(st.moe_manager.status().clone())
}

// ─── Workspace & file system ──────────────────────────────────────────────────

static WORKSPACE_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Directory names never shown in the explorer or searched.
const IGNORED_DIRS: &[&str] = &[".git", "target", "node_modules", ".svn", ".hg"];

/// Find the workspace root: the nearest ancestor of `start` whose Cargo.toml
/// declares `[workspace]`, else the nearest ancestor holding `.git`, else `start`.
pub fn detect_workspace_root(start: &Path) -> PathBuf {
    let mut git_root = None;
    for dir in start.ancestors() {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest) {
            if text.lines().any(|l| l.trim() == "[workspace]") {
                return dir.to_path_buf();
            }
        }
        if git_root.is_none() && dir.join(".git").exists() {
            git_root = Some(dir.to_path_buf());
        }
    }
    git_root.unwrap_or_else(|| start.to_path_buf())
}

/// The current workspace root (detected lazily from the process cwd).
pub fn workspace_root() -> PathBuf {
    if let Some(root) = WORKSPACE_ROOT.read().ok().and_then(|g| g.clone()) {
        return root;
    }
    let start = std::env::var_os("RIDE_WORKSPACE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let root = detect_workspace_root(&start);
    set_workspace_root(root.clone());
    root
}

/// Switch the workspace root. The process cwd follows it so agent tools,
/// diff application and relative paths all agree on one base directory.
pub fn set_workspace_root(root: PathBuf) {
    let _ = std::env::set_current_dir(&root);
    if let Ok(mut guard) = WORKSPACE_ROOT.write() {
        *guard = Some(root);
    }
}

fn clean_path_arg(path: &str) -> &str {
    path.trim().trim_matches('"').trim_matches('\'')
}

/// Resolve a UI-supplied path: absolute paths are used as-is, relative paths
/// are joined onto the workspace root.
pub fn resolve_in_workspace(path: &str) -> PathBuf {
    let p = Path::new(clean_path_arg(path));
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace_root().join(p)
    }
}

/// Render `path` relative to the workspace root with forward slashes.
fn to_workspace_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub root: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntryInfo {
    pub name: String,
    /// Workspace-relative path with forward slashes.
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

fn workspace_info(root: &Path) -> WorkspaceInfo {
    WorkspaceInfo {
        root: root.to_string_lossy().to_string(),
        name: root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string()),
    }
}

#[tauri::command]
pub fn get_workspace_info() -> Result<WorkspaceInfo, String> {
    Ok(workspace_info(&workspace_root()))
}

/// Open a different folder as the workspace.
#[tauri::command]
pub fn open_workspace(path: String) -> Result<WorkspaceInfo, String> {
    let p = PathBuf::from(clean_path_arg(&path));
    let p = if p.is_absolute() { p } else { workspace_root().join(p) };
    let p = std::fs::canonicalize(&p).map_err(|e| format!("Cannot open '{}': {}", p.display(), e))?;
    if !p.is_dir() {
        return Err(format!("'{}' is not a folder", p.display()));
    }
    // canonicalize on Windows yields a `\\?\` verbatim prefix; strip it for display.
    let p = PathBuf::from(p.to_string_lossy().trim_start_matches(r"\\?\").to_string());
    set_workspace_root(p.clone());
    Ok(workspace_info(&p))
}

/// List one directory level (directories first, then files, each alphabetical).
pub fn list_dir_entries(root: &Path, rel: &str) -> Result<Vec<DirEntryInfo>, String> {
    let dir = if rel.is_empty() { root.to_path_buf() } else { root.join(rel) };
    let read = std::fs::read_dir(&dir).map_err(|e| format!("Cannot list '{}': {}", dir.display(), e))?;
    let mut entries: Vec<DirEntryInfo> = read
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir && IGNORED_DIRS.contains(&name.as_str()) {
                return None;
            }
            Some(DirEntryInfo {
                path: to_workspace_relative(&entry.path(), root),
                name,
                is_dir,
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub async fn list_dir(path: Option<String>) -> Result<Vec<DirEntryInfo>, String> {
    let root = workspace_root();
    let rel = path.map(|p| clean_path_arg(&p).trim_matches('/').to_string()).unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || list_dir_entries(&root, &rel))
        .await
        .map_err(|e| e.to_string())?
}

/// Text search across the workspace (skips .git, target, node_modules).
#[tauri::command]
pub async fn search_workspace(
    query: String,
    case_sensitive: Option<bool>,
    is_regex: Option<bool>,
) -> Result<Vec<SearchHit>, String> {
    let root = workspace_root();
    tauri::async_runtime::spawn_blocking(move || {
        ride_agent::Toolset::search_code_full(
            &root,
            &query,
            case_sensitive.unwrap_or(false),
            is_regex.unwrap_or(false),
            300,
        )
        .map(|hits| {
            hits.into_iter()
                .map(|m| SearchHit {
                    path: to_workspace_relative(&m.file, &root),
                    line: m.line_number,
                    text: m.line_text.chars().take(240).collect(),
                })
                .collect()
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Save file content to disk. Creates parent directories if necessary.
#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<String, String> {
    let full = resolve_in_workspace(&path);
    if let Some(parent) = full.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
    }
    tokio::fs::write(&full, &content)
        .await
        .map_err(|e| format!("Failed to save '{}': {}", full.display(), e))?;
    Ok(format!("Saved {}", full.display()))
}

/// Read file content from disk.
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    let full = resolve_in_workspace(&path);
    tokio::fs::read_to_string(&full)
        .await
        .map_err(|e| format!("Failed to read '{}': {}", full.display(), e))
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;
    use ride_core::UiLayout;

    fn make_state() -> DesktopState {
        DesktopState::default()
    }

    #[test]
    fn test_desktop_state_default_layout() {
        let st = make_state();
        assert_eq!(st.active_layout, UiLayout::VsCode);
        assert!(st.enabled_skills.is_empty());
        assert!(st.pending_diffs.is_empty());
        assert!(!st.moe_manager.status().is_running);
    }

    // ── Layout parsing ────────────────────────────────────────────────────────

    fn parse_layout(s: &str) -> Result<UiLayout, String> {
        match s {
            "vs_code" | "vscode" => Ok(UiLayout::VsCode),
            "claude_code" | "claudecode" => Ok(UiLayout::ClaudeCode),
            "codex" | "zen" => Ok(UiLayout::Codex),
            other => Err(format!("Unknown layout '{}'", other)),
        }
    }

    #[test]
    fn test_layout_vs_code() {
        assert_eq!(parse_layout("vs_code").unwrap(), UiLayout::VsCode);
        assert_eq!(parse_layout("vscode").unwrap(), UiLayout::VsCode);
    }

    #[test]
    fn test_layout_claude_code() {
        assert_eq!(parse_layout("claude_code").unwrap(), UiLayout::ClaudeCode);
        assert_eq!(parse_layout("claudecode").unwrap(), UiLayout::ClaudeCode);
    }

    #[test]
    fn test_layout_codex() {
        assert_eq!(parse_layout("codex").unwrap(), UiLayout::Codex);
        assert_eq!(parse_layout("zen").unwrap(), UiLayout::Codex);
    }

    #[test]
    fn test_layout_unknown() {
        assert!(parse_layout("unknown").is_err());
        assert!(parse_layout("").is_err());
    }

    // ── Skill toggle logic ────────────────────────────────────────────────────

    fn toggle_skill_logic(st: &mut DesktopState, name: &str, enabled: bool) {
        if enabled {
            if !st.enabled_skills.contains(&name.to_string()) {
                st.enabled_skills.push(name.to_string());
            }
        } else {
            st.enabled_skills.retain(|s| s != name);
        }
    }

    #[test]
    fn test_toggle_skill_enable() {
        let mut st = make_state();
        toggle_skill_logic(&mut st, "rust-expert", true);
        assert!(st.enabled_skills.contains(&"rust-expert".to_string()));
    }

    #[test]
    fn test_toggle_skill_disable() {
        let mut st = make_state();
        st.enabled_skills.push("rust-expert".to_string());
        toggle_skill_logic(&mut st, "rust-expert", false);
        assert!(!st.enabled_skills.contains(&"rust-expert".to_string()));
    }

    #[test]
    fn test_toggle_skill_no_duplicate() {
        let mut st = make_state();
        toggle_skill_logic(&mut st, "rust-expert", true);
        toggle_skill_logic(&mut st, "rust-expert", true);
        assert_eq!(
            st.enabled_skills
                .iter()
                .filter(|s| *s == "rust-expert")
                .count(),
            1
        );
    }

    #[test]
    fn test_toggle_skill_disable_non_existent() {
        let mut st = make_state();
        toggle_skill_logic(&mut st, "rust-expert", false);
        assert!(st.enabled_skills.is_empty());
    }

    // ── Diff queue & disk application logic ───────────────────────────────────

    #[test]
    fn test_diff_payload_serialization() {
        let diff = DiffEngine::create_diff("src/main.rs", "fn a() {}", "fn a() { b(); }", 3);
        let payload = DiffPayload {
            path: diff.path.to_string_lossy().to_string(),
            diff_text: diff.diff_text.clone(),
            additions: diff.additions,
            deletions: diff.deletions,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("src/main.rs"));
        assert!(json.contains("additions"));
    }

    #[test]
    fn test_stage_diff_stores_proposed_content() {
        let mut st = make_state();
        let payload = st.stage_diff("test.rs", "fn old() {}", "fn new() {}");
        assert_eq!(payload.path, "test.rs");
        assert_eq!(st.pending_diffs.len(), 1);
        assert_eq!(st.pending_diffs[0].proposed_content, "fn new() {}");
        assert_eq!(st.pending_diffs[0].original_content, "fn old() {}");
    }

    #[test]
    fn test_apply_diff_writes_to_disk_and_creates_backup() {
        let mut st = make_state();
        let temp_dir = std::env::temp_dir().join(format!(
            "ride_test_diff_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test.txt");
        let path_str = file_path.to_string_lossy().to_string();

        let original = "Hello World\nLine 2";
        std::fs::write(&file_path, original).unwrap();

        let modified = "Hello World\nLine 2 Modified\nLine 3";
        let payload = st.stage_diff(&path_str, original, modified);
        assert_eq!(payload.additions, 2);
        assert_eq!(st.pending_diffs.len(), 1);

        // Apply the diff
        let res = st.apply_diff_path(&path_str);
        assert!(res.is_ok());
        assert!(st.pending_diffs.is_empty());

        // Verify content on disk
        let disk_content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(disk_content, modified);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rollback_diff_restores_original_file_content() {
        let mut st = make_state();
        let temp_dir = std::env::temp_dir().join(format!(
            "ride_test_rollback_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test_rollback.txt");
        let path_str = file_path.to_string_lossy().to_string();

        let original = "Original Line 1\nOriginal Line 2";
        std::fs::write(&file_path, original).unwrap();

        let modified = "Modified Line 1\nModified Line 2\nExtra";
        st.stage_diff(&path_str, original, modified);

        // Apply diff to disk
        st.apply_diff_path(&path_str).unwrap();
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), modified);

        // Now rollback
        let rollback_res = st.rollback_diff_path(&path_str);
        assert!(rollback_res.is_ok());
        assert!(rollback_res.unwrap().contains("Rolled back"));

        // Verify original content is restored on disk
        let restored_content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(restored_content, original);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rollback_staged_diff_before_apply_discards_gracefully() {
        let mut st = make_state();
        st.stage_diff("fake_file.rs", "fn old() {}", "fn new() {}");
        assert_eq!(st.pending_diffs.len(), 1);

        let res = st.rollback_diff_path("fake_file.rs");
        assert!(res.is_ok());
        assert!(res.unwrap().contains("Discarded staged changes"));
        assert!(st.pending_diffs.is_empty());
    }

    #[test]
    fn test_apply_diff_nonexistent_returns_err() {
        let mut st = make_state();
        let res = st.apply_diff_path("missing.rs");
        assert!(res.is_err());
    }

    #[test]
    fn test_get_diff_preview_empty() {
        let st = make_state();
        assert!(st.pending_diffs.is_empty());
    }

    #[test]
    fn test_stage_diff_preview() {
        let diff = DiffEngine::create_diff("test.rs", "fn a() {}", "fn a() { println!(\"hi\"); }", 3);
        assert!(diff.additions > 0);
        let payload = DiffPayload {
            path: diff.path.to_string_lossy().to_string(),
            diff_text: diff.diff_text.clone(),
            additions: diff.additions,
            deletions: diff.deletions,
        };
        assert_eq!(payload.path, "test.rs");
        assert!(payload.diff_text.contains("+fn a() { println!(\"hi\"); }") || payload.diff_text.contains("hi"));
    }

    // ── ProviderStatus serialization ──────────────────────────────────────────

    #[test]
    fn test_provider_status_serialization() {
        let ps = ProviderStatus {
            provider: "Ollama".to_string(),
            base_url: "http://localhost:11434".to_string(),
            is_online: true,
            latency_ms: Some(15),
            models: vec![ModelSummary {
                id: "llama3.2".to_string(),
                name: "Llama 3.2 3B".to_string(),
                is_moe: false,
                context_length: Some(128_000),
            }],
        };
        let json = serde_json::to_string(&ps).unwrap();
        assert!(json.contains("Ollama"));
        assert!(json.contains("llama3.2"));

        let round_tripped: ProviderStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.provider, "Ollama");
        assert!(round_tripped.is_online);
        assert_eq!(round_tripped.latency_ms, Some(15));
    }

    #[test]
    fn test_model_summary_no_context_length() {
        let ms = ModelSummary {
            id: "mixtral".to_string(),
            name: "Mixtral 8x7B".to_string(),
            is_moe: true,
            context_length: None,
        };
        let json = serde_json::to_string(&ms).unwrap();
        let parsed: ModelSummary = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_moe);
        assert!(parsed.context_length.is_none());
    }

    // ── SkillInfo serialization ───────────────────────────────────────────────

    #[test]
    fn test_skill_info_serialization() {
        let si = SkillInfo {
            name: "rust-expert".to_string(),
            description: "Expert Rust guidance".to_string(),
            triggers: vec!["rust".to_string(), "cargo".to_string()],
            enabled: true,
        };
        let json = serde_json::to_string(&si).unwrap();
        let parsed: SkillInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "rust-expert");
        assert_eq!(parsed.triggers.len(), 2);
        assert!(parsed.enabled);
    }

    #[test]
    fn test_skill_info_disabled() {
        let si = SkillInfo {
            name: "py-expert".to_string(),
            description: "Python guidance".to_string(),
            triggers: vec![],
            enabled: false,
        };
        assert!(!si.enabled);
        assert!(si.triggers.is_empty());
    }

    // ── MoE Status serialization ──────────────────────────────────────────────

    #[test]
    fn test_moe_engine_status_integration() {
        let st = make_state();
        let status = st.moe_manager.status();
        assert!(!status.is_running);
        assert_eq!(status.streamed_experts_count, 0);
    }

    #[tokio::test]
    async fn test_save_and_read_file_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ride_test_save_read_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file_path = temp_dir.join("subfolder").join("hello.rs");
        let path_str = file_path.to_string_lossy().to_string();

        let code = "fn main() {\n    println!(\"Hello R.I.D.E\");\n}\n";
        // Test padded/quoted path
        let padded_path = format!("  \"{}\"  ", path_str);
        let save_res = save_file(padded_path.clone(), code.to_string()).await;
        assert!(save_res.is_ok());

        let read_res = read_file(padded_path).await;
        assert!(read_res.is_ok());
        assert_eq!(read_res.unwrap(), code);

        // Read non-existent file
        let read_missing = read_file(temp_dir.join("nonexistent.txt").to_string_lossy().to_string()).await;
        assert!(read_missing.is_err());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    fn unique_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ride_test_{}_{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_detect_workspace_root_finds_cargo_workspace() {
        let root = unique_temp_dir("ws_root");
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let nested = root.join("crates").join("app").join("src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("crates").join("app").join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();

        assert_eq!(detect_workspace_root(&nested), root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_list_dir_entries_sorts_dirs_first_and_skips_ignored() {
        let root = unique_temp_dir("list_dir");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("b.rs"), "").unwrap();
        std::fs::write(root.join("A.toml"), "").unwrap();
        std::fs::write(root.join("src").join("main.rs"), "").unwrap();

        let top = list_dir_entries(&root, "").unwrap();
        let names: Vec<_> = top.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["src", "A.toml", "b.rs"]);
        assert!(top[0].is_dir);

        let nested = list_dir_entries(&root, "src").unwrap();
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].path, "src/main.rs");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_resolve_in_workspace_keeps_absolute_paths() {
        let abs = std::env::temp_dir().join("x.rs");
        let quoted = format!("\"{}\"", abs.display());
        assert_eq!(resolve_in_workspace(&quoted), abs);
    }

    #[tokio::test]
    async fn approval_resolves_once_by_id() {
        let approvals: PendingApprovals = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = oneshot::channel();
        approvals.lock().unwrap().insert("req_1".into(), tx);

        assert!(!resolve_approval(&approvals, "req_9", ApprovalDecision::Approved));
        assert!(resolve_approval(&approvals, "req_1", ApprovalDecision::Approved));
        assert!(!resolve_approval(&approvals, "req_1", ApprovalDecision::Approved));
        assert_eq!(rx.await.unwrap(), ApprovalDecision::Approved);
    }

    #[tokio::test]
    async fn deny_all_unblocks_every_waiter() {
        let approvals: PendingApprovals = Arc::new(Mutex::new(HashMap::new()));
        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
        approvals.lock().unwrap().insert("a".into(), tx1);
        approvals.lock().unwrap().insert("b".into(), tx2);

        assert_eq!(deny_all_approvals(&approvals, "cancelled"), 2);
        assert!(approvals.lock().unwrap().is_empty());
        for rx in [rx1, rx2] {
            assert!(matches!(rx.await.unwrap(), ApprovalDecision::Denied { .. }));
        }
    }

    #[tokio::test]
    async fn gate_round_trip_through_pending_map() {
        let approvals: PendingApprovals = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) = mpsc::channel(4);
        let gate = TerminalApprovalGate::new(ApprovalPolicy::AllowWhitelisted).with_channel(tx);

        let pending = Arc::clone(&approvals);
        tokio::spawn(async move {
            while let Some((req, responder)) = rx.recv().await {
                pending.lock().unwrap().insert(req.id, responder);
            }
        });

        // Whitelisted commands never reach the UI.
        gate.check_approval("git status", None).await.unwrap();

        let check = tokio::spawn(async move { gate.check_approval("rm -rf build", None).await });
        let id = loop {
            if let Some(k) = approvals.lock().unwrap().keys().next().cloned() {
                break k;
            }
            tokio::task::yield_now().await;
        };
        assert!(resolve_approval(&approvals, &id, ApprovalDecision::Denied { reason: "no".into() }));
        assert!(check.await.unwrap().is_err());
    }
}
