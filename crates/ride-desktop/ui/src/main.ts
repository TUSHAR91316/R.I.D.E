import type { AppState, ChatMessage, ProviderStatus, UiLayout } from './types';
import * as api from './api';

import { TitleBar } from './components/TitleBar';
import { StatusBar } from './components/StatusBar';
import { EditorComponent } from './components/Editor';
import { AgentChat } from './components/AgentChat';
import { FileTree } from './components/FileTree';
import { SearchPanel } from './components/SearchPanel';
import { SkillsManager } from './components/SkillsManager';
import { CommandPalette, type CommandItem } from './components/CommandPalette';
import type { DiffHandlers } from './components/DiffViewer';

import { VsCodeLayout } from './layouts/VsCodeLayout';
import { ClaudeCodeLayout } from './layouts/ClaudeCodeLayout';
import { CodexLayout } from './layouts/CodexLayout';
import { askText, toast } from './util';

/** Fallback poll interval; discovery events from the backend drive most refreshes. */
const PROVIDER_POLL_MS = 15_000;

type Theme = 'dark' | 'light';

function loadTheme(): Theme {
  try {
    return localStorage.getItem('ride.theme') === 'light' ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

class RideApplication {
  private state: AppState = {
    activeLayout: 'vs_code',
    workspace: { root: '', name: 'Workspace' },
    providers: [],
    skills: [],
    diffs: [],
    messages: [],
    currentModel: '',
    currentProvider: 'Offline',
    isStreaming: false,
    rev: { messages: 0, providers: 0, skills: 0, diffs: 0 },
  };

  private layoutHost: HTMLElement;
  private titleBar: TitleBar;
  private statusBar: StatusBar;
  private editor: EditorComponent;
  private chat: AgentChat;
  private fileTree: FileTree;
  private search: SearchPanel;
  private skills: SkillsManager;
  private palette: CommandPalette;
  private vscode: VsCodeLayout;
  private agent: ClaudeCodeLayout;
  private zen: CodexLayout;

  private renderQueued = false;
  private providersKey = '';
  private userPickedModel = false;
  private theme: Theme = loadTheme();

  constructor(root: HTMLElement) {
    this.applyTheme(this.theme);

    this.editor = new EditorComponent();
    this.chat = new AgentChat({
      onSend: (msg) => void this.sendMessage(msg),
      onSelectModel: (id) => this.selectModel(id),
      onApproval: (id, approved) => void this.respondApproval(id, approved),
      onStop: () => void this.cancelTurn(),
      getContextPath: () => {
        const tab = this.editor.getActiveTab();
        return tab && !tab.isUntitled ? tab.path : null;
      },
    });
    this.fileTree = new FileTree((path) => void this.editor.openFile(path));
    this.search = new SearchPanel((path, line) => void this.editor.openFile(path, undefined, line));
    this.skills = new SkillsManager((name, enabled) => this.toggleSkill(name, enabled));
    this.palette = new CommandPalette((prompt) => void this.sendMessage(prompt));

    const diffHandlers: DiffHandlers = {
      onApply: (path) => this.applyDiff(path),
      onRollback: (path) => this.rollbackDiff(path),
      onKeep: (path) => this.keepDiff(path),
    };
    const agentHandlers = {
      onApproval: (id: string, approved: boolean) => void this.respondApproval(id, approved),
      onStop: () => void this.cancelTurn(),
    };

    this.vscode = new VsCodeLayout({
      editor: this.editor,
      chat: this.chat,
      fileTree: this.fileTree,
      search: this.search,
      skills: this.skills,
      diffHandlers,
      onSelectModel: (id) => this.selectModel(id),
      onNewChat: () => this.clearChat(),
      onNewFile: () => this.editor.createNewFile(),
      onOpenSettings: () => this.palette.open(),
    });
    this.agent = new ClaudeCodeLayout({
      diffHandlers,
      ...agentHandlers,
      onSend: (msg) => void this.sendMessage(msg),
      onClear: () => this.clearChat(),
      onShowEditor: () => void this.switchLayout('vs_code'),
    });
    this.zen = new CodexLayout({
      editor: this.editor,
      chat: this.chat,
      onSend: (msg) => void this.sendMessage(msg),
      onOpenPalette: () => this.palette.open(),
    });

    this.titleBar = new TitleBar({
      onLayout: (layout) => void this.switchLayout(layout),
      onCommandCenter: () => this.palette.open(),
      onOpenFolder: () => void this.openFolder(),
    });
    this.statusBar = new StatusBar({
      onProviderClick: () => {
        void this.switchLayout('vs_code');
        this.vscode.showView('models');
      },
      onChangesClick: () => this.showChanges(),
    });

    // Editor → dependent UI (no global re-render needed for tab/cursor changes).
    this.editor.onChange(() => {
      const tab = this.editor.getActiveTab();
      if (tab && !tab.isUntitled) this.fileTree.setActive(tab.path);
      this.fileTree.setModified(this.editor.getTabs().filter((t) => t.isModified).map((t) => t.path));
      this.chat.renderChips();
      this.zen.syncFile();
      document.title = tab ? `${tab.isModified ? '● ' : ''}${tab.name} — R.I.D.E` : 'R.I.D.E';
    });
    this.editor.onCursor((c) => {
      this.statusBar.setCursor(c);
      this.zen.setCursor(c.line, c.col);
    });

    this.layoutHost = document.createElement('main');
    this.layoutHost.className = 'layout-host';
    const shell = document.createElement('div');
    shell.className = 'app';
    shell.append(this.titleBar.getElement(), this.layoutHost, this.statusBar.getElement());
    root.appendChild(shell);

    this.palette.setCommands(this.buildCommands());
    this.registerShortcuts();
    this.mountLayout(this.state.activeLayout);
    this.render();
    void this.init();
  }

  // ── Startup ──────────────────────────────────────────────────────────────

  private async init(): Promise<void> {
    const [layout, workspace, skills, diffs] = await Promise.allSettled([
      api.getActiveLayout(),
      api.getWorkspaceInfo(),
      api.getActiveSkills(),
      api.getDiffPreview(),
    ]);

    if (layout.status === 'fulfilled' && layout.value !== this.state.activeLayout) {
      this.state.activeLayout = layout.value;
      this.mountLayout(layout.value);
    }
    if (workspace.status === 'fulfilled') this.setWorkspace(workspace.value.root, workspace.value.name);
    if (skills.status === 'fulfilled') {
      this.state.skills = skills.value;
      this.state.rev.skills++;
    }
    if (diffs.status === 'fulfilled') {
      this.state.diffs = diffs.value;
      this.state.rev.diffs++;
    }
    this.render();

    await this.fileTree.init();
    void this.openStartupFile();

    // Provider discovery probes several localhost ports; don't block first paint on it.
    void this.refreshProviders();
    void api.onModelEvent(() => void this.refreshProviders());
    void api.onAgentEvent((event) => this.handleAgentEvent(event));
    window.setInterval(() => {
      if (document.visibilityState === 'visible' && !this.state.isStreaming) void this.refreshProviders();
    }, PROVIDER_POLL_MS);

    window.addEventListener('beforeunload', (e) => {
      if (this.editor.hasUnsavedChanges()) e.preventDefault();
    });

    // Query parameters for screenshot automation and layout testing
    const urlParams = new URLSearchParams(window.location.search);
    const reqLayout = urlParams.get('layout') as UiLayout | null;
    if (reqLayout && (reqLayout === 'vs_code' || reqLayout === 'claude_code' || reqLayout === 'codex')) {
      void this.switchLayout(reqLayout);
    }
    const reqView = urlParams.get('view');
    if (reqView) {
      this.vscode.showView(reqView as any);
    }
    if (urlParams.get('mockChat') === '1') {
      this.state.messages = [
        {
          id: 'm1',
          role: 'user',
          content: 'Analyze crates/ride-moe expert router and verify zero-copy stream performance under local NVMe execution.',
          timestamp: Date.now() - 60000,
        },
        {
          id: 'm2',
          role: 'assistant',
          content: 'I analyzed `crates/ride-moe/src/router.rs` and the memory-mapped tensor pipeline.\n\n### Audit Summary\n1. **Zero-Copy Streaming**: Expert weights are loaded via `memmap2::Mmap` with aligned 64-byte chunks, avoiding heap allocations.\n2. **Sub-millisecond Latency**: Kernel dispatch is scheduled on dedicated Tokio async worker pools.\n3. **Local Privacy**: Model execution runs strictly on-device without external telemetry.',
          thoughts: [
            'Inspecting crates/ride-moe/src/router.rs for mmap borrow lifecycles',
            'Validating AVX2/AVX-512 tensor SIMD vectorization',
            'Testing local model dispatch via Ollama & ColibriMoE sockets',
          ],
          toolCalls: [
            {
              name: 'run_command',
              args: { command: 'cargo test -p ride-moe -- --nocapture' },
              output: 'test result: ok. 42 passed; 0 failed (0.038s)',
              status: 'success',
            },
          ],
          timestamp: Date.now() - 30000,
        },
      ];
      this.state.rev.messages++;
      this.render();
    }
  }

  private async openStartupFile(): Promise<void> {
    for (const candidate of ['crates/ride-desktop/src/main.rs', 'README.md', 'Cargo.toml']) {
      try {
        const content = await api.readFile(candidate);
        await this.editor.openFile(candidate, content);
        return;
      } catch {
        /* try next */
      }
    }
  }

  private setWorkspace(root: string, name: string): void {
    this.state.workspace = { root, name };
    this.titleBar.setWorkspace(name, root);
    this.fileTree.setRootName(name);
  }

  // ── Rendering (coalesced into one frame) ─────────────────────────────────

  private render(): void {
    if (this.renderQueued) return;
    this.renderQueued = true;
    requestAnimationFrame(() => {
      this.renderQueued = false;
      const s = this.state;
      this.titleBar.setLayout(s.activeLayout);
      this.statusBar.render(s);
      // Only the visible layout renders; the others catch up on mount.
      if (s.activeLayout === 'claude_code') this.agent.update(s);
      else if (s.activeLayout === 'codex') this.zen.update(s);
      else this.vscode.update(s);
    });
  }

  private mountLayout(layout: UiLayout): void {
    const target =
      layout === 'claude_code' ? this.agent : layout === 'codex' ? this.zen : this.vscode;
    this.layoutHost.replaceChildren(target.getElement());
    target.mount();
    target.update(this.state);
    this.titleBar.setLayout(layout);
  }

  public async switchLayout(layout: UiLayout): Promise<void> {
    if (this.state.activeLayout === layout) return;
    this.state.activeLayout = layout;
    this.mountLayout(layout);
    this.render();
    try {
      await api.setActiveLayout(layout);
    } catch (err) {
      console.warn('Could not persist layout preference:', err);
    }
  }

  // ── Providers & models ───────────────────────────────────────────────────

  private async refreshProviders(): Promise<void> {
    let providers: ProviderStatus[];
    try {
      providers = await api.getModelStatus();
    } catch (err) {
      console.debug('[model status] unavailable:', err);
      return;
    }
    const key = JSON.stringify(providers.map((p) => [p.provider, p.is_online, p.latency_ms, p.models.map((m) => m.id)]));
    if (key === this.providersKey) return; // nothing changed → no re-render
    this.providersKey = key;
    this.state.providers = providers;
    this.state.rev.providers++;

    const online = providers.filter((p) => p.is_online);
    const all = online.flatMap((p) => p.models.map((m) => ({ provider: p.provider, model: m })));
    const current = all.find((x) => x.model.id === this.state.currentModel || x.model.name === this.state.currentModel);
    if (current) {
      this.state.currentProvider = current.provider;
    } else if (all.length) {
      if (this.userPickedModel && this.state.currentModel) {
        toast(`${this.state.currentModel} is no longer available — switched to ${all[0].model.id}`, 'info', 3500);
      }
      this.state.currentModel = all[0].model.id;
      this.state.currentProvider = all[0].provider;
      this.userPickedModel = false;
    } else {
      this.state.currentModel = '';
      this.state.currentProvider = online[0]?.provider ?? 'Offline';
    }
    this.render();
  }

  private selectModel(id: string): void {
    const owner = this.state.providers.find((p) => p.is_online && p.models.some((m) => m.id === id));
    if (!owner) return;
    this.state.currentModel = id;
    this.state.currentProvider = owner.provider;
    this.userPickedModel = true;
    this.state.rev.providers++;
    this.render();
  }

  // ── Agent ────────────────────────────────────────────────────────────────

  private async sendMessage(content: string): Promise<void> {
    if (this.state.isStreaming) {
      toast('The agent is still working on the previous request.', 'info');
      return;
    }
    const now = Date.now();
    const assistant: ChatMessage = { id: `a-${now}`, role: 'assistant', content: '', thoughts: [], toolCalls: [], timestamp: now };
    this.state.messages.push({ id: `u-${now}`, role: 'user', content, timestamp: now }, assistant);
    this.state.isStreaming = true;
    this.state.rev.messages++;
    this.render();

    const started = performance.now();
    try {
      assistant.content = await api.sendAgentMessage(content, this.state.currentModel || undefined);
    } catch (err) {
      assistant.content = `The agent turn failed: ${err instanceof Error ? err.message : String(err)}`;
      assistant.isError = true;
    } finally {
      assistant.durationMs = performance.now() - started;
      // Any tool still "pending" when the turn ends did not report back, and
      // the backend denies approvals that were never answered.
      assistant.toolCalls?.forEach((tc) => {
        if (tc.status === 'pending') tc.status = 'failed';
      });
      assistant.approvals?.forEach((a) => {
        if (a.status === 'pending' || a.status === 'sending') a.status = 'denied';
      });
      this.state.isStreaming = false;
      this.state.rev.messages++;
      this.render();
      void this.syncDiffs();
    }
  }

  private handleAgentEvent(event: any): void {
    const last = this.state.messages[this.state.messages.length - 1];
    if (!last || last.role !== 'assistant') return;

    // Unit variants (e.g. `TurnCancelled`) arrive as bare strings.
    if (event === 'TurnCancelled') {
      last.cancelled = true;
      this.state.rev.messages++;
      this.render();
      return;
    }
    if (!event || typeof event !== 'object') return;

    if (event.ApprovalRequired) {
      const e = event.ApprovalRequired;
      const list = (last.approvals ??= []);
      if (!list.some((a) => a.id === e.request_id)) {
        list.push({ id: e.request_id, command: e.command, safety: e.safety_level, status: 'pending' });
      }
    } else if (event.ApprovalResponded) {
      const e = event.ApprovalResponded;
      const a = last.approvals?.find((x) => x.id === e.request_id);
      if (a) a.status = e.approved ? 'approved' : 'denied';
    } else if (event.ThinkingDelta) {
      (last.thoughts ??= []).push(String(event.ThinkingDelta.delta ?? ''));
    } else if (event.ToolCallRequested) {
      (last.toolCalls ??= []).push({
        name: event.ToolCallRequested.tool_name,
        args: event.ToolCallRequested.arguments,
        status: 'pending',
      });
    } else if (event.ToolExecuted) {
      const e = event.ToolExecuted;
      const calls = (last.toolCalls ??= []);
      const tc = calls.find((t) => t.name === e.tool_name && t.status === 'pending');
      const status = e.success ? 'success' : 'failed';
      if (tc) {
        tc.status = status;
        tc.output = e.output_preview;
      } else {
        calls.push({ name: e.tool_name, args: {}, output: e.output_preview, status });
      }
    } else if (event.DiffGenerated) {
      // The agent's write/edit tools have already written this to disk (with a
      // backup snapshot), so it is shown as applied and offered for revert.
      const { path, diff } = event.DiffGenerated as { path: string; diff: string };
      const norm = path.replace(/\\/g, '/');
      const stats = {
        diff_text: diff,
        additions: (diff.match(/^\+(?!\+\+)/gm) || []).length,
        deletions: (diff.match(/^-(?!--)/gm) || []).length,
      };
      const existing = this.state.diffs.find((d) => d.applied && d.path === norm);
      if (existing) {
        Object.assign(existing, stats);
        existing.edits = (existing.edits ?? 1) + 1;
      } else {
        this.state.diffs.push({ path: norm, ...stats, applied: true, edits: 1 });
      }
      this.state.rev.diffs++;
      void this.editor.reloadIfClean(norm);
    } else {
      return;
    }
    this.state.rev.messages++;
    this.render(); // coalesced: a burst of deltas costs one frame
  }

  private async respondApproval(id: string, approved: boolean): Promise<void> {
    const last = this.state.messages[this.state.messages.length - 1];
    const item = last?.approvals?.find((a) => a.id === id);
    if (!item || item.status !== 'pending') return;
    item.status = 'sending';
    this.state.rev.messages++;
    this.render();
    try {
      await api.respondApproval(id, approved);
      // The ApprovalResponded event normally lands first; this covers its loss.
      if (item.status === 'sending') item.status = approved ? 'approved' : 'denied';
    } catch (err) {
      item.status = 'denied';
      toast(String(err), 'err', 5000);
    }
    this.state.rev.messages++;
    this.render();
  }

  private async cancelTurn(): Promise<void> {
    if (!this.state.isStreaming) return;
    try {
      const wasRunning = await api.cancelAgentTurn();
      if (wasRunning) toast('Stopping the agent…', 'info', 2000);
    } catch (err) {
      toast(`Could not stop the agent: ${String(err)}`, 'err');
    }
  }

  private clearChat(): void {
    if (this.state.isStreaming) {
      toast('Wait for the current turn to finish before starting a new chat.', 'info');
      return;
    }
    this.state.messages = [];
    this.state.rev.messages++;
    this.render();
  }

  // ── Diffs & skills ───────────────────────────────────────────────────────

  private async syncDiffs(): Promise<void> {
    try {
      const staged = await api.getDiffPreview();
      // Keep event-only diffs that the backend has not staged yet.
      const merged = [...staged, ...this.state.diffs.filter((d) => !staged.some((s) => s.path === d.path))];
      this.state.diffs = merged;
      this.state.rev.diffs++;
      this.render();
    } catch {
      /* keep what we have */
    }
  }

  private async applyDiff(path: string): Promise<void> {
    try {
      await api.applyDiff(path);
      this.state.diffs = this.state.diffs.filter((d) => d.path !== path);
      this.state.rev.diffs++;
      toast(`Applied changes to ${path.split('/').pop()}`, 'ok');
      void this.editor.reloadIfClean(path);
    } catch (err) {
      toast(`Apply failed: ${String(err)}`, 'err', 5000);
      this.state.rev.diffs++;
    }
    this.render();
  }

  private async rollbackDiff(path: string): Promise<void> {
    const entry = this.state.diffs.find((d) => d.path === path);
    const name = path.split('/').pop();
    try {
      if (entry?.applied) {
        if (this.editor.isDirty(path)) {
          toast(`Save or close ${name} before reverting the agent's edit.`, 'info', 4000);
          return;
        }
        // Each agent edit left one snapshot; undo them newest-first.
        let reverted = 0;
        for (let i = 0; i < (entry.edits ?? 1); i++) {
          try {
            await api.rollbackDiff(path);
            reverted++;
          } catch (err) {
            if (reverted === 0) throw err;
            break; // older snapshots were evicted; keep what we restored
          }
        }
        toast(`Reverted the agent's edit to ${name}`, 'ok');
      } else {
        await api.rollbackDiff(path);
        toast(`Discarded changes to ${name}`, 'ok');
      }
      this.state.diffs = this.state.diffs.filter((d) => d.path !== path);
      void this.editor.reloadIfClean(path);
    } catch (err) {
      toast(`${entry?.applied ? 'Revert' : 'Discard'} failed: ${String(err)}`, 'err', 5000);
    } finally {
      this.state.rev.diffs++;
      this.render();
    }
  }

  /** Forget an agent-applied diff without touching the file. */
  private keepDiff(path: string): void {
    this.state.diffs = this.state.diffs.filter((d) => !(d.applied && d.path === path));
    this.state.rev.diffs++;
    this.render();
  }

  private async toggleSkill(name: string, enabled: boolean): Promise<void> {
    const skill = this.state.skills.find((s) => s.name === name);
    try {
      await api.toggleSkill(name, enabled);
      if (skill) skill.enabled = enabled;
    } catch (err) {
      toast(`Could not ${enabled ? 'enable' : 'disable'} ${name}: ${String(err)}`, 'err');
    }
    this.state.rev.skills++; // re-render either way so the switch reflects the real state
    this.render();
  }

  private showChanges(): void {
    if (this.state.activeLayout === 'vs_code') this.vscode.showChanges();
    else if (this.state.activeLayout === 'codex') {
      void this.switchLayout('vs_code').then(() => this.vscode.showChanges());
    }
  }

  // ── Files & workspace ────────────────────────────────────────────────────

  private async promptOpenFile(): Promise<void> {
    const path = await askText('Open file', {
      message: 'Path relative to the workspace root, or an absolute path.',
      placeholder: 'crates/ride-desktop/src/main.rs',
      confirmLabel: 'Open',
    });
    if (path) await this.editor.openFile(path.replace(/\\/g, '/'));
  }

  private async openFolder(): Promise<void> {
    if (this.editor.hasUnsavedChanges()) {
      toast('Save or close modified files before switching folders.', 'info', 3500);
      return;
    }
    const path = await askText('Open folder', {
      message: 'Absolute path of the folder to open as the workspace.',
      value: this.state.workspace.root,
      confirmLabel: 'Open folder',
    });
    if (!path) return;
    try {
      const info = await api.openWorkspace(path);
      this.setWorkspace(info.root, info.name);
      await this.fileTree.refresh();
      toast(`Opened ${info.name}`, 'ok');
    } catch (err) {
      toast(String(err), 'err', 5000);
    }
  }

  private applyTheme(theme: Theme): void {
    this.theme = theme;
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem('ride.theme', theme);
    } catch {
      /* ignore */
    }
  }

  // ── Commands & shortcuts ─────────────────────────────────────────────────

  private buildCommands(): CommandItem[] {
    const layout = (id: UiLayout) => () => void this.switchLayout(id);
    const inEditor = (fn: () => void) => () => {
      void this.switchLayout('vs_code').then(fn);
    };
    return [
      { id: 'file.open', group: 'File', label: 'Open File…', icon: 'files', keybinding: 'Ctrl+O', run: () => void this.promptOpenFile() },
      { id: 'file.new', group: 'File', label: 'New File', icon: 'fileAdd', keybinding: 'Ctrl+N', run: () => this.editor.createNewFile() },
      { id: 'file.save', group: 'File', label: 'Save', icon: 'save', keybinding: 'Ctrl+S', run: () => void this.editor.saveActiveFile() },
      { id: 'file.close', group: 'File', label: 'Close Editor', icon: 'close', keybinding: 'Ctrl+W', run: () => void this.editor.closeActiveTab() },
      { id: 'ws.open', group: 'File', label: 'Open Folder…', icon: 'folderOpen', run: () => void this.openFolder() },
      { id: 'ws.search', group: 'File', label: 'Search in Workspace', icon: 'search', keybinding: 'Ctrl+Shift+F', run: inEditor(() => this.vscode.showView('search')) },
      { id: 'view.vscode', group: 'View', label: 'Layout: Editor', sublabel: 'Explorer, tabs and an agent panel', icon: 'layout', keybinding: 'Ctrl+1', run: layout('vs_code') },
      { id: 'view.agent', group: 'View', label: 'Layout: Agent', sublabel: 'Terminal-first reasoning stream and change review', icon: 'terminal', keybinding: 'Ctrl+2', run: layout('claude_code') },
      { id: 'view.zen', group: 'View', label: 'Layout: Zen', sublabel: 'Distraction-free canvas with inline Ctrl+K edits', icon: 'sparkle', keybinding: 'Ctrl+3', run: layout('codex') },
      { id: 'view.sidebar', group: 'View', label: 'Toggle Side Bar', icon: 'panelLeft', keybinding: 'Ctrl+B', run: inEditor(() => this.vscode.toggleSidebar()) },
      { id: 'view.agentPanel', group: 'View', label: 'Toggle Agent Panel', icon: 'panelRight', keybinding: 'Ctrl+J', run: inEditor(() => this.vscode.toggleAgentPanel()) },
      { id: 'view.theme', group: 'View', label: 'Toggle Light/Dark Theme', icon: 'sun', run: () => this.applyTheme(this.theme === 'dark' ? 'light' : 'dark') },
      { id: 'agent.new', group: 'Agent', label: 'New Chat', icon: 'plus', run: () => this.clearChat() },
      { id: 'agent.stop', group: 'Agent', label: 'Stop Agent Turn', icon: 'x', run: () => void this.cancelTurn() },
      { id: 'agent.changes', group: 'Agent', label: 'Review Pending Changes', icon: 'diff', run: () => this.showChanges() },
      { id: 'agent.models', group: 'Agent', label: 'Show Local Models', icon: 'chip', run: inEditor(() => this.vscode.showView('models')) },
      { id: 'agent.skills', group: 'Agent', label: 'Manage Skills', icon: 'grid', run: inEditor(() => this.vscode.showView('skills')) },
      { id: 'agent.rescan', group: 'Agent', label: 'Rescan Local Model Servers', icon: 'refresh', run: () => { this.providersKey = ''; void this.refreshProviders(); } },
    ];
  }

  private registerShortcuts(): void {
    window.addEventListener('keydown', (e) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      if (document.querySelector('.dialog-overlay')) return;
      const key = e.key.toLowerCase();
      const shift = e.shiftKey;
      const handled = (fn: () => void) => {
        e.preventDefault();
        e.stopPropagation();
        fn();
      };

      if (key === 'k' && !shift) {
        handled(() => (this.state.activeLayout === 'codex' && !this.palette.isOpen ? this.zen.openInlinePrompt() : this.palette.toggle()));
      } else if (key === 'p') {
        handled(() => this.palette.toggle());
      } else if (key === 's' && !shift) {
        handled(() => void this.editor.saveActiveFile());
      } else if (key === 'o' && !shift) {
        handled(() => void this.promptOpenFile());
      } else if (key === 'n' && !shift) {
        handled(() => this.editor.createNewFile());
      } else if (key === 'w' && !shift) {
        handled(() => void this.editor.closeActiveTab());
      } else if (key === 'b' && !shift && this.state.activeLayout === 'vs_code') {
        handled(() => this.vscode.toggleSidebar());
      } else if (key === 'j' && !shift && this.state.activeLayout === 'vs_code') {
        handled(() => this.vscode.toggleAgentPanel());
      } else if (key === 'e' && shift) {
        handled(() => void this.switchLayout('vs_code').then(() => this.vscode.showView('explorer')));
      } else if (key === 'f' && shift) {
        handled(() => void this.switchLayout('vs_code').then(() => this.vscode.showView('search')));
      } else if (key === 'l' && !shift && this.state.activeLayout === 'vs_code') {
        handled(() => {
          this.vscode.toggleAgentPanel(true);
          this.chat.focusInput();
        });
      } else if (e.key === '1' || e.key === '2' || e.key === '3') {
        const map: Record<string, UiLayout> = { '1': 'vs_code', '2': 'claude_code', '3': 'codex' };
        handled(() => void this.switchLayout(map[e.key]));
      }
    }, true);
  }
}

window.addEventListener('DOMContentLoaded', () => {
  const root = document.getElementById('app');
  if (!root) throw new Error('#app root element not found');
  new RideApplication(root);
});
