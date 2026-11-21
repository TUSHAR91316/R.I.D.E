import type { AppState } from '../types';
import type { EditorComponent } from '../components/Editor';
import type { AgentChat } from '../components/AgentChat';
import type { FileTree } from '../components/FileTree';
import type { SearchPanel } from '../components/SearchPanel';
import type { SkillsManager } from '../components/SkillsManager';
import { DiffViewer, type DiffHandlers } from '../components/DiffViewer';
import { icon, type IconName } from '../icons';
import { esc } from '../util';
import { attachSplitter } from './splitter';

type View = 'explorer' | 'search' | 'skills' | 'models';
type DockTab = 'chat' | 'changes';

export interface VsCodeDeps {
  editor: EditorComponent;
  chat: AgentChat;
  fileTree: FileTree;
  search: SearchPanel;
  skills: SkillsManager;
  diffHandlers: DiffHandlers;
  onSelectModel: (modelId: string) => void;
  onNewChat: () => void;
  onNewFile: () => void;
  onOpenSettings: () => void;
}

const VIEWS: Array<{ id: View; label: string; icon: IconName; key?: string }> = [
  { id: 'explorer', label: 'Explorer', icon: 'files', key: 'Ctrl+Shift+E' },
  { id: 'search', label: 'Search', icon: 'search', key: 'Ctrl+Shift+F' },
  { id: 'skills', label: 'Skills', icon: 'grid' },
  { id: 'models', label: 'Local models', icon: 'chip' },
];

export class VsCodeLayout {
  private element: HTMLElement;
  private deps: VsCodeDeps;
  private diffViewer: DiffViewer;
  private view: View = 'explorer';
  private dockTab: DockTab = 'chat';
  private sidebarEl!: HTMLElement;
  private agentEl!: HTMLElement;
  private sideBody!: HTMLElement;
  private sideTitle!: HTMLElement;
  private sideActions!: HTMLElement;
  private dockBody!: HTMLElement;
  private editorHost!: HTMLElement;
  private modelsEl: HTMLElement;
  private modelsMini!: HTMLElement;
  private lastProvidersRev = -1;
  private lastModel = '';

  constructor(deps: VsCodeDeps) {
    this.deps = deps;
    this.diffViewer = new DiffViewer(deps.diffHandlers);
    this.modelsEl = document.createElement('div');
    this.modelsEl.className = 'models-panel';
    this.modelsEl.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest<HTMLElement>('[data-model]');
      if (row) deps.onSelectModel(row.dataset.model!);
    });

    this.element = document.createElement('div');
    this.element.className = 'ws';
    this.renderSkeleton();
    this.showView('explorer');
    this.setDockTab('chat');
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  /** Re-attach shared components (the Zen layout borrows the editor and chat). */
  public mount(): void {
    if (!this.editorHost.contains(this.deps.editor.getElement())) {
      this.editorHost.appendChild(this.deps.editor.getElement());
    }
    this.setDockTab(this.dockTab);
  }

  public update(state: AppState): void {
    this.diffViewer.render(state.diffs, state.rev.diffs);
    this.deps.skills.render(state.skills, state.rev.skills);
    this.deps.chat.update(state); // revision-guarded: cheap when nothing changed

    const badge = this.element.querySelector<HTMLElement>('[data-dock="changes"] .badge')!;
    badge.textContent = String(state.diffs.length);
    badge.hidden = state.diffs.length === 0;

    if (state.rev.providers !== this.lastProvidersRev || state.currentModel !== this.lastModel) {
      this.lastProvidersRev = state.rev.providers;
      this.lastModel = state.currentModel;
      this.renderModels(state);
    }
  }

  public toggleSidebar(force?: boolean): void {
    this.sidebarEl.hidden = force === undefined ? !this.sidebarEl.hidden : !force;
    this.syncSplitters();
  }

  public toggleAgentPanel(force?: boolean): void {
    this.agentEl.hidden = force === undefined ? !this.agentEl.hidden : !force;
    this.syncSplitters();
    this.element.querySelector('[data-act="toggle-agent"]')?.classList.toggle('on', !this.agentEl.hidden);
    if (!this.agentEl.hidden && this.dockTab === 'chat') this.deps.chat.focusInput();
  }

  public showView(view: View): void {
    this.view = view;
    this.toggleSidebar(true);
    this.element.querySelectorAll<HTMLElement>('.ab-btn[data-view]').forEach((b) => {
      const on = b.dataset.view === view;
      b.classList.toggle('active', on);
      b.setAttribute('aria-pressed', String(on));
    });
    const meta = VIEWS.find((v) => v.id === view)!;
    this.sideTitle.textContent = meta.label;
    this.sideActions.innerHTML =
      view === 'explorer'
        ? `<button type="button" class="icon-btn" data-act="new-file" title="New file" aria-label="New file">${icon('fileAdd')}</button>
           <button type="button" class="icon-btn" data-act="refresh" title="Refresh explorer" aria-label="Refresh explorer">${icon('refresh')}</button>
           <button type="button" class="icon-btn" data-act="collapse" title="Collapse folders" aria-label="Collapse folders">${icon('collapse')}</button>`
        : '';

    const content =
      view === 'explorer'
        ? this.deps.fileTree.getElement()
        : view === 'search'
          ? this.deps.search.getElement()
          : view === 'skills'
            ? this.deps.skills.getElement()
            : this.modelsEl;
    if (this.sideBody.firstElementChild !== content) this.sideBody.replaceChildren(content);
    if (view === 'search') this.deps.search.focus();
  }

  public showChanges(): void {
    this.toggleAgentPanel(true);
    this.setDockTab('changes');
  }

  private setDockTab(tab: DockTab): void {
    this.dockTab = tab;
    this.element.querySelectorAll<HTMLElement>('[data-dock]').forEach((b) => b.setAttribute('aria-selected', String(b.dataset.dock === tab)));
    const content = tab === 'chat' ? this.deps.chat.getElement() : this.diffViewer.getElement();
    if (this.dockBody.firstElementChild !== content) this.dockBody.replaceChildren(content);
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <nav class="activitybar" aria-label="Views">
        ${VIEWS.map((v) => `<button type="button" class="ab-btn" data-view="${v.id}" title="${v.label}${v.key ? ` (${v.key})` : ''}" aria-label="${v.label}">${icon(v.icon)}</button>`).join('')}
        <span class="ab-spacer"></span>
        <button type="button" class="ab-btn" data-act="settings" title="Commands & settings (Ctrl+K)" aria-label="Commands and settings">${icon('sliders')}</button>
      </nav>
      <aside class="sidebar" aria-label="Side bar">
        <div class="pane-header"><span class="pane-title"></span><span class="pane-actions" style="display:flex;gap:2px"></span></div>
        <div class="pane-body"></div>
        <div class="models-mini" aria-label="Local model status"></div>
      </aside>
      <div class="splitter" data-split="side" aria-label="Resize side bar"></div>
      <section class="editor-area" aria-label="Editor"></section>
      <div class="splitter" data-split="agent" aria-label="Resize agent panel"></div>
      <aside class="agent-panel" aria-label="Agent">
        <div class="pane-header">
          <div class="panel-tabs" role="tablist" aria-label="Agent panel">
            <button type="button" class="tab-btn" role="tab" data-dock="chat">Chat</button>
            <button type="button" class="tab-btn" role="tab" data-dock="changes">Changes<span class="badge" hidden>0</span></button>
          </div>
          <span style="flex:1"></span>
          <button type="button" class="icon-btn" data-act="new-chat" title="New chat" aria-label="New chat">${icon('plus')}</button>
          <button type="button" class="icon-btn" data-act="close-agent" title="Hide panel (Ctrl+J)" aria-label="Hide agent panel">${icon('close')}</button>
        </div>
        <div class="pane-body"></div>
      </aside>
    `;
    this.sidebarEl = this.element.querySelector('.sidebar')!;
    this.agentEl = this.element.querySelector('.agent-panel')!;
    this.sideBody = this.sidebarEl.querySelector('.pane-body')!;
    this.sideTitle = this.sidebarEl.querySelector('.pane-title')!;
    this.sideActions = this.sidebarEl.querySelector('.pane-actions')!;
    this.modelsMini = this.sidebarEl.querySelector('.models-mini')!;
    this.dockBody = this.agentEl.querySelector('.pane-body')!;
    this.editorHost = this.element.querySelector('.editor-area')!;
    this.editorHost.appendChild(this.deps.editor.getElement());

    // Agent-panel toggle sits in the editor tab bar, like VS Code's secondary side bar button.
    const tabActions = this.deps.editor.getElement().querySelector('.tab-actions');
    if (tabActions && !tabActions.querySelector('[data-act="toggle-agent"]')) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'icon-btn on';
      btn.dataset.act = 'toggle-agent';
      btn.title = 'Toggle agent panel (Ctrl+J)';
      btn.setAttribute('aria-label', 'Toggle agent panel');
      btn.innerHTML = icon('panelRight');
      btn.addEventListener('click', () => this.toggleAgentPanel());
      tabActions.appendChild(btn);
    }

    attachSplitter({
      handle: this.element.querySelector('[data-split="side"]')!,
      pane: this.sidebarEl,
      host: this.element,
      cssVar: '--sidebar-w',
      storageKey: 'ride.sidebarWidth',
      side: 'right',
      min: 180,
      max: 600,
      fallback: 260,
    });
    attachSplitter({
      handle: this.element.querySelector('[data-split="agent"]')!,
      pane: this.agentEl,
      host: this.element,
      cssVar: '--agent-w',
      storageKey: 'ride.agentWidth',
      side: 'left',
      min: 300,
      max: 720,
      fallback: 384,
    });

    this.element.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      const viewBtn = target.closest<HTMLElement>('.ab-btn[data-view]');
      if (viewBtn) {
        const v = viewBtn.dataset.view as View;
        // Clicking the active view collapses the side bar, as in VS Code.
        if (v === this.view && !this.sidebarEl.hidden) this.toggleSidebar(false);
        else this.showView(v);
        return;
      }
      const dock = target.closest<HTMLElement>('[data-dock]')?.dataset.dock as DockTab | undefined;
      if (dock) {
        this.setDockTab(dock);
        return;
      }
      switch (target.closest<HTMLElement>('[data-act]')?.dataset.act) {
        case 'new-file':
          this.deps.onNewFile();
          break;
        case 'refresh':
          void this.deps.fileTree.refresh();
          break;
        case 'collapse':
          this.deps.fileTree.collapseAll();
          break;
        case 'new-chat':
          this.deps.onNewChat();
          this.setDockTab('chat');
          break;
        case 'close-agent':
          this.toggleAgentPanel(false);
          break;
        case 'settings':
          this.deps.onOpenSettings();
          break;
        case 'models':
          this.showView('models');
          break;
      }
    });
  }

  private syncSplitters(): void {
    (this.element.querySelector('[data-split="side"]') as HTMLElement).hidden = this.sidebarEl.hidden;
    (this.element.querySelector('[data-split="agent"]') as HTMLElement).hidden = this.agentEl.hidden;
  }

  private renderModels(state: AppState): void {
    this.modelsMini.innerHTML =
      `<button type="button" class="section-label" data-act="models" style="border:0;background:none;padding:0;text-align:left">Local models</button>` +
      (state.providers.length
        ? state.providers
            .map(
              (p) =>
                `<div class="row${p.is_online ? '' : ' off'}"><span class="dot ${p.is_online ? 'on' : 'off'}"></span><span class="grow">${esc(p.provider)}</span><span class="meta">${p.is_online ? (p.latency_ms != null ? `${p.latency_ms} ms` : 'online') : 'offline'}</span></div>`
            )
            .join('')
        : `<div class="row off"><span class="dot off"></span><span class="grow">Scanning localhost…</span></div>`);

    this.modelsEl.innerHTML = state.providers.length
      ? state.providers
          .map(
            (p) => `<div class="provider">
              <div class="provider-head"><span class="dot ${p.is_online ? 'on' : 'off'}"></span><strong>${esc(p.provider)}</strong><span class="meta mono" style="font-size:11px;color:var(--text-muted)">${p.is_online ? (p.latency_ms != null ? `${p.latency_ms} ms` : 'online') : 'offline'}</span></div>
              <div class="provider-url">${esc(p.base_url)}</div>
              ${
                p.models.length
                  ? p.models
                      .map(
                        (m) => `<button type="button" class="model-row${m.id === state.currentModel || m.name === state.currentModel ? ' selected' : ''}" data-model="${esc(m.id)}" ${p.is_online ? '' : 'disabled'}>
                          ${icon(m.is_moe ? 'layout' : 'zap')}<span class="grow">${esc(m.name || m.id)}</span>
                          ${m.is_moe ? '<span class="chip">MoE</span>' : ''}
                          ${m.context_length ? `<span class="ctx">${Math.round(m.context_length / 1024)}k</span>` : ''}
                        </button>`
                      )
                      .join('')
                  : `<div class="provider-empty">${p.is_online ? 'No models loaded' : 'Not running'}</div>`
              }
            </div>`
          )
          .join('')
      : `<div class="empty-note">Scanning localhost for Ollama, LM Studio, llama.cpp, vLLM and Rabbit…</div>`;
  }
}
