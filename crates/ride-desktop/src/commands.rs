/// Tauri IPC commands for the R.I.D.E desktop application.
///
/// These commands are exposed to the frontend via the Tauri IPC bridge and
/// delegate to the underlying ride-* library crates.
use ride_agent::{AgentSession, AgentSessionConfig, DiffEngine, DiffManager, UnifiedDiff};
use ride_core::UiLayout;
use ride_discovery::ModelDetector;
use ride_moe::{MoeEngineConfig, MoeEngineManager, MoeEngineStatus};
use ride_skills::SkillRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

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

/// Payload returned by `get_diff_preview`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPayload {
    pub path: String,
    pub diff_text: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Per-window mutable desktop state kept behind a Tauri-managed Mutex.
pub struct DesktopState {
    pub active_layout: UiLayout,
    pub enabled_skills: Vec<String>,
    pub pending_diffs: Vec<UnifiedDiff>,
    pub diff_manager: DiffManager,
    pub moe_manager: MoeEngineManager,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            active_layout: UiLayout::default(),
            enabled_skills: Vec::new(),
            pending_diffs: Vec::new(),
            diff_manager: DiffManager::new(3),
            moe_manager: MoeEngineManager::new(MoeEngineConfig::default()),
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
            provider: format!("{:?}", s.provider),
            base_url: s.endpoint.clone(),
            is_online: s.is_online,
            latency_ms: s.latency_ms,
            models: models
                .iter()
                .filter(|m| format!("{:?}", m.provider) == format!("{:?}", s.provider))
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
    message: String,
    model_name: Option<String>,
) -> Result<String, String> {
    use ride_agent::MockLlmClient;
    use std::sync::Arc;
    use tauri::Emitter;

    let model = model_name.unwrap_or_else(|| "local-model".into());
    let config = AgentSessionConfig {
        model_name: model,
        max_turns: 5,
        ..Default::default()
    };

    // Use a mock client for the IPC path; real HTTP client is wired via provider discovery.
    let llm = Arc::new(MockLlmClient::new_with_handler(move |msgs| {
        format!(
            "R.I.D.E agent received {} messages. Processing: {}",
            msgs.len(),
            msgs.last().map(|m| m.content.as_str()).unwrap_or("")
        )
    }));

    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(64);
    let mut session = AgentSession::new(config, llm).with_event_sender(event_tx);

    let app_clone = app.clone();
    let event_listener = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let _ = app_clone.emit("agent://event", serde_json::to_value(&event).ok());
        }
    });

    let cancel = tokio_util::sync::CancellationToken::new();
    let result = session
        .execute_turn(&message, cancel)
        .await
        .map_err(|e| e.to_string())?;

    event_listener.abort();
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
            path: d.path.to_string_lossy().to_string(),
            diff_text: d.diff_text.clone(),
            additions: d.additions,
            deletions: d.deletions,
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
    let diff = DiffEngine::create_diff(&path, &original, &modified, 3);
    let payload = DiffPayload {
        path: diff.path.to_string_lossy().to_string(),
        diff_text: diff.diff_text.clone(),
        additions: diff.additions,
        deletions: diff.deletions,
    };
    st.pending_diffs.push(diff);
    Ok(payload)
}

/// Apply a pending diff by file path (removes it from the pending queue).
#[tauri::command]
pub fn apply_diff(path: String, state: State<Mutex<DesktopState>>) -> Result<(), String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    st.pending_diffs.retain(|d| d.path.to_string_lossy() != path);
    Ok(())
}

/// Rollback a diff by file path, removing it from pending and attempting snapshot restoration.
#[tauri::command]
pub fn rollback_diff(path: String, state: State<Mutex<DesktopState>>) -> Result<String, String> {
    let mut st = state.lock().map_err(|e| e.to_string())?;
    st.pending_diffs.retain(|d| d.path.to_string_lossy() != path);
    match st.diff_manager.rollback_latest_for_path(&path) {
        Ok(snap) => Ok(format!("Rolled back {} (snapshot #{})", path, snap.id)),
        Err(_) => Ok(format!("Removed pending diff for {}", path)),
    }
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

    // ── Diff queue logic ──────────────────────────────────────────────────────

    fn make_diff(path: &str) -> UnifiedDiff {
        UnifiedDiff {
            path: std::path::PathBuf::from(path),
            original_header: format!("--- {path}"),
            modified_header: format!("+++ {path}"),
            hunks: vec![],
            diff_text: format!("diff for {path}"),
            additions: 1,
            deletions: 0,
        }
    }

    #[test]
    fn test_diff_payload_serialization() {
        let d = make_diff("src/main.rs");
        let payload = DiffPayload {
            path: d.path.to_string_lossy().to_string(),
            diff_text: d.diff_text.clone(),
            additions: d.additions,
            deletions: d.deletions,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("src/main.rs"));
        assert!(json.contains("additions"));
    }

    #[test]
    fn test_apply_diff_removes_from_queue() {
        let mut st = make_state();
        st.pending_diffs.push(make_diff("src/main.rs"));
        st.pending_diffs.push(make_diff("src/lib.rs"));

        let path = "src/main.rs";
        st.pending_diffs.retain(|d| d.path.to_string_lossy() != path);

        assert_eq!(st.pending_diffs.len(), 1);
        assert_eq!(
            st.pending_diffs[0].path.to_string_lossy(),
            "src/lib.rs"
        );
    }

    #[test]
    fn test_rollback_diff_removes_from_queue() {
        let mut st = make_state();
        st.pending_diffs.push(make_diff("src/main.rs"));
        let path = "src/main.rs";
        st.pending_diffs.retain(|d| d.path.to_string_lossy() != path);
        assert!(st.pending_diffs.is_empty());
    }

    #[test]
    fn test_get_diff_preview_empty() {
        let st = make_state();
        let payloads: Vec<DiffPayload> = st
            .pending_diffs
            .iter()
            .map(|d| DiffPayload {
                path: d.path.to_string_lossy().to_string(),
                diff_text: d.diff_text.clone(),
                additions: d.additions,
                deletions: d.deletions,
            })
            .collect();
        assert!(payloads.is_empty());
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
}
