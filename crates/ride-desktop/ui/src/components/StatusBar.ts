import type { AppState } from '../types';
import type { CursorInfo } from './Editor';
import { esc } from '../util';

export interface StatusHandlers {
  onProviderClick: () => void;
  onChangesClick: () => void;
}

/** Bottom status bar. Rendered as a string and only written when it changes. */
export class StatusBar {
  private element: HTMLElement;
  private cursor: CursorInfo | null = null;
  private state: AppState | null = null;
  private lastHtml = '';
  private turnStarted = 0;
  private timer: number | undefined;

  constructor(handlers: StatusHandlers) {
    this.element = document.createElement('footer');
    this.element.className = 'statusbar';
    this.element.addEventListener('click', (e) => {
      const act = (e.target as HTMLElement).closest<HTMLElement>('[data-act]')?.dataset.act;
      if (act === 'provider') handlers.onProviderClick();
      if (act === 'changes') handlers.onChangesClick();
    });
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public setCursor(c: CursorInfo | null): void {
    this.cursor = c;
    this.paint();
  }

  public render(state: AppState): void {
    this.state = state;
    if (state.isStreaming && !this.turnStarted) {
      this.turnStarted = performance.now();
      this.timer = window.setInterval(() => this.paint(), 500);
    } else if (!state.isStreaming && this.turnStarted) {
      this.turnStarted = 0;
      window.clearInterval(this.timer);
    }
    this.paint();
  }

  private paint(): void {
    const state = this.state;
    if (!state) return;
    const online = state.providers.find((p) => p.is_online);
    const latency = online?.latency_ms != null ? ` · ${online.latency_ms} ms` : '';
    const provider = online
      ? `<button type="button" class="sb-item" data-act="provider" title="${esc(online.base_url)}"><span class="dot on"></span>${esc(online.provider)}${latency}</button>`
      : `<button type="button" class="sb-item" data-act="provider" title="No local model server detected — start Ollama or LM Studio"><span class="dot off"></span>Offline · editor only</button>`;
    const model = online && state.currentModel ? `<span class="sb-item">${esc(state.currentModel)}</span>` : '';
    const working = state.isStreaming
      ? `<span class="sb-item accent"><span class="dot busy"></span>Agent working · ${((performance.now() - this.turnStarted) / 1000).toFixed(1)} s</span>`
      : '';
    const changes = state.diffs.length
      ? `<button type="button" class="sb-item" data-act="changes">${state.diffs.length} pending change${state.diffs.length === 1 ? '' : 's'}</button>`
      : '';
    const c = this.cursor;
    const cursor = c
      ? `<span class="sb-item">Ln ${c.line}, Col ${c.col}${c.selected ? ` (${c.selected} selected)` : ''}</span><span class="sb-item">UTF-8</span><span class="sb-item">${esc(c.language)}</span>`
      : '';

    const html = `${provider}${model}${working}${changes}<span class="sb-spacer"></span>${cursor}`;
    if (html !== this.lastHtml) {
      this.lastHtml = html;
      this.element.innerHTML = html;
    }
  }
}
