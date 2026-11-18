import type { DiffPayload, MoeEngineStatus, ProviderStatus, SkillInfo, UiLayout } from './types';

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

export async function onAgentEvent(callback: (event: any) => void): Promise<() => void> {
  if (isTauri()) {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen('agent://event', (e) => callback(e.payload));
      return unlisten;
    } catch (err) {
      console.warn('[Tauri Event Listener Error]:', err);
    }
  }
  return () => {};
}

// ─── Browser Fallback Mocks ───────────────────────────────────────────────────

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

    case 'send_agent_message': {
      const text = args?.message || '';
      return `R.I.D.E Local Agent processed request: "${text}". Checked workspace files and AST structures.` as unknown as T;
    }

    default:
      console.warn(`[Mock IPC] Unknown command: ${cmd}`);
      return undefined as unknown as T;
  }
}
