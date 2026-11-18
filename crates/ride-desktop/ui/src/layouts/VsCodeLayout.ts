import type { AppState } from '../types';
import { EditorComponent } from '../components/Editor';
import { AgentChat } from '../components/AgentChat';
import { DiffViewer } from '../components/DiffViewer';
import { SkillsManager } from '../components/SkillsManager';

export class VsCodeLayout {
  private element: HTMLElement;
  private editor: EditorComponent;
  private chat: AgentChat;
  private diffViewer: DiffViewer;
  private skillsManager: SkillsManager;
  private activeActivity: 'explorer' | 'search' | 'skills' | 'models' = 'explorer';
  private activeDockTab: 'chat' | 'diffs' = 'chat';
  private isDockCollapsed: boolean = false;
  private currentState: AppState | null = null;

  constructor(
    editor: EditorComponent,
    chat: AgentChat,
    diffViewer: DiffViewer,
    skillsManager: SkillsManager
  ) {
    this.editor = editor;
    this.chat = chat;
    this.diffViewer = diffViewer;
    this.skillsManager = skillsManager;

    this.element = document.createElement('div');
    this.element.className = 'layout-vscode';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public mount(): void {
    const editorPane = this.element.querySelector<HTMLElement>('.editor-pane');
    if (editorPane && !editorPane.contains(this.editor.getElement())) {
      editorPane.innerHTML = '';
      editorPane.appendChild(this.editor.getElement());
    }
    this.updateDockContent();
    if (this.activeActivity === 'skills') {
      const sidebarContent = this.element.querySelector<HTMLElement>('.sidebar-content');
      if (sidebarContent && !sidebarContent.contains(this.skillsManager.getElement())) {
        sidebarContent.innerHTML = '';
        sidebarContent.appendChild(this.skillsManager.getElement());
      }
    }
  }

  public update(state: AppState): void {
    this.currentState = state;
    this.diffViewer.render(state.diffs);
    this.skillsManager.render(state.skills);
    this.chat.updateMessages(state.messages);
    this.updateSideBarContent(state);

    const diffBadge = this.element.querySelector<HTMLElement>('.dock-tab[data-tab="diffs"] .dock-badge');
    if (diffBadge) {
      diffBadge.innerText = String(state.diffs.length);
      diffBadge.style.display = state.diffs.length > 0 ? 'inline-block' : 'none';
    }
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="vscode-body">
        <!-- 1. Left Activity Bar -->
        <div class="activity-bar">
          <div class="activity-top">
            <button class="activity-btn ${this.activeActivity === 'explorer' ? 'active' : ''}" data-activity="explorer" title="Explorer (Ctrl+Shift+E)">
              <span class="icon">📁</span>
            </button>
            <button class="activity-btn ${this.activeActivity === 'search' ? 'active' : ''}" data-activity="search" title="Search Workspace (Ctrl+Shift+F)">
              <span class="icon">🔍</span>
            </button>
            <button class="activity-btn ${this.activeActivity === 'skills' ? 'active' : ''}" data-activity="skills" title="Skills & Tools">
              <span class="icon">🧩</span>
            </button>
            <button class="activity-btn ${this.activeActivity === 'models' ? 'active' : ''}" data-activity="models" title="Model Providers">
              <span class="icon">🧠</span>
            </button>
          </div>
          <div class="activity-bottom">
            <button class="activity-btn" title="Settings">
              <span class="icon">⚙️</span>
            </button>
          </div>
        </div>

        <!-- 2. Primary Side Bar -->
        <div class="primary-sidebar">
          <div class="sidebar-header">
            <span class="sidebar-title">EXPLORER</span>
          </div>
          <div class="sidebar-content"></div>
        </div>

        <!-- 3. Central Working Area: Editor + Agent Dock -->
        <div class="main-editor-area">
          <div class="editor-pane"></div>
          <div class="dock-resizer" title="Drag or click to toggle dock"></div>
          <div class="agent-dock ${this.isDockCollapsed ? 'collapsed' : ''}">
            <div class="dock-header">
              <div class="dock-tabs">
                <button class="dock-tab ${this.activeDockTab === 'chat' ? 'active' : ''}" data-tab="chat">
                  <span>🤖 Agent Dock</span>
                </button>
                <button class="dock-tab ${this.activeDockTab === 'diffs' ? 'active' : ''}" data-tab="diffs">
                  <span>± Changes</span>
                  <span class="dock-badge" style="display: none;">0</span>
                </button>
              </div>
              <div class="dock-controls">
                <button class="dock-toggle-btn" title="Toggle Dock">─</button>
              </div>
            </div>
            <div class="dock-content"></div>
          </div>
        </div>
      </div>
    `;

    // Mount Editor
    const editorPane = this.element.querySelector<HTMLElement>('.editor-pane');
    if (editorPane) {
      editorPane.appendChild(this.editor.getElement());
    }

    // Mount Dock Content
    this.updateDockContent();

    this.attachEvents();
  }

  private updateDockContent(): void {
    const dockContent = this.element.querySelector<HTMLElement>('.dock-content');
    if (!dockContent) return;
    dockContent.innerHTML = '';

    if (this.activeDockTab === 'chat') {
      dockContent.appendChild(this.chat.getElement());
    } else {
      dockContent.appendChild(this.diffViewer.getElement());
    }
  }

  private updateSideBarContent(state: AppState): void {
    const sidebarTitle = this.element.querySelector<HTMLElement>('.sidebar-title');
    const sidebarContent = this.element.querySelector<HTMLElement>('.sidebar-content');
    if (!sidebarContent || !sidebarTitle) return;

    if (this.activeActivity === 'explorer') {
      sidebarTitle.innerText = 'WORKSPACE EXPLORER';
      sidebarContent.innerHTML = `
        <div class="explorer-tree">
          <div class="tree-section">
            <div class="tree-header">▾ R.I.D.E (Workspace)</div>
            <div class="tree-item indent-1">📁 crates</div>
            <div class="tree-item indent-2">📁 ride-core</div>
            <div class="tree-item indent-2">📁 ride-discovery</div>
            <div class="tree-item indent-2">📁 ride-moe</div>
            <div class="tree-item indent-2">📁 ride-skills</div>
            <div class="tree-item indent-2">📁 ride-agent</div>
            <div class="tree-item indent-2 open">▾ 📁 ride-desktop</div>
            <div class="tree-item indent-3 file active" data-path="crates/ride-desktop/src/main.rs">🦀 main.rs</div>
            <div class="tree-item indent-3 file" data-path="crates/ride-desktop/src/commands.rs">🦀 commands.rs</div>
            <div class="tree-item indent-3 file" data-path="crates/ride-desktop/Cargo.toml">⚙️ Cargo.toml</div>
            <div class="tree-item indent-3 file" data-path="crates/ride-desktop/tauri.conf.json">⚙️ tauri.conf.json</div>
            <div class="tree-item indent-1 file" data-path="README.md">📄 README.md</div>
            <div class="tree-item indent-1 file" data-path="Cargo.toml">⚙️ Cargo.toml</div>
          </div>
        </div>
      `;

      // Attach file click handler
      sidebarContent.querySelectorAll<HTMLElement>('.tree-item.file[data-path]').forEach((item) => {
        item.addEventListener('click', () => {
          const path = item.getAttribute('data-path');
          if (path) {
            sidebarContent.querySelectorAll('.tree-item.file').forEach((f) => f.classList.remove('active'));
            item.classList.add('active');
            void this.editor.openFile(path);
          }
        });
      });
    } else if (this.activeActivity === 'search') {
      sidebarTitle.innerText = 'SEARCH WORKSPACE';
      sidebarContent.innerHTML = `
        <div class="search-panel">
          <input type="text" class="search-input" placeholder="Search in files... (regex allowed)" />
          <div class="search-hint">Press Enter to search entire workspace</div>
        </div>
      `;
    } else if (this.activeActivity === 'skills') {
      sidebarTitle.innerText = 'SKILLS REGISTRY';
      sidebarContent.innerHTML = '';
      sidebarContent.appendChild(this.skillsManager.getElement());
    } else if (this.activeActivity === 'models') {
      sidebarTitle.innerText = 'LOCAL MODELS & PROVIDERS';
      sidebarContent.innerHTML = `
        <div class="models-panel">
          ${state.providers
            .map(
              (p) => `
            <div class="provider-card ${p.is_online ? 'online' : 'offline'}">
              <div class="p-header">
                <span class="p-dot"></span>
                <strong>${p.provider}</strong>
                <span class="p-status">${p.is_online ? `${p.latency_ms || 10}ms` : 'Offline'}</span>
              </div>
              <div class="p-url">${p.base_url}</div>
              <div class="p-models">
                ${
                  p.models.length > 0
                    ? p.models
                        .map(
                          (m) => `
                  <div class="model-row ${m.id === state.currentModel ? 'selected' : ''}">
                    <span class="m-icon">${m.is_moe ? '🔀' : '⚡'}</span>
                    <span class="m-name">${m.name || m.id}</span>
                    ${m.is_moe ? '<span class="m-moe">MoE</span>' : ''}
                  </div>`
                        )
                        .join('')
                    : '<div class="p-empty">No active models detected</div>'
                }
              </div>
            </div>`
            )
            .join('')}
        </div>
      `;
    }
  }

  private attachEvents(): void {
    const activityButtons = this.element.querySelectorAll<HTMLButtonElement>('.activity-btn[data-activity]');
    activityButtons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const act = btn.getAttribute('data-activity') as any;
        if (act) {
          this.activeActivity = act;
          activityButtons.forEach((b) => b.classList.remove('active'));
          btn.classList.add('active');
          if (this.currentState) {
            this.updateSideBarContent(this.currentState);
          }
        }
      });
    });

    const dockTabs = this.element.querySelectorAll<HTMLButtonElement>('.dock-tab');
    dockTabs.forEach((tab) => {
      tab.addEventListener('click', () => {
        const tabKey = tab.getAttribute('data-tab') as 'chat' | 'diffs';
        if (tabKey) {
          this.activeDockTab = tabKey;
          dockTabs.forEach((t) => t.classList.remove('active'));
          tab.classList.add('active');
          this.updateDockContent();
        }
      });
    });

    const toggleBtn = this.element.querySelector<HTMLButtonElement>('.dock-toggle-btn');
    const dockEl = this.element.querySelector<HTMLElement>('.agent-dock');
    if (toggleBtn && dockEl) {
      toggleBtn.addEventListener('click', () => {
        this.isDockCollapsed = !this.isDockCollapsed;
        dockEl.classList.toggle('collapsed', this.isDockCollapsed);
        toggleBtn.innerText = this.isDockCollapsed ? '▢' : '─';
      });
    }
  }
}
