import * as api from '../api';
import type { SearchHit } from '../types';
import { basename, debounce, dirname, esc, fileSwatch } from '../util';

/** Workspace text search backed by the `search_workspace` command. */
export class SearchPanel {
  private element: HTMLElement;
  private input!: HTMLInputElement;
  private resultsEl!: HTMLElement;
  private summaryEl!: HTMLElement;
  private caseSensitive = false;
  private isRegex = false;
  private requestId = 0;
  private onOpen: (path: string, line: number) => void;

  constructor(onOpen: (path: string, line: number) => void) {
    this.onOpen = onOpen;
    this.element = document.createElement('div');
    this.element.className = 'search-panel';
    this.element.innerHTML = `
      <div class="field">
        <label class="sr-only" for="ws-search">Search in workspace</label>
        <input id="ws-search" type="text" placeholder="Search" spellcheck="false" autocomplete="off">
        <button type="button" class="icon-btn" data-opt="case" title="Match case" aria-label="Match case" aria-pressed="false">Aa</button>
        <button type="button" class="icon-btn" data-opt="regex" title="Use regular expression" aria-label="Use regular expression" aria-pressed="false">.*</button>
      </div>
      <div class="search-summary" aria-live="polite"></div>
      <div class="search-results"></div>
    `;
    this.input = this.element.querySelector('input')!;
    this.resultsEl = this.element.querySelector('.search-results')!;
    this.summaryEl = this.element.querySelector('.search-summary')!;

    const run = debounce(() => void this.run(), 250);
    this.input.addEventListener('input', run);
    this.input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        void this.run();
      }
    });
    this.element.querySelectorAll<HTMLButtonElement>('[data-opt]').forEach((btn) => {
      btn.addEventListener('click', () => {
        if (btn.dataset.opt === 'case') this.caseSensitive = !this.caseSensitive;
        else this.isRegex = !this.isRegex;
        const on = btn.dataset.opt === 'case' ? this.caseSensitive : this.isRegex;
        btn.classList.toggle('on', on);
        btn.setAttribute('aria-pressed', String(on));
        void this.run();
      });
    });
    this.resultsEl.addEventListener('click', (e) => {
      const hit = (e.target as HTMLElement).closest<HTMLElement>('[data-hit]');
      if (hit) this.onOpen(hit.dataset.path!, Number(hit.dataset.line));
    });
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public focus(): void {
    this.input.focus();
    this.input.select();
  }

  private async run(): Promise<void> {
    const query = this.input.value;
    const id = ++this.requestId;
    if (!query.trim()) {
      this.summaryEl.textContent = '';
      this.resultsEl.innerHTML = '';
      return;
    }
    this.summaryEl.textContent = 'Searching…';
    try {
      const hits = await api.searchWorkspace(query, this.caseSensitive, this.isRegex);
      if (id !== this.requestId) return; // a newer query superseded this one
      this.renderHits(query, hits);
    } catch (err) {
      if (id !== this.requestId) return;
      this.summaryEl.textContent = String(err);
      this.resultsEl.innerHTML = '';
    }
  }

  private highlight(text: string, query: string): string {
    const trimmed = text.trim();
    try {
      const re = this.isRegex
        ? new RegExp(query, this.caseSensitive ? 'g' : 'gi')
        : new RegExp(query.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), this.caseSensitive ? 'g' : 'gi');
      let out = '';
      let last = 0;
      for (const m of trimmed.matchAll(re)) {
        if (!m[0]) break;
        out += esc(trimmed.slice(last, m.index)) + `<mark>${esc(m[0])}</mark>`;
        last = (m.index ?? 0) + m[0].length;
      }
      return out + esc(trimmed.slice(last));
    } catch {
      return esc(trimmed);
    }
  }

  private renderHits(query: string, hits: SearchHit[]): void {
    const byFile = new Map<string, SearchHit[]>();
    for (const h of hits) {
      const list = byFile.get(h.path) ?? [];
      list.push(h);
      byFile.set(h.path, list);
    }
    const capped = hits.length >= 300 ? ' (showing first 300)' : '';
    this.summaryEl.textContent = hits.length
      ? `${hits.length} result${hits.length === 1 ? '' : 's'} in ${byFile.size} file${byFile.size === 1 ? '' : 's'}${capped}`
      : 'No results.';

    const html: string[] = [];
    for (const [path, list] of byFile) {
      html.push(
        `<div class="sr-file">${fileSwatch(path)}<span>${esc(basename(path))}</span><span class="dir">${esc(dirname(path))}</span></div>`
      );
      for (const h of list) {
        html.push(
          `<button type="button" class="sr-hit" data-hit data-path="${esc(h.path)}" data-line="${h.line}"><span class="ln">${h.line}</span><span class="txt">${this.highlight(h.text, query)}</span></button>`
        );
      }
    }
    this.resultsEl.innerHTML = html.join('');
  }
}
