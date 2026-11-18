import type { AppState } from '../types';
import { EditorComponent } from '../components/Editor';
import { AgentChat } from '../components/AgentChat';
import { CommandPalette } from '../components/CommandPalette';

export class CodexLayout {
  private element: HTMLElement;
  private editor: EditorComponent;
  private chat: AgentChat;
  private commandPalette: CommandPalette;
  private isAssistantOpen: boolean = false;

  constructor(
    editor: EditorComponent,
    chat: AgentChat,
    commandPalette: CommandPalette
  ) {
    this.editor = editor;
    this.chat = chat;
    this.commandPalette = commandPalette;

    this.element = document.createElement('div');
    this.element.className = 'layout-codex';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public mount(): void {
    const editorWrapper = this.element.querySelector<HTMLElement>('.zen-editor-wrapper');
    if (editorWrapper && !editorWrapper.contains(this.editor.getElement())) {
      editorWrapper.innerHTML = '';
      editorWrapper.appendChild(this.editor.getElement());
    }

    const assistantContent = this.element.querySelector<HTMLElement>('.assistant-content');
    if (assistantContent && !assistantContent.contains(this.chat.getElement())) {
      assistantContent.innerHTML = '';
      assistantContent.appendChild(this.chat.getElement());
    }
  }

  public update(state: AppState): void {
    this.chat.updateMessages(state.messages);
    const docEl = this.element.querySelector<HTMLElement>('.hud-doc');
    if (docEl) {
      const activeTab = this.editor.getActiveTab();
      docEl.innerText = activeTab ? activeTab.name : 'Editor';
    }
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="codex-zen-container">
        <!-- Floating HUD Header -->
        <div class="codex-floating-hud">
          <div class="hud-left">
            <span class="hud-brand">✧ Codex Zen</span>
            <span class="hud-dot"></span>
            <span class="hud-doc">main.rs</span>
          </div>
          <div class="hud-center">
            <button class="hud-cmd-btn" title="Open Command Palette (Ctrl+K)">
              <span class="hud-icon">⌘</span>
              <span class="hud-text">Ask R.I.D.E AI...</span>
              <span class="hud-badge">Ctrl+K</span>
            </button>
          </div>
          <div class="hud-right">
            <button class="hud-toggle-assistant" title="Toggle AI Side Drawer">
              <span class="icon">🤖</span>
              <span class="label">Assistant</span>
            </button>
          </div>
        </div>

        <!-- Full-bleed Zen Editor Canvas -->
        <div class="codex-canvas">
          <div class="zen-editor-wrapper"></div>
        </div>

        <!-- Floating Collapsible Side Assistant -->
        <div class="codex-side-assistant ${this.isAssistantOpen ? 'open' : ''}">
          <div class="assistant-header">
            <div class="assistant-title">
              <span class="icon">✧</span>
              <span>Zen AI Companion</span>
            </div>
            <button class="assistant-close-btn" title="Close Drawer">✕</button>
          </div>
          <div class="assistant-content"></div>
        </div>
      </div>
    `;

    this.mount();
    this.attachEvents();
  }

  private attachEvents(): void {
    const cmdBtn = this.element.querySelector<HTMLButtonElement>('.hud-cmd-btn');
    if (cmdBtn) {
      cmdBtn.addEventListener('click', () => {
        this.commandPalette.open();
      });
    }

    const toggleBtn = this.element.querySelector<HTMLButtonElement>('.hud-toggle-assistant');
    const closeBtn = this.element.querySelector<HTMLButtonElement>('.assistant-close-btn');
    const drawer = this.element.querySelector<HTMLElement>('.codex-side-assistant');

    const toggleAssistant = () => {
      this.isAssistantOpen = !this.isAssistantOpen;
      if (drawer) {
        drawer.classList.toggle('open', this.isAssistantOpen);
      }
    };

    if (toggleBtn) {
      toggleBtn.addEventListener('click', toggleAssistant);
    }
    if (closeBtn) {
      closeBtn.addEventListener('click', toggleAssistant);
    }
  }
}
