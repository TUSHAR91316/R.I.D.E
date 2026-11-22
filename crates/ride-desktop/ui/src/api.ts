import type {
  DiffPayload,
  DirEntryInfo,
  MoeEngineStatus,
  ProviderStatus,
  SearchHit,
  SkillInfo,
  UiLayout,
  WorkspaceInfo,
} from './types';

// Check if running inside Tauri WebView
function isTauri(): boolean {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
}

// Dynamic invoke import to prevent bundler errors in browser mode
async function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC Error on ${cmd}]:`, err);
      throw err;
    }
  }

  // Fallback mocks for development and standalone testing
  return mockInvoke<T>(cmd, args);
}

export async function getModelStatus(): Promise<ProviderStatus[]> {
  return callInvoke<ProviderStatus[]>('get_model_status');
}

export async function sendAgentMessage(message: string, modelName?: string): Promise<string> {
  return callInvoke<string>('send_agent_message', { message, modelName });
}

/** Stop the agent turn in flight. Resolves to false if no turn was running. */
export async function cancelAgentTurn(): Promise<boolean> {
  return callInvoke<boolean>('cancel_agent_turn');
}

/** Allow or deny a shell command the agent is waiting on. */
export async function respondApproval(requestId: string, approved: boolean): Promise<void> {
  return callInvoke<void>('respond_approval', { requestId, approved });
}

export async function getActiveSkills(): Promise<SkillInfo[]> {
  return callInvoke<SkillInfo[]>('get_active_skills');
}

export async function toggleSkill(name: string, enabled: boolean): Promise<void> {
  return callInvoke<void>('toggle_skill', { name, enabled });
}

export async function getDiffPreview(): Promise<DiffPayload[]> {
  return callInvoke<DiffPayload[]>('get_diff_preview');
}

export async function stageDiffPreview(path: string, original: string, modified: string): Promise<DiffPayload> {
  return callInvoke<DiffPayload>('stage_diff_preview', { path, original, modified });
}

export async function applyDiff(path: string): Promise<void> {
  return callInvoke<void>('apply_diff', { path });
}

export async function rollbackDiff(path: string): Promise<string> {
  return callInvoke<string>('rollback_diff', { path });
}

export async function getActiveLayout(): Promise<UiLayout> {
  const layout = await callInvoke<string>('get_active_layout');
  if (layout === 'claude_code' || layout === 'codex' || layout === 'vs_code') {
    return layout;
  }
  return 'vs_code';
}

export async function setActiveLayout(layout: UiLayout): Promise<void> {
  return callInvoke<void>('set_active_layout', { layout });
}

export async function getMoeStatus(): Promise<MoeEngineStatus> {
  return callInvoke<MoeEngineStatus>('get_moe_status');
}

export async function saveFile(path: string, content: string): Promise<string> {
  return callInvoke<string>('save_file', { path, content });
}

export async function readFile(path: string): Promise<string> {
  return callInvoke<string>('read_file', { path });
}

export async function getWorkspaceInfo(): Promise<WorkspaceInfo> {
  return callInvoke<WorkspaceInfo>('get_workspace_info');
}

export async function openWorkspace(path: string): Promise<WorkspaceInfo> {
  return callInvoke<WorkspaceInfo>('open_workspace', { path });
}

export async function listDir(path?: string): Promise<DirEntryInfo[]> {
  return callInvoke<DirEntryInfo[]>('list_dir', { path: path ?? null });
}

export async function searchWorkspace(
  query: string,
  caseSensitive = false,
  isRegex = false
): Promise<SearchHit[]> {
  return callInvoke<SearchHit[]>('search_workspace', { query, caseSensitive, isRegex });
}

export async function onModelEvent(callback: (event: any) => void): Promise<() => void> {
  if (isTauri()) {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen('model://event', (e) => callback(e.payload));
      return unlisten;
    } catch (err) {
      console.warn('[Tauri Model Event Listener Error]:', err);
    }
  }
  return () => {};
}

export async function onAgentEvent(callback: (event: any) => void): Promise<() => void> {
  if (isTauri()) {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen('agent://event', (e) => callback(e.payload));
      return unlisten;
    } catch (err) {
      console.warn('[Tauri Event Listener Error]:', err);
    }
    return () => {};
  }
  mockAgentListeners.add(callback);
  return () => mockAgentListeners.delete(callback);
}

// ─── Browser Fallback Mocks ───────────────────────────────────────────────────

const mockAgentListeners = new Set<(event: any) => void>();
const mockApprovalWaiters = new Map<string, (approved: boolean) => void>();
let mockTurn: { finish: (text: string, cancelled?: boolean) => void } | null = null;
let mockApprovalSeq = 1;

function emitMock(event: unknown): void {
  mockAgentListeners.forEach((cb) => cb(event));
}

/**
 * Browser-preview stand-in for an agent turn. It emits the same events the
 * backend does, so approvals, stop and agent-applied diffs can be exercised
 * without Tauri: mention "test"/"run"/"cargo" to trigger a shell-command
 * approval, and "edit"/"fix" to get an applied diff.
 */
function mockAgentTurn(text: string): Promise<string> {
  return new Promise<string>((resolve) => {
    let done = false;
    const timers: number[] = [];
    const later = (ms: number, fn: () => void) => timers.push(window.setTimeout(() => !done && fn(), ms));
    const finish = (reply: string, cancelled = false) => {
      if (done) return;
      done = true;
      timers.forEach(clearTimeout);
      mockApprovalWaiters.clear();
      mockTurn = null;
      if (cancelled) emitMock('TurnCancelled'); // serde serialises unit variants as bare strings
      resolve(reply);
    };
    mockTurn = { finish };

    const lower = text.toLowerCase();
    const wantsShell = /\b(test|run|cargo)\b/.test(lower);
    const wantsEdit = /\b(edit|fix|change)\b/.test(lower);

    const afterShell = () => {
      if (wantsEdit) {
        const path = 'crates/ride-desktop/src/main.rs';
        emitMock({ ToolCallRequested: { tool_name: 'edit_file', arguments: { path } } });
        later(500, () => {
          emitMock({
            DiffGenerated: {
              path,
              diff: `--- a/${path}\n+++ b/${path}\n@@ -1,3 +1,3 @@\n-// Browser preview: start the Tauri app to edit real files.\n+// Browser preview (edited by the mock agent).\n mod commands;\n `,
            },
          });
          emitMock({ ToolExecuted: { tool_name: 'edit_file', success: true, output_preview: `Successfully edited ${path} (1 additions, 1 deletions)` } });
          later(400, () => finish(`Done. I edited \`${path}\`; review it under **Changes** and revert it if you don't want it.`));
        });
      } else {
        later(500, () => finish(`(Browser preview) Mock reply to: "${text}". Start the desktop app to talk to a local model.`));
      }
    };

    emitMock({ ThinkingDelta: { delta: 'Looking at the request… ' } });
    if (!wantsShell) {
      later(600, afterShell);
      return;
    }

    const command = 'cargo test -p ride-desktop';
    const id = `req_${mockApprovalSeq++}`;
    later(500, () => {
      emitMock({ ToolCallRequested: { tool_name: 'run_command', arguments: { command } } });
      emitMock({ ApprovalRequired: { request_id: id, command, safety_level: 'SafeExecution' } });
      mockApprovalWaiters.set(id, (approved) => {
        emitMock({ ApprovalResponded: { request_id: id, approved } });
        later(700, () => {
          emitMock({
            ToolExecuted: {
              tool_name: 'run_command',
              success: approved,
              output_preview: approved ? 'test result: ok. 29 passed; 0 failed' : `Command '${command}' denied by user`,
            },
          });
          afterShell();
        });
      });
    });
  });
}

let mockSkills: SkillInfo[] = [
  { name: 'rust-expert', description: 'Deep idiomatic Rust engineering, borrow checker mastery, zero-copy architecture', triggers: ['rust', 'cargo', 'tokio', 'unsafe'], enabled: true },
  { name: 'clean-code', description: 'Robert C. Martin Clean Code principles, readability, refactoring, SOLID', triggers: ['refactor', 'clean', 'format'], enabled: true },
  { name: 'systematic-debugging', description: 'Root cause analysis, test-driven reproduction, edge case boundary tracing', triggers: ['debug', 'bug', 'fix', 'error'], enabled: true },
  { name: 'git-pushing', description: 'Conventional commits, atomic staging, push safety verification', triggers: ['git', 'commit', 'push'], enabled: false },
  { name: 'api-security-testing', description: 'OWASP 2025 security scanning, input injection checks, boundary audit', triggers: ['security', 'audit', 'vuln'], enabled: false },
];

let mockDiffs: DiffPayload[] = [
  {
    path: 'crates/ride-desktop/src/commands.rs',
    diff_text: `--- a/crates/ride-desktop/src/commands.rs
+++ b/crates/ride-desktop/src/commands.rs
@@ -108,3 +108,6 @@
+    // Stage diff preview helper for UI inspection
+    #[tauri::command]
+    pub fn stage_diff_preview(...)
     let cancel = tokio_util::sync::CancellationToken::new();`,
    additions: 4,
    deletions: 0,
  },
];

let mockLayout: UiLayout = 'vs_code';

const dir = (path: string): DirEntryInfo => ({ name: path.split('/').pop()!, path, is_dir: true });
const file = (path: string): DirEntryInfo => ({ name: path.split('/').pop()!, path, is_dir: false });

const mockTree: Record<string, DirEntryInfo[]> = {
  '': [dir('crates'), file('Cargo.lock'), file('Cargo.toml'), file('README.md')],
  crates: ['ride-agent', 'ride-core', 'ride-desktop', 'ride-discovery', 'ride-moe', 'ride-skills'].map((n) =>
    dir(`crates/${n}`)
  ),
  'crates/ride-desktop': [
    dir('crates/ride-desktop/src'),
    dir('crates/ride-desktop/ui'),
    file('crates/ride-desktop/Cargo.toml'),
    file('crates/ride-desktop/tauri.conf.json'),
  ],
  'crates/ride-desktop/src': [file('crates/ride-desktop/src/commands.rs'), file('crates/ride-desktop/src/main.rs')],
};

const mockFiles: Record<string, string> = {
  'README.md': '# R.I.D.E — Rust Intelligent Development Environment\n\n**R.I.D.E** is a sovereign, local-first code editor engineered by Byte-Labs.\nFeatures zero cloud telemetry, local MoE intelligence, and sub-millisecond discovery.\n',
  'Cargo.toml': '[workspace]\nresolver = "2"\nmembers = [\n    "crates/ride-core",\n    "crates/ride-discovery",\n    "crates/ride-moe",\n    "crates/ride-skills",\n    "crates/ride-agent",\n    "crates/ride-desktop",\n]\n',
  'crates/ride-desktop/src/main.rs':
    '//! R.I.D.E — Rust Intelligent Development Environment\n//! Engineered by Byte-Labs. High-performance local MoE architecture.\n\nuse std::sync::Arc;\nuse ride_core::pipeline::ExecutionStream;\nuse ride_moe::{ExpertRouter, QuantumConfig};\n\n/// Sovereign runtime running zero-cloud neural inference.\npub struct IntelligentRuntime {\n    router: Arc<ExpertRouter>,\n    workspace: String,\n}\n\nimpl IntelligentRuntime {\n    /// Initializes the local-first MoE dispatcher with NVMe streaming.\n    pub async fn new(workspace: impl Into<String>) -> Result<Self, Box<dyn std::error::Error>> {\n        let config = QuantumConfig::local_defaults();\n        let router = ExpertRouter::boot(config).await?;\n        Ok(Self {\n            router: Arc::new(router),\n            workspace: workspace.into(),\n        })\n    }\n\n    /// Dispatches an asynchronous turn across sovereign expert models.\n    pub async fn dispatch_turn(&self, prompt: &str) -> ExecutionStream {\n        self.router.route_expert_parallel(prompt).await\n    }\n}\n',
  'crates/ride-desktop/src/commands.rs':
    '/// Tauri IPC commands for the R.I.D.E desktop application.\n#[tauri::command]\npub async fn read_file(path: String) -> Result<String, String> {\n    tokio::fs::read_to_string(&path)\n        .await\n        .map_err(|e| format!("Failed to read {path}: {e}"))\n}\n',
};

async function mockInvoke<T>(cmd: string, args?: Record<string, any>): Promise<T> {
  switch (cmd) {
    case 'get_model_status':
      return [
        {
          provider: 'Ollama',
          base_url: 'http://localhost:11434',
          is_online: true,
          latency_ms: 12,
          models: [
            { id: 'qwen2.5-coder:7b', name: 'Qwen 2.5 Coder 7B', is_moe: false, context_length: 32768 },
            { id: 'llama3.2:3b', name: 'Llama 3.2 3B (Fast)', is_moe: false, context_length: 131072 },
          ],
        },
        {
          provider: 'ColibriMoe',
          base_url: 'http://127.0.0.1:8080',
          is_online: true,
          latency_ms: 4,
          models: [
            { id: 'mixtral-8x7b-instruct', name: 'Mixtral 8x7B MoE (NVMe Streamed)', is_moe: true, context_length: 32768 },
          ],
        },
        {
          provider: 'LmStudio',
          base_url: 'http://localhost:1234',
          is_online: false,
          latency_ms: undefined,
          models: [],
        },
      ] as unknown as T;

    case 'get_active_skills':
      return [...mockSkills] as unknown as T;

    case 'toggle_skill': {
      const name = args?.name;
      const enabled = Boolean(args?.enabled);
      const s = mockSkills.find((item) => item.name === name);
      if (s) s.enabled = enabled;
      return undefined as unknown as T;
    }

    case 'get_diff_preview':
      return [...mockDiffs] as unknown as T;

    case 'stage_diff_preview': {
      const payload: DiffPayload = {
        path: args?.path || 'src/sample.rs',
        diff_text: `--- a/${args?.path}\n+++ b/${args?.path}\n@@ -1,1 +1,2 @@\n-${args?.original}\n+${args?.modified}`,
        additions: 1,
        deletions: 1,
      };
      mockDiffs.push(payload);
      return payload as unknown as T;
    }

    case 'apply_diff': {
      const p = args?.path;
      mockDiffs = mockDiffs.filter((d) => d.path !== p);
      return undefined as unknown as T;
    }

    case 'rollback_diff': {
      const p = args?.path;
      mockDiffs = mockDiffs.filter((d) => d.path !== p);
      return `Rolled back ${p}` as unknown as T;
    }

    case 'get_active_layout':
      return mockLayout as unknown as T;

    case 'set_active_layout':
      mockLayout = args?.layout || 'vs_code';
      return undefined as unknown as T;

    case 'get_moe_status':
      return {
        is_running: true,
        active_model_name: 'Mixtral 8x7B MoE',
        streamed_experts_count: 8,
        disk_read_mbs: 1240.5,
        ram_resident_mb: 48,
      } as unknown as T;

    case 'save_file': {
      console.log(`[Mock Save] ${args?.path} (${(args?.content as string)?.length || 0} bytes)`);
      return `Saved ${args?.path}` as unknown as T;
    }

    case 'read_file': {
      const p = String(args?.path ?? '');
      if (mockFiles[p] !== undefined) return mockFiles[p] as unknown as T;
      throw new Error(`Failed to read '${p}': not found (browser preview)`);
    }

    case 'get_workspace_info':
      return { root: 'G:/Projects/R.I.D.E', name: 'R.I.D.E' } as unknown as T;

    case 'open_workspace': {
      const p = String(args?.path ?? '');
      return { root: p, name: p.split(/[\\/]/).filter(Boolean).pop() || p } as unknown as T;
    }

    case 'list_dir': {
      const p = String(args?.path ?? '');
      return (mockTree[p] ?? []) as unknown as T;
    }

    case 'search_workspace': {
      const q = String(args?.query ?? '').toLowerCase();
      if (!q) return [] as unknown as T;
      const hits: SearchHit[] = [];
      for (const [path, text] of Object.entries(mockFiles)) {
        text.split('\n').forEach((line, i) => {
          if (line.toLowerCase().includes(q)) hits.push({ path, line: i + 1, text: line });
        });
      }
      return hits as unknown as T;
    }

    case 'send_agent_message': {
      if (mockTurn) throw new Error('The agent is already working on a turn.');
      const text = String(args?.message || '');
      return (await mockAgentTurn(text)) as unknown as T;
    }

    case 'cancel_agent_turn': {
      const running = mockTurn !== null;
      mockTurn?.finish('Execution cancelled by user.', true);
      return running as unknown as T;
    }

    case 'respond_approval': {
      const id = String(args?.requestId ?? '');
      const waiter = mockApprovalWaiters.get(id);
      if (!waiter) throw new Error(`No pending approval with id '${id}' (the turn may have ended)`);
      mockApprovalWaiters.delete(id);
      waiter(Boolean(args?.approved));
      return undefined as unknown as T;
    }

    default:
      console.warn(`[Mock IPC] Unknown command: ${cmd}`);
      return undefined as unknown as T;
  }
}
