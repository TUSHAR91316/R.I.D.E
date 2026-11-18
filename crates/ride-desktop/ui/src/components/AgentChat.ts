import type { ChatMessage } from '../types';

export class AgentChat {
  private element: HTMLElement;
  private onSendMessage: (msg: string) => Promise<void>;
  private isBusy: boolean = false;

  constructor(onSendMessage: (msg: string) => Promise<void>) {
    this.onSendMessage = onSendMessage;
    this.element = document.createElement('div');
    this.element.className = 'agent-chat-container';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public setBusy(busy: boolean): void {
    this.isBusy = busy;
    const sendBtn = this.element.querySelector<HTMLButtonElement>('.chat-send-btn');
    const input = this.element.querySelector<HTMLTextAreaElement>('.chat-input');
    if (sendBtn) sendBtn.disabled = busy;
    if (input) input.disabled = busy;
    const indicator = this.element.querySelector<HTMLElement>('.stream-indicator');
    if (indicator) {
      indicator.style.display = busy ? 'flex' : 'none';
    }
  }

  public updateMessages(messages: ChatMessage[]): void {
    const list = this.element.querySelector<HTMLElement>('.chat-messages');
    if (!list) return;

    if (messages.length === 0) {
      list.innerHTML = `
        <div class="chat-empty-state">
          <div class="empty-icon">🤖</div>
          <h3>R.I.D.E Autonomous Agent</h3>
          <p>Powered by local MoE streaming & offline discovery.</p>
          <div class="quick-prompts">
            <button class="quick-chip" data-prompt="Analyze active workspace and list high priority tasks">Analyze workspace</button>
            <button class="quick-chip" data-prompt="Implement a fast zero-copy parser for unified diffs in Rust">Create diff parser</button>
            <button class="quick-chip" data-prompt="Run cargo test --workspace and report any warnings">Run test suite</button>
          </div>
        </div>
      `;
      this.attachQuickPrompts();
      return;
    }

    list.innerHTML = messages
      .map((m) => {
        const isUser = m.role === 'user';
        const roleLabel = isUser ? 'You' : 'R.I.D.E Agent';
        const avatar = isUser ? '👤' : '⚡';

        // Thoughts markup
        const thoughtsHtml =
          m.thoughts && m.thoughts.length > 0
            ? `
          <details class="thought-accordion" open>
            <summary class="thought-summary">
              <span class="thought-icon">💭</span>
              <span>Reasoning & Plan (${m.thoughts.length} steps)</span>
            </summary>
            <div class="thought-body">
              ${m.thoughts.map((t) => `<div class="thought-step">${this.escapeHtml(t)}</div>`).join('')}
            </div>
          </details>`
            : '';

        // Tool calls markup
        const toolsHtml =
          m.toolCalls && m.toolCalls.length > 0
            ? `
          <div class="tool-calls-container">
            ${m.toolCalls
              .map(
                (tc) => `
              <div class="tool-call-item ${tc.status || 'success'}">
                <span class="tool-icon">🛠️</span>
                <span class="tool-name">${this.escapeHtml(tc.name)}</span>
                <span class="tool-status">${tc.status || 'executed'}</span>
                ${tc.output ? `<pre class="tool-output">${this.escapeHtml(tc.output)}</pre>` : ''}
              </div>`
              )
              .join('')}
          </div>`
            : '';

        return `
          <div class="message-card ${m.role}">
            <div class="message-header">
              <span class="avatar">${avatar}</span>
              <span class="author">${roleLabel}</span>
              <span class="time">${new Date(m.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
            </div>
            ${thoughtsHtml}
            ${toolsHtml}
            <div class="message-content">${this.formatContent(m.content)}</div>
          </div>
        `;
      })
      .join('');

    // Scroll to bottom smoothly
    list.scrollTop = list.scrollHeight;
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="chat-header">
        <div class="chat-title">
          <span class="icon">🤖</span>
          <span>Agent Session</span>
        </div>
        <div class="chat-actions">
          <span class="stream-indicator" style="display: none;">
            <span class="dot"></span> Generating...
          </span>
        </div>
      </div>
      <div class="chat-messages"></div>
      <div class="chat-input-bar">
        <textarea class="chat-input" placeholder="Ask R.I.D.E agent... (Press Enter to send, Shift+Enter for newline)" rows="2"></textarea>
        <button class="chat-send-btn" title="Send (Enter)">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="22" y1="2" x2="11" y2="13"></line>
            <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
          </svg>
        </button>
      </div>
    `;

    this.attachEvents();
  }

  private attachEvents(): void {
    const input = this.element.querySelector<HTMLTextAreaElement>('.chat-input');
    const sendBtn = this.element.querySelector<HTMLButtonElement>('.chat-send-btn');

    const handleSend = () => {
      if (this.isBusy || !input) return;
      const val = input.value.trim();
      if (!val) return;
      input.value = '';
      this.onSendMessage(val);
    };

    if (sendBtn) {
      sendBtn.addEventListener('click', handleSend);
    }

    if (input) {
      input.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          handleSend();
        }
      });
    }

    this.attachQuickPrompts();
  }

  private attachQuickPrompts(): void {
    const chips = this.element.querySelectorAll<HTMLButtonElement>('.quick-chip');
    chips.forEach((chip) => {
      chip.addEventListener('click', () => {
        const prompt = chip.getAttribute('data-prompt');
        if (prompt && !this.isBusy) {
          this.onSendMessage(prompt);
        }
      });
    });
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.innerText = text;
    return div.innerHTML;
  }

  private formatContent(content: string): string {
    if (!content) return '';
    const parts = content.split(/(```[\s\S]*?```)/g);
    return parts
      .map((part) => {
        if (part.startsWith('```') && part.endsWith('```')) {
          const firstLineEnd = part.indexOf('\n');
          const lang = firstLineEnd !== -1 ? part.slice(3, firstLineEnd).trim() : '';
          const code = firstLineEnd !== -1 ? part.slice(firstLineEnd + 1, -3) : part.slice(3, -3);
          return `<pre class="code-block"${lang ? ` data-lang="${this.escapeHtml(lang)}"` : ''}><code>${this.escapeHtml(code)}</code></pre>`;
        }
        let formatted = this.escapeHtml(part);
        formatted = formatted.replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>');
        formatted = formatted.replace(/\n/g, '<br/>');
        return formatted;
      })
      .join('');
  }
}
