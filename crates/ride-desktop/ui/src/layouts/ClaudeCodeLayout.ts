import type { AppState, ChatMessage } from '../types';
import { DiffViewer } from '../components/DiffViewer';

export class ClaudeCodeLayout {
  private element: HTMLElement;
  private diffViewer: DiffViewer;
  private onSendMessage: (msg: string) => Promise<void>;
  private isBusy: boolean = false;
  private history: string[] = [];
  private historyIndex: number = -1;

  constructor(
    diffViewer: DiffViewer,
    onSendMessage: (msg: string) => Promise<void>
  ) {
    this.diffViewer = diffViewer;
    this.onSendMessage = onSendMessage;
    this.element = document.createElement('div');
    this.element.className = 'layout-claude-code';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public mount(): void {
    const drawerContent = this.element.querySelector<HTMLElement>('.drawer-content');
    if (drawerContent && !drawerContent.contains(this.diffViewer.getElement())) {
      drawerContent.innerHTML = '';
      drawerContent.appendChild(this.diffViewer.getElement());
    }
  }

  public update(state: AppState): void {
    this.diffViewer.render(state.diffs);
    this.renderTerminalStream(state.messages, state);
    this.updateStatusPill(state);
  }

  private updateStatusPill(state: AppState): void {
    const pill = this.element.querySelector<HTMLElement>('.terminal-status-pill');
    const txt = this.element.querySelector<HTMLElement>('.status-txt');
    if (!pill || !txt) return;

    const isOnline = state.providers.some((p) => p.is_online);
    pill.classList.toggle('offline', !isOnline);
    pill.classList.toggle('online', isOnline);
    if (isOnline) {
      txt.innerText = `${state.currentProvider} online`;
    } else {
      txt.innerText = 'offline mode';
    }
  }

  public setBusy(busy: boolean): void {
    this.isBusy = busy;
    const input = this.element.querySelector<HTMLInputElement>('.cli-input');
    const promptIndicator = this.element.querySelector<HTMLElement>('.cli-prompt-status');
    if (input) input.disabled = busy;
    if (promptIndicator) {
      promptIndicator.innerText = busy ? '⚡ executing...' : '❯';
      promptIndicator.classList.toggle('busy', busy);
    }
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="claude-terminal-window">
        <!-- Terminal Header -->
        <div class="terminal-header">
          <div class="terminal-dots">
            <span class="dot red"></span>
            <span class="dot yellow"></span>
            <span class="dot green"></span>
          </div>
          <div class="terminal-title">
            <span class="term-icon">❯_</span>
            <span>claude-code session — R.I.D.E Autonomous Agent (Terminal-First)</span>
          </div>
          <div class="terminal-status-pill offline">
            <span class="pulse-dot"></span>
            <span class="status-txt">offline mode</span>
          </div>
        </div>

        <!-- Terminal Output Stream -->
        <div class="terminal-stream-area"></div>

        <!-- Inline Diffs Drawer (if diffs are pending) -->
        <div class="terminal-diffs-drawer" style="display: none;">
          <div class="drawer-header">
            <span class="drawer-icon">±</span>
            <span class="drawer-title">Pending Diff Reviews</span>
            <button class="drawer-toggle-btn">Hide</button>
          </div>
          <div class="drawer-content"></div>
        </div>

        <!-- Terminal Command Line Prompt -->
        <div class="terminal-input-bar">
          <span class="cli-prompt-status">❯</span>
          <input type="text" class="cli-input" placeholder="Type a task, /command, or natural language instruction..." autofocus />
          <button class="cli-submit-btn">Run</button>
        </div>
      </div>
    `;

    // Mount DiffViewer inside drawer
    const drawerContent = this.element.querySelector<HTMLElement>('.drawer-content');
    if (drawerContent) {
      drawerContent.appendChild(this.diffViewer.getElement());
    }

    this.attachEvents();
  }

  private renderTerminalStream(messages: ChatMessage[], state: AppState): void {
    const streamArea = this.element.querySelector<HTMLElement>('.terminal-stream-area');
    if (!streamArea) return;

    if (messages.length === 0) {
      streamArea.innerHTML = `
        <div class="terminal-welcome">
          <pre class="ascii-banner">
██████╗     ██╗    ██████╗     ███████╗
██╔══██╗    ██║    ██╔══██╗    ██╔════╝
██████╔╝    ██║    ██║  ██║    █████╗  
██╔══██╗    ██║    ██║  ██║    ██╔══╝  
██║  ██║    ██║    ██████╔╝    ███████╗
          </pre>
          <div class="welcome-line"><strong>R.I.D.E Claude Code Terminal Interface</strong> v1.0.1</div>
          <div class="welcome-line dim">Autonomous agent loop with local MoE routing & safety approval gate.</div>
          <div class="welcome-line dim">Type <span class="highlight">/help</span>, <span class="highlight">/diff</span>, or any coding prompt to begin.</div>
          <div class="welcome-suggestions">
            <button class="cli-chip" data-cmd="Analyze workspace crate dependencies and check build speed">Analyze workspace</button>
            <button class="cli-chip" data-cmd="Generate zero-copy memory benchmarks for MoE router">Benchmark router</button>
            <button class="cli-chip" data-cmd="Refactor commands.rs to add async streaming preview">Refactor commands.rs</button>
          </div>
        </div>
      `;
      this.attachChips();
      return;
    }

    let streamHtml = '';
    messages.forEach((m) => {
      if (m.role === 'user') {
        streamHtml += `
          <div class="terminal-entry user-entry">
            <span class="cli-user-prompt">❯ </span>
            <span class="cli-user-cmd">${this.escapeHtml(m.content)}</span>
          </div>
        `;
      } else {
        // Thoughts section
        const thoughtsHtml =
          m.thoughts && m.thoughts.length > 0
            ? `
          <div class="cli-thought-box">
            <div class="cli-thought-header">
              <span class="thought-spinner">⚙</span>
              <span>Thinking Process (${m.thoughts.length} steps)</span>
            </div>
            <div class="cli-thought-content">
              ${m.thoughts.map((t) => `<div class="cli-step">· ${this.escapeHtml(t)}</div>`).join('')}
            </div>
          </div>`
            : '';

        // Tools execution log
        const toolsHtml =
          m.toolCalls && m.toolCalls.length > 0
            ? `
          <div class="cli-tools-stream">
            ${m.toolCalls
              .map(
                (tc) => `
              <div class="cli-tool-entry ${tc.status || 'success'}">
                <span class="tool-arrow">→</span>
                <span class="tool-call-name">${this.escapeHtml(tc.name)}</span>
                <span class="tool-badge">${tc.status || 'executed'}</span>
                ${tc.output ? `<pre class="cli-tool-output">${this.escapeHtml(tc.output)}</pre>` : ''}
              </div>`
              )
              .join('')}
          </div>`
            : '';

        streamHtml += `
          <div class="terminal-entry agent-entry">
            ${thoughtsHtml}
            ${toolsHtml}
            <div class="cli-response-text">${this.formatCliOutput(m.content)}</div>
          </div>
        `;
      }
    });

    streamArea.innerHTML = streamHtml;
    streamArea.scrollTop = streamArea.scrollHeight;

    // Show inline diffs drawer if diffs exist
    const diffsDrawer = this.element.querySelector<HTMLElement>('.terminal-diffs-drawer');
    if (diffsDrawer) {
      diffsDrawer.style.display = state.diffs.length > 0 ? 'block' : 'none';
    }
  }

  private attachEvents(): void {
    const input = this.element.querySelector<HTMLInputElement>('.cli-input');
    const submitBtn = this.element.querySelector<HTMLButtonElement>('.cli-submit-btn');

    const handleRun = () => {
      if (this.isBusy || !input) return;
      const val = input.value.trim();
      if (!val) return;

      this.history.push(val);
      this.historyIndex = this.history.length;
      input.value = '';

      if (val === '/clear') {
        // Handle clear
        const streamArea = this.element.querySelector<HTMLElement>('.terminal-stream-area');
        if (streamArea) streamArea.innerHTML = '';
        return;
      }

      this.onSendMessage(val);
    };

    if (submitBtn) {
      submitBtn.addEventListener('click', handleRun);
    }

    if (input) {
      input.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          handleRun();
        } else if (e.key === 'ArrowUp') {
          e.preventDefault();
          if (this.historyIndex > 0) {
            this.historyIndex--;
            input.value = this.history[this.historyIndex] || '';
          }
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          if (this.historyIndex < this.history.length - 1) {
            this.historyIndex++;
            input.value = this.history[this.historyIndex] || '';
          } else {
            this.historyIndex = this.history.length;
            input.value = '';
          }
        }
      });
    }

    const drawerToggle = this.element.querySelector<HTMLButtonElement>('.drawer-toggle-btn');
    const drawerContent = this.element.querySelector<HTMLElement>('.drawer-content');
    if (drawerToggle && drawerContent) {
      drawerToggle.addEventListener('click', () => {
        const isHidden = drawerContent.style.display === 'none';
        drawerContent.style.display = isHidden ? 'block' : 'none';
        drawerToggle.innerText = isHidden ? 'Hide' : 'Show';
      });
    }

    this.attachChips();
  }

  private attachChips(): void {
    const chips = this.element.querySelectorAll<HTMLButtonElement>('.cli-chip');
    chips.forEach((c) => {
      c.addEventListener('click', () => {
        const cmd = c.getAttribute('data-cmd');
        if (cmd && !this.isBusy) {
          this.onSendMessage(cmd);
        }
      });
    });
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.innerText = text;
    return div.innerHTML;
  }

  private formatCliOutput(text: string): string {
    if (!text) return '';
    const parts = text.split(/(```[\s\S]*?```)/g);
    return parts
      .map((part) => {
        if (part.startsWith('```') && part.endsWith('```')) {
          const firstLineEnd = part.indexOf('\n');
          const lang = firstLineEnd !== -1 ? part.slice(3, firstLineEnd).trim() : '';
          const code = firstLineEnd !== -1 ? part.slice(firstLineEnd + 1, -3) : part.slice(3, -3);
          return `<pre class="cli-code-block"${lang ? ` data-lang="${this.escapeHtml(lang)}"` : ''}><code>${this.escapeHtml(code)}</code></pre>`;
        }
        let out = this.escapeHtml(part);
        out = out.replace(/`([^`]+)`/g, '<span class="cli-highlight">$1</span>');
        out = out.replace(/\n/g, '<br/>');
        return out;
      })
      .join('');
  }
}
