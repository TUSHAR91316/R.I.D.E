export type UiLayout = 'vs_code' | 'claude_code' | 'codex';

export interface ModelSummary {
  id: string;
  name: string;
  is_moe: boolean;
  context_length?: number;
}

export interface ProviderStatus {
  provider: string;
  base_url: string;
  is_online: boolean;
  latency_ms?: number;
  models: ModelSummary[];
}

export interface SkillInfo {
  name: string;
  description: string;
  triggers: string[];
  enabled: boolean;
}

export interface DiffPayload {
  path: string;
  diff_text: string;
  additions: number;
  deletions: number;
  /**
   * True for edits the agent already wrote to disk (via its write/edit tools,
   * with a backup snapshot). These can only be reverted, not applied.
   */
  applied?: boolean;
  /** Number of agent edits to this file in the session (each has a snapshot). */
  edits?: number;
}

/** A shell command the agent is blocked on until the user allows or denies it. */
export interface ApprovalItem {
  id: string;
  command: string;
  /** Backend `CommandSafetyLevel` name, e.g. `PotentiallyDestructive`. */
  safety: string;
  status: 'pending' | 'sending' | 'approved' | 'denied';
}

export interface MoeEngineStatus {
  is_running: boolean;
  active_model_name?: string;
  streamed_experts_count: number;
  disk_read_mbs: number;
  ram_resident_mb: number;
}

export interface WorkspaceInfo {
  root: string;
  name: string;
}

export interface DirEntryInfo {
  name: string;
  path: string;
  is_dir: boolean;
}

export interface SearchHit {
  path: string;
  line: number;
  text: string;
}

export interface ToolCall {
  name: string;
  args: unknown;
  output?: string;
  status?: 'pending' | 'success' | 'failed';
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  thoughts?: string[];
  toolCalls?: ToolCall[];
  approvals?: ApprovalItem[];
  /** The user stopped this turn. */
  cancelled?: boolean;
  timestamp: number;
  isError?: boolean;
  /** Wall-clock duration of the agent turn, in ms. */
  durationMs?: number;
}

/** Monotonic counters bumped whenever the matching slice of state changes. */
export interface StateRevisions {
  messages: number;
  providers: number;
  skills: number;
  diffs: number;
}

export interface AppState {
  activeLayout: UiLayout;
  workspace: WorkspaceInfo;
  providers: ProviderStatus[];
  skills: SkillInfo[];
  diffs: DiffPayload[];
  messages: ChatMessage[];
  currentModel: string;
  currentProvider: string;
  isStreaming: boolean;
  rev: StateRevisions;
}
