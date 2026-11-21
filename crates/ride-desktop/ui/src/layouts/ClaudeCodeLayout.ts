import type { AppState, ChatMessage } from '../types';
import { DiffViewer, type DiffHandlers } from '../components/DiffViewer';
import { icon } from '../icons';
import { esc, renderMarkdown, summarizeArgs, wireCopyButtons } from '../util';
import { isAwaitingApproval, renderApprovals, wireApprovalButtons } from '../components/AgentChat';

export interface AgentLayoutDeps {
  diffHandlers: DiffHandlers;
  onSend: (message: string) => void;
  onApproval: (requestId: string, approved: boolean) => void;
  onStop: () => void;
  onClear: () => void;
  onShowEditor: () => void;
}

const SUGGESTIONS = [
  'Summarise this workspace and its crates',
  'Run cargo test --workspace and fix any failures',
  'Find TODOs and unfinished code paths',
];

const SLASH_HELP = `Commands:
  /clear   start a new session
  /diff    show pending changes
  /editor  switch to the editor layout
  /help    show this help`;

/** Terminal-first layout: a reasoning/tool stream plus a change-review aside. */
export class ClaudeCodeLayout {
  private element: HTMLElement;
  private deps: AgentLayoutDeps;
  private diffViewer: DiffViewer;
  private streamEl!: HTMLElement;
  private input!: HTMLInputElement;
  private glyph!: HTMLElement;
  private runBtn!: HTMLButtonElement;
  private statsEl!: HTMLElement;
  private skillsEl!: HTMLElement;
  private changesCount!: HTMLElement;
  private history: string[] = [];
  private historyIndex = 0;
  private busy = false;
  private lastRev = -1;

  constructor(deps: AgentLayoutDeps) {
    this.deps = deps;
    this.diffViewer = new DiffViewer(deps.diffHandlers);
    this.element = document.createElement('div');
    this.element.className = 'agent-layout';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public mount(): void {
    requestAnimationFrame(() => this.input.focus());
  }

  public update(state: AppState): void {
    this.setBusy(state.isStreaming);
    this.diffViewer.render(state.diffs, state.rev.diffs);
    this.changesCount.textContent = `${state.diffs.length} file${state.diffs.length === 1 ? '' : 's'}`;
    this.renderStats(state);

    if (state.rev.messages === this.lastRev) return;
    this.lastRev = state.rev.messages;
    this.renderStream(state);
  }

  private setBusy(busy: boolean): void {
    if (busy === this.busy) return;
    this.busy = busy;
    // The input stays focusable while busy so Esc can stop the turn.
    this.input.readOnly = busy;
    this.input.placeholder = busy ? 'Agent is working… press Esc or ■ to stop' : 'Describe a task, or type / for commands';
    this.glyph.innerHTML = busy ? '<span class="spinner"></span>' : '❯';
    this.runBtn.classList.toggle('is-stop', busy);
    this.runBtn.innerHTML = icon(busy ? 'stop' : 'send');
    this.runBtn.setAttribute('aria-label', busy ? 'Stop the agent' : 'Run');
    this.runBtn.title = busy ? 'Stop the agent (Esc)' : 'Run (Enter)';
    if (!busy) this.input.focus();
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <section class="stream-col" aria-label="Agent session">
        <div class="stream"><div class="stream-inner" aria-live="polite"></div></div>
        <div class="cli-wrap">
          <div class="cli-inner">
            <div class="cli">
              <span class="prompt-glyph" aria-hidden="true">❯</span>
              <label class="sr-only" for="cli-input">Task or command</label>
              <input id="cli-input" type="text" placeholder="Describe a task, or type / for commands" spellcheck="false" autocomplete="off">
              <button type="button" class="send-btn" data-act="run" aria-label="Run">${icon('send')}</button>
            </div>
            <div class="cli-hints"><span>/clear new session</span><span>/diff review</span><span>↑ history</span><span class="grow"></span><span data-model></span></div>
          </div>
        </div>
      </section>
      <aside class="agent-aside" aria-label="Session details">
        <div class="pane-header" style="border-bottom:1px solid var(--border)">
          <span class="pane-title" style="text-transform:none;letter-spacing:0;font-size:13px;color:var(--text)">Changes</span>
          <span class="chip" data-count>0 files</span>
        </div>
        <div class="aside-section"><span class="section-label">Session</span><div class="stat-grid"></div></div>
        <div class="aside-section"><span class="section-label">Active skills</span><div class="chip-row"></div></div>
        <div class="aside-section" style="padding-bottom:0"><span class="section-label">Pending changes</span></div>
      </aside>
    `;
    this.streamEl = this.element.querySelector('.stream-inner')!;
    this.input = this.element.querySelector('#cli-input')!;
    this.glyph = this.element.querySelector('.prompt-glyph')!;
    this.statsEl = this.element.querySelector('.stat-grid')!;
    this.skillsEl = this.element.querySelector('.chip-row')!;
    this.changesCount = this.element.querySelector('[data-count]')!;
    this.element.querySelector('.agent-aside')!.appendChild(this.diffViewer.getElement());

    const run = () => {
      if (this.busy) return;
      const val = this.input.value.trim();
      if (!val) return;
      this.history.push(val);
      this.historyIndex = this.history.length;
      this.input.value = '';
      this.handleCommand(val);
    };

    this.runBtn = this.element.querySelector<HTMLButtonElement>('[data-act="run"]')!;
    this.runBtn.addEventListener('click', () => (this.busy ? this.deps.onStop() : run()));
    wireApprovalButtons(this.streamEl, this.deps.onApproval);
    this.input.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.busy) {
        e.preventDefault();
        this.deps.onStop();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        run();
      } else if (e.key === 'ArrowUp' && this.history.length) {
        e.preventDefault();
        this.historyIndex = Math.max(0, this.historyIndex - 1);
        this.input.value = this.history[this.historyIndex] ?? '';
      } else if (e.key === 'ArrowDown' && this.history.length) {
        e.preventDefault();
        this.historyIndex = Math.min(this.history.length, this.historyIndex + 1);
        this.input.value = this.history[this.historyIndex] ?? '';
      }
    });
    this.streamEl.addEventListener('click', (e) => {
      const s = (e.target as HTMLElement).closest<HTMLElement>('[data-prompt]');
      if (s && !this.busy) this.deps.onSend(s.dataset.prompt!);
    });
    wireCopyButtons(this.streamEl);
  }

  private handleCommand(val: string): void {
    const cmd = val.toLowerCase();
    if (cmd === '/clear') {
      this.deps.onClear(); // clears app state, so the next update re-renders the welcome view
      return;
    }
    if (cmd === '/diff') {
      this.diffViewer.getElement().scrollIntoView({ behavior: 'smooth', block: 'start' });
      return;
    }
    if (cmd === '/editor') {
      this.deps.onShowEditor();
      return;
    }
    if (cmd === '/help' || cmd === '/') {
      this.appendNote(SLASH_HELP);
      return;
    }
    if (cmd.startsWith('/')) {
      this.appendNote(`Unknown command ${val}. Type /help for the list.`);
      return;
    }
    this.deps.onSend(val);
  }

  private appendNote(text: string): void {
    const el = document.createElement('div');
    el.className = 'entry-sub';
    el.style.marginLeft = '0';
    el.textContent = text;
    this.streamEl.appendChild(el);
    this.scrollToEnd();
  }

  private scrollToEnd(): void {
    const scroller = this.streamEl.parentElement!;
    scroller.scrollTop = scroller.scrollHeight;
  }

  private renderStats(state: AppState): void {
    const toolCalls = state.messages.reduce((n, m) => n + (m.toolCalls?.length ?? 0), 0);
    const turns = state.messages.filter((m) => m.role === 'user').length;
    const online = state.providers.find((p) => p.is_online);
    const model = online ? state.currentModel || '—' : 'offline';
    const html = `
      <div class="stat"><span class="k">Turns</span><span class="v">${turns}</span></div>
      <div class="stat"><span class="k">Tool calls</span><span class="v">${toolCalls}</span></div>
      <div class="stat" style="grid-column:span 2"><span class="k">Model</span><span class="v" style="font-size:13px">${esc(model)}</span></div>`;
    if (this.statsEl.innerHTML !== html) this.statsEl.innerHTML = html;

    const enabled = state.skills.filter((s) => s.enabled);
    const idle = state.skills.length - enabled.length;
    const skills =
      enabled.map((s) => `<span class="chip">${esc(s.name)}</span>`).join('') +
      (idle ? `<span class="chip dashed">+${idle} idle</span>` : '') || '<span class="chip dashed">none</span>';
    if (this.skillsEl.innerHTML !== skills) this.skillsEl.innerHTML = skills;

    const modelHint = this.element.querySelector<HTMLElement>('[data-model]')!;
    modelHint.textContent = online ? `${online.provider} · ${state.currentModel}` : 'offline · editor-only mode';
  }

  private renderStream(state: AppState): void {
    if (state.messages.length === 0) {
      this.streamEl.innerHTML = `
        <div class="welcome">
          <h2>What should we work on?</h2>
          <p>The agent reads, searches and edits files in this workspace using your local model. Each file edit is saved with a backup and listed on the right so you can revert it. Shell commands other than read-only and whitelisted ones ask first.</p>
          <div class="suggestions">${SUGGESTIONS.map((s) => `<button type="button" class="suggestion" data-prompt="${esc(s)}">${icon('terminal')}<span>${esc(s)}</span></button>`).join('')}</div>
        </div>`;
      return;
    }
    this.streamEl.innerHTML = state.messages.map((m, i) => this.renderEntry(m, i === state.messages.length - 1 && state.isStreaming)).join('');
    this.scrollToEnd();
  }

  private renderEntry(m: ChatMessage, live: boolean): string {
    if (m.role === 'user') {
      return `<div class="entry-user"><span class="prompt-glyph" aria-hidden="true">❯</span><span>${esc(m.content)}</span></div>`;
    }
    const parts: string[] = [];
    if (m.thoughts?.length) {
      parts.push(`<div class="entry-line"><span class="bullet">✻</span><span>${esc(m.thoughts.join('').trim())}</span></div>`);
    }
    for (const tc of m.toolCalls ?? []) {
      const status = tc.status ?? 'success';
      const bullet = status === 'pending' ? 'run' : status === 'failed' ? 'err' : 'ok';
      parts.push(
        `<div><div class="entry-line"><span class="bullet ${bullet}">●</span><span><strong>${esc(tc.name)}</strong>(${esc(summarizeArgs(tc.args))})</span></div>${
          tc.output ? `<div class="entry-sub">└ ${esc(tc.output)}</div>` : ''
        }</div>`
      );
    }
    const approvals = renderApprovals(m.approvals);
    if (approvals) parts.push(`<div class="entry-approvals">${approvals}</div>`);
    if (m.isError) {
      parts.push(`<div class="msg-error">${esc(m.content)}</div>`);
    } else if (m.content) {
      parts.push(`<div class="entry-text">${renderMarkdown(m.content)}</div>`);
    } else if (live) {
      parts.push(
        `<div class="entry-line"><span class="spinner" style="margin-top:5px"></span><span>${isAwaitingApproval(m) ? 'Waiting for approval…' : 'Working…'}</span></div>`
      );
    }
    if (m.cancelled) {
      parts.push(`<div class="msg-note">${icon('stop')}Stopped by you</div>`);
    }
    if (m.durationMs && !live) {
      parts.push(`<div class="entry-sub" style="margin-left:0">${(m.durationMs / 1000).toFixed(1)} s</div>`);
    }
    return parts.join('');
  }
}
