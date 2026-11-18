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
  additions: usize;
  deletions: usize;
}

type usize = number;

export interface MoeEngineStatus {
  is_running: boolean;
  active_model_name?: string;
  streamed_experts_count: number;
  disk_read_mbs: number;
  ram_resident_mb: number;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  thoughts?: string[];
  toolCalls?: Array<{ name: string; args: any; output?: string; status?: 'pending' | 'success' | 'failed' }>;
  timestamp: number;
}

export interface AppState {
  activeLayout: UiLayout;
  activeFile: string;
  openFiles: string[];
  providers: ProviderStatus[];
  skills: SkillInfo[];
  diffs: DiffPayload[];
  messages: ChatMessage[];
  currentModel: string;
  currentProvider: string;
  tokensPerSec: number;
  ramUsageMb: number;
  isStreaming: boolean;
}
