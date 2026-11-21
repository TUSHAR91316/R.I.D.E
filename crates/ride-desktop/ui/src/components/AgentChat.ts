import type { AppState, ApprovalItem, ChatMessage } from '../types';
import { icon } from '../icons';
import { esc, renderMarkdown, summarizeArgs, wireCopyButtons } from '../util';

export interface ChatHandlers {
  onSend: (message: string) => void;
  onSelectModel: (modelId: string) => void;
  onApproval: (requestId: string, approved: boolean) => void;
  onStop: () => void;
  getContextPath: () => string | null;
}

const SAFETY_LABEL: Record<string, string> = {
  PotentiallyDestructive: 'Potentially destructive',
  SafeExecution: 'Build / test',
  ReadOnly: 'Read-only',
  Unknown: 'Unclassified',
};

/** "Run shell command?" cards (shared by the chat panel and the Agent stream). */
export function renderApprovals(approvals: ApprovalItem[] | undefined): string {
  if (!approvals?.length) return '';
  return approvals
    .map((a) => {
      const danger = a.safety === 'PotentiallyDestructive';
      const safety = SAFETY_LABEL[a.safety] ?? a.safety;
      if (a.status === 'approved' || a.status === 'denied') {
        const ok = a.status === 'approved';
        return `<div class="approval resolved ${a.status}">
          <span class="status-ic">${icon(ok ? 'check' : 'x')}</span>
          <span class="approval-verdict">${ok ? 'Allowed' : 'Denied'}</span>
          <code class="approval-cmd">${esc(a.command)}</code>
        </div>`;
      }
      const busy = a.status === 'sending';
      return `<div class="approval${danger ? ' danger' : ''}" role="group" aria-label="Approve shell command">
        <div class="approval-title">${icon(danger ? 'alert' : 'terminal')}<span>Run shell command?</span><span class="approval-safety">${esc(safety)}</span></div>
        <code class="approval-cmd">${esc(a.command)}</code>
        <div class="approval-actions">
          <button type="button" class="btn btn-primary btn-sm" data-approval="${esc(a.id)}" data-allow="1"${busy ? ' disabled' : ''}>Allow once</button>
          <button type="button" class="btn btn-sm" data-approval="${esc(a.id)}" data-allow="0"${busy ? ' disabled' : ''}>Deny</button>
        </div>
      </div>`;
    })
    .join('');
}

/** Delegated click handling for approval buttons inside `root`. */
export function wireApprovalButtons(root: HTMLElement, onApproval: (id: string, approved: boolean) => void): void {
  root.addEventListener('click', (e) => {
    const btn = (e.target as HTMLElement).closest<HTMLButtonElement>('[data-approval]');
    if (!btn || btn.disabled) return;
    e.stopPropagation();
    onApproval(btn.dataset.approval!, btn.dataset.allow === '1');
  });
}

export function isAwaitingApproval(m: ChatMessage | undefined): boolean {
  return Boolean(m?.approvals?.some((a) => a.status === 'pending' || a.status === 'sending'));
}

const SUGGESTIONS = [
  'Explain what the active file does',
  'Find and fix bugs in the active file',
  'Write unit tests for the active file',
  'Run cargo test --workspace and summarise failures',
];

/** Render a list of chat messages (shared by the chat panel and Zen drawer). */
export function renderMessages(messages: ChatMessage[], streaming: boolean, openThoughts: Set<string>): string {
  return messages
    .map((m, i) => {
      if (m.role === 'user') {
        return `<div class="msg-user">${esc(m.content)}</div>`;
      }
      const isLast = i === messages.length - 1;
      const live = isLast && streaming;
      const thoughts = m.thoughts ?? [];
      const tools = m.toolCalls ?? [];
      const time = new Date(m.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      const duration = m.durationMs ? ` · ${(m.durationMs / 1000).toFixed(1)} s` : '';

      const thoughtsHtml = thoughts.length
        ? `<details class="thoughts" data-mid="${esc(m.id)}"${live || openThoughts.has(m.id) ? ' open' : ''}>
            <summary>${icon('chevronRight')}${live ? 'Thinking…' : `Thought for ${thoughts.length} step${thoughts.length === 1 ? '' : 's'}`}</summary>
            <div class="thoughts-body">${esc(thoughts.join(''))}</div>
          </details>`
        : '';

      const toolsHtml = tools.length
        ? `<div class="tools">${tools
            .map((tc) => {
              const status = tc.status ?? 'success';
              const statusIcon =
                status === 'pending' ? '<span class="spinner"></span>' : `<span class="status-ic">${icon(status === 'failed' ? 'x' : 'check')}</span>`;
              return `<details class="tool-row ${status}">
                <summary>${statusIcon}<span class="tname">${esc(tc.name)}</span><span class="targ">${esc(summarizeArgs(tc.args))}</span></summary>
                ${tc.output ? `<pre class="tool-out">${esc(tc.output)}</pre>` : ''}
              </details>`;
            })
            .join('')}</div>`
        : '';

      const body = m.isError
        ? `<div class="msg-error">${esc(m.content)}</div>`
        : m.content
          ? `<div class="msg-body">${renderMarkdown(m.content)}</div>`
          : live
            ? `<div class="typing"><span class="spinner"></span>${isAwaitingApproval(m) ? 'Waiting for approval…' : 'Working…'}</div>`
            : '';
      const stopped = m.cancelled ? `<div class="msg-note">${icon('stop')}Stopped by you</div>` : '';

      return `<div class="msg-agent">
        <div class="msg-meta"><span class="avatar" aria-hidden="true">R</span><span class="who">Agent</span><span class="time">${time}${duration}</span></div>
        ${thoughtsHtml}${toolsHtml}${renderApprovals(m.approvals)}${body}${stopped}
      </div>`;
    })
    .join('');
}

export class AgentChat {
  private element: HTMLElement;
  private scrollEl!: HTMLElement;
  private input!: HTMLTextAreaElement;
  private sendBtn!: HTMLButtonElement;
  private modelSelect!: HTMLSelectElement;
  private chipsEl!: HTMLElement;
  private handlers: ChatHandlers;
  private busy = false;
  private lastRev = -1;
  private lastModelsKey = '';
  private openThoughts = new Set<string>();
  private includeContext = true;

  constructor(handlers: ChatHandlers) {
    this.handlers = handlers;
    this.element = document.createElement('div');
    this.element.className = 'chat';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public focusInput(): void {
    this.input.focus();
  }

  public update(state: AppState): void {
    this.setBusy(state.isStreaming);
    this.renderModels(state);
    this.renderChips();
    if (state.rev.messages === this.lastRev) return;
    this.lastRev = state.rev.messages;

    const nearBottom = this.scrollEl.scrollHeight - this.scrollEl.scrollTop - this.scrollEl.clientHeight < 80;
    if (state.messages.length === 0) {
      this.scrollEl.innerHTML = `
        <div class="chat-empty">
          <span class="avatar" style="width:28px;height:28px;font-size:13px;border-radius:8px" aria-hidden="true">R</span>
          <h3>Local agent</h3>
          <p>Runs on your machine against models detected on localhost. Ask a question or describe a change. The agent's edits are listed under Changes, where you can revert them, and shell commands ask before they run.</p>
          <div class="suggestions">
            ${SUGGESTIONS.map((s) => `<button type="button" class="suggestion" data-prompt="${esc(s)}">${icon('sparkle')}<span>${esc(s)}</span></button>`).join('')}
          </div>
        </div>`;
      return;
    }
    this.scrollEl.innerHTML = renderMessages(state.messages, state.isStreaming, this.openThoughts);
    if (nearBottom) this.scrollEl.scrollTop = this.scrollEl.scrollHeight;
  }

  /** Re-render context chips (cheap; called when the active file changes). */
  public renderChips(): void {
    const ctx = this.handlers.getContextPath();
    const html = ctx
      ? `<button type="button" class="chip" data-ctx title="${this.includeContext ? 'Click to stop sending' : 'Click to send'} the active file path as context" aria-pressed="${this.includeContext}" style="${this.includeContext ? '' : 'opacity:.55;text-decoration:line-through'}">${icon('files')}${esc(ctx.split('/').pop() ?? ctx)}</button>`
      : '';
    if (this.chipsEl.innerHTML !== html) this.chipsEl.innerHTML = html;
  }

  private renderModels(state: AppState): void {
    const online = state.providers.filter((p) => p.is_online && p.models.length > 0);
    const key = online.map((p) => p.provider + ':' + p.models.map((m) => m.id).join(',')).join('|') + '#' + state.currentModel;
    if (key === this.lastModelsKey) return;
    this.lastModelsKey = key;

    if (online.length === 0) {
      this.modelSelect.innerHTML = '<option value="">No local model</option>';
      this.modelSelect.disabled = true;
      return;
    }
    this.modelSelect.disabled = false;
    this.modelSelect.innerHTML = online
      .map(
        (p) =>
          `<optgroup label="${esc(p.provider)}">${p.models
            .map((m) => `<option value="${esc(m.id)}"${m.id === state.currentModel || m.name === state.currentModel ? ' selected' : ''}>${esc(m.name || m.id)}</option>`)
            .join('')}</optgroup>`
      )
      .join('');
  }

  private setBusy(busy: boolean): void {
    if (busy === this.busy) return;
    this.busy = busy;
    // While a turn runs the send button becomes a stop button.
    this.sendBtn.classList.toggle('is-stop', busy);
    this.sendBtn.innerHTML = icon(busy ? 'stop' : 'send');
    this.sendBtn.setAttribute('aria-label', busy ? 'Stop the agent' : 'Send');
    this.sendBtn.title = busy ? 'Stop the agent' : 'Send (Enter)';
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="chat-scroll" aria-live="polite"></div>
      <div class="composer-wrap">
        <div class="composer">
          <div class="composer-chips"></div>
          <label class="sr-only" for="chat-input">Message the agent</label>
          <textarea id="chat-input" rows="2" placeholder="Ask, or describe a change… (Enter to send, Shift+Enter for newline)"></textarea>
          <div class="composer-foot">
            <label class="sr-only" for="chat-model">Model</label>
            <select id="chat-model" class="model-select"></select>
            <span class="grow"></span>
            <button type="button" class="send-btn" aria-label="Send">${icon('send')}</button>
          </div>
        </div>
      </div>
    `;
    this.scrollEl = this.element.querySelector('.chat-scroll')!;
    this.input = this.element.querySelector('textarea')!;
    this.sendBtn = this.element.querySelector('.send-btn')!;
    this.modelSelect = this.element.querySelector('select')!;
    this.chipsEl = this.element.querySelector('.composer-chips')!;

    const send = () => {
      if (this.busy) return;
      const text = this.input.value.trim();
      if (!text) return;
      this.input.value = '';
      this.autosize();
      const ctx = this.includeContext ? this.handlers.getContextPath() : null;
      this.handlers.onSend(ctx ? `${text}\n\n(Active file: \`${ctx}\`)` : text);
    };

    this.sendBtn.addEventListener('click', () => (this.busy ? this.handlers.onStop() : send()));
    wireApprovalButtons(this.scrollEl, this.handlers.onApproval);
    this.input.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.busy) {
        e.preventDefault();
        this.handlers.onStop();
      } else if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
        e.preventDefault();
        send();
      }
    });
    this.input.addEventListener('input', () => this.autosize());
    this.modelSelect.addEventListener('change', () => this.handlers.onSelectModel(this.modelSelect.value));
    this.chipsEl.addEventListener('click', (e) => {
      if ((e.target as HTMLElement).closest('[data-ctx]')) {
        this.includeContext = !this.includeContext;
        this.renderChips();
      }
    });
    this.scrollEl.addEventListener('click', (e) => {
      const s = (e.target as HTMLElement).closest<HTMLElement>('[data-prompt]');
      if (s && !this.busy) {
        const ctx = this.includeContext ? this.handlers.getContextPath() : null;
        const text = s.dataset.prompt!;
        this.handlers.onSend(ctx ? `${text}\n\n(Active file: \`${ctx}\`)` : text);
      }
    });
    // Remember which "thoughts" blocks the user expanded across re-renders.
    this.scrollEl.addEventListener(
      'toggle',
      (e) => {
        const d = e.target as HTMLDetailsElement;
        const id = d.dataset?.mid;
        if (!id) return;
        if (d.open) this.openThoughts.add(id);
        else this.openThoughts.delete(id);
      },
      true
    );
    wireCopyButtons(this.scrollEl);
  }

  private autosize(): void {
    this.input.style.height = 'auto';
    this.input.style.height = `${Math.min(this.input.scrollHeight, 200)}px`;
  }
}
