import type { AppState } from '../types';
import type { EditorComponent } from '../components/Editor';
import type { AgentChat } from '../components/AgentChat';
import { icon } from '../icons';
import { esc, renderMarkdown, toast, wireCopyButtons } from '../util';

export interface ZenDeps {
  editor: EditorComponent;
  chat: AgentChat;
  onSend: (message: string) => void;
  onOpenPalette: () => void;
}

interface PendingEdit {
  baseCount: number;
  path: string;
  from: number;
  to: number;
  original: string;
}

/** Distraction-free canvas with an inline Ctrl+K edit bar (Codex-style). */
export class CodexLayout {
  private element: HTMLElement;
  private deps: ZenDeps;
  private canvas!: HTMLElement;
  private drawer!: HTMLElement;
  private fileEl!: HTMLElement;
  private prompt!: HTMLElement;
  private promptInput!: HTMLInputElement;
  private promptCtx!: HTMLElement;
  private promptResult!: HTMLElement;
  private promptFoot!: HTMLElement;
  private hudCursor!: HTMLElement;
  private hudModel!: HTMLElement;
  private pending: PendingEdit | null = null;
  private suggestion: string | null = null;
  private state: AppState | null = null;

  constructor(deps: ZenDeps) {
    this.deps = deps;
    this.element = document.createElement('div');
    this.element.className = 'zen';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public mount(): void {
    if (!this.canvas.contains(this.deps.editor.getElement())) this.canvas.appendChild(this.deps.editor.getElement());
    if (!this.drawer.hidden && !this.drawer.contains(this.deps.chat.getElement())) {
      this.drawer.querySelector('.pane-body')!.appendChild(this.deps.chat.getElement());
    }
    this.syncFile();
    requestAnimationFrame(() => this.deps.editor.focus());
  }

  public setCursor(line: number, col: number): void {
    this.hudCursor.textContent = `Ln ${line}, Col ${col}`;
  }

  public syncFile(): void {
    const tab = this.deps.editor.getActiveTab();
    this.fileEl.innerHTML = tab
      ? `<span>${esc(tab.path.split('/').slice(0, -1).join(' / '))}${tab.path.includes('/') ? ' /' : ''}</span> <span class="file">${esc(tab.name)}</span>${tab.isModified ? ' <span class="dot" style="background:var(--text-2)" aria-label="Unsaved"></span>' : ''}`
      : '<span>No file open</span>';
  }

  public update(state: AppState): void {
    this.state = state;
    if (!this.drawer.hidden) this.deps.chat.update(state);
    const online = state.providers.find((p) => p.is_online);
    this.hudModel.innerHTML = `<span class="dot ${online ? 'on' : 'off'}"></span>${esc(online ? state.currentModel || online.provider : 'Offline')}`;
    this.checkPending(state);
  }

  /** Ctrl+K inside Zen opens the inline edit bar on the current selection. */
  public openInlinePrompt(): void {
    const tab = this.deps.editor.getActiveTab();
    if (!tab) {
      this.deps.onOpenPalette();
      return;
    }
    const sel = this.deps.editor.getSelectionOrDoc();
    this.promptCtx.textContent = sel.isSelection
      ? `Editing ${tab.name} · lines ${sel.fromLine}–${sel.toLine}`
      : `Editing ${tab.name} · whole file (select code to target a range)`;
    this.promptResult.hidden = true;
    this.promptFoot.hidden = true;
    this.prompt.hidden = false;
    this.promptInput.value = '';
    this.promptInput.disabled = false;
    this.promptInput.focus();
  }

  private closePrompt(): void {
    this.prompt.hidden = true;
    this.pending = null;
    this.suggestion = null;
    this.deps.editor.focus();
  }

  private submitPrompt(): void {
    const instruction = this.promptInput.value.trim();
    const tab = this.deps.editor.getActiveTab();
    if (!instruction || !tab || !this.state || this.state.isStreaming) return;
    const sel = this.deps.editor.getSelectionOrDoc();
    if (sel.text.length > 60_000) {
      toast('Selection is too large for an inline edit — select a smaller range.', 'err');
      return;
    }
    this.pending = { baseCount: this.state.messages.length, path: tab.path, from: sel.from, to: sel.to, original: sel.text };
    this.suggestion = null;
    this.promptInput.disabled = true;
    this.promptResult.hidden = false;
    this.promptResult.innerHTML = '<div class="typing"><span class="spinner"></span>Generating edit…</div>';
    this.promptFoot.hidden = true;

    const lang = tab.name.split('.').pop() ?? '';
    this.deps.onSend(
      `${instruction}\n\nApply this to ${sel.isSelection ? `lines ${sel.fromLine}-${sel.toLine} of` : 'the whole of'} \`${tab.path}\`. ` +
        `Reply with ONLY the replacement code in a single fenced code block, no explanation.\n\n\`\`\`${lang}\n${sel.text}\n\`\`\``
    );
  }

  private checkPending(state: AppState): void {
    const p = this.pending;
    if (!p || state.isStreaming || state.messages.length <= p.baseCount) return;
    const reply = [...state.messages].reverse().find((m) => m.role === 'assistant');
    if (!reply) return;
    this.pending = null;
    this.promptInput.disabled = false;

    const block = /```[^\n]*\n([\s\S]*?)```/.exec(reply.content);
    this.suggestion = block ? block[1].replace(/\n$/, '') : null;
    this.promptResult.innerHTML = reply.isError
      ? `<div class="msg-error">${esc(reply.content)}</div>`
      : `<div class="msg-body">${renderMarkdown(reply.content)}</div>`;
    this.promptFoot.hidden = false;
    this.promptFoot.innerHTML = this.suggestion
      ? `<button type="button" class="btn btn-primary btn-sm" data-act="accept">Accept <span class="kbd" style="background:transparent;color:inherit">Ctrl ⏎</span></button><button type="button" class="btn btn-sm" data-act="reject">Reject</button><span class="grow"></span><span>${esc(state.currentModel)}${reply.durationMs ? ` · ${(reply.durationMs / 1000).toFixed(1)} s` : ''}</span>`
      : `<span class="grow">No code block in the reply — nothing to apply.</span><button type="button" class="btn btn-sm" data-act="reject">Close</button>`;
    // Keep the target range for accept.
    this.pendingAccept = { ...p };
  }

  private pendingAccept: PendingEdit | null = null;

  private accept(): void {
    const target = this.pendingAccept;
    const tab = this.deps.editor.getActiveTab();
    if (!target || !this.suggestion || !tab) return;
    if (tab.path !== target.path) {
      toast(`Switch back to ${target.path.split('/').pop()} to apply this edit.`, 'err');
      return;
    }
    // Guard against applying over text that changed since the request.
    if (this.deps.editor.sliceDoc(target.from, target.to) !== target.original) {
      toast('The code changed since the request — re-run the edit.', 'err');
      return;
    }
    this.deps.editor.replaceRange(target.from, target.to, this.suggestion);
    toast('Edit applied — Ctrl+Z to undo', 'ok');
    this.pendingAccept = null;
    this.closePrompt();
  }

  private toggleDrawer(): void {
    this.drawer.hidden = !this.drawer.hidden;
    const btn = this.element.querySelector('[data-act="drawer"]');
    btn?.classList.toggle('on', !this.drawer.hidden);
    if (!this.drawer.hidden) {
      this.drawer.querySelector('.pane-body')!.appendChild(this.deps.chat.getElement());
      if (this.state) this.deps.chat.update(this.state);
      this.deps.chat.focusInput();
    }
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="zen-main">
        <div class="zen-top">
          <span class="zen-file"></span>
          <span class="grow"></span>
          <button type="button" class="icon-btn" data-act="inline" title="Edit with AI (Ctrl+K)" aria-label="Edit with AI">${icon('sparkle')}</button>
          <button type="button" class="icon-btn" data-act="drawer" title="Toggle assistant" aria-label="Toggle assistant">${icon('panelRight')}</button>
        </div>
        <div class="zen-canvas"></div>
        <div class="inline-prompt" hidden role="dialog" aria-label="Inline edit">
          <div class="ip-row">
            ${icon('sparkle')}
            <label class="sr-only" for="ip-input">Edit instruction</label>
            <input id="ip-input" type="text" placeholder="Describe the edit… (Enter to generate)" spellcheck="false" autocomplete="off">
            <span class="kbd">Esc</span>
          </div>
          <div class="ip-context"></div>
          <div class="ip-result" hidden></div>
          <div class="ip-foot" hidden></div>
        </div>
        <div class="zen-hud">
          <span data-hud="model"></span>
          <span data-hud="cursor">Ln 1, Col 1</span>
          <button type="button" data-act="inline"><span class="kbd">Ctrl K</span>edit inline</button>
          <button type="button" data-act="palette"><span class="kbd">Ctrl P</span>commands</button>
        </div>
      </div>
      <aside class="zen-drawer" hidden aria-label="Assistant">
        <div class="pane-header" style="border-bottom:1px solid var(--border)"><span class="pane-title">Assistant</span><button type="button" class="icon-btn" data-act="drawer" aria-label="Close assistant">${icon('close')}</button></div>
        <div class="pane-body" style="display:flex;flex-direction:column;overflow:hidden"></div>
      </aside>
    `;
    this.canvas = this.element.querySelector('.zen-canvas')!;
    this.drawer = this.element.querySelector('.zen-drawer')!;
    this.fileEl = this.element.querySelector('.zen-file')!;
    this.prompt = this.element.querySelector('.inline-prompt')!;
    this.promptInput = this.element.querySelector('#ip-input')!;
    this.promptCtx = this.element.querySelector('.ip-context')!;
    this.promptResult = this.element.querySelector('.ip-result')!;
    this.promptFoot = this.element.querySelector('.ip-foot')!;
    this.hudCursor = this.element.querySelector('[data-hud="cursor"]')!;
    this.hudModel = this.element.querySelector('[data-hud="model"]')!;
    this.hudModel.style.cssText = 'display:inline-flex;align-items:center;gap:6px';
    wireCopyButtons(this.promptResult);

    this.element.addEventListener('click', (e) => {
      switch ((e.target as HTMLElement).closest<HTMLElement>('[data-act]')?.dataset.act) {
        case 'inline':
          this.openInlinePrompt();
          break;
        case 'drawer':
          this.toggleDrawer();
          break;
        case 'palette':
          this.deps.onOpenPalette();
          break;
        case 'accept':
          this.accept();
          break;
        case 'reject':
          this.pendingAccept = null;
          this.closePrompt();
          break;
      }
    });
    this.prompt.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        this.closePrompt();
      } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && this.suggestion) {
        e.preventDefault();
        e.stopPropagation();
        this.accept();
      } else if (e.key === 'Enter' && e.target === this.promptInput) {
        e.preventDefault();
        this.submitPrompt();
      }
    });
  }
}
