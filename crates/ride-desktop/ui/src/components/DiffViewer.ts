import type { DiffPayload } from '../types';
import { icon } from '../icons';
import { basename, dirname, esc, fileSwatch } from '../util';

export interface DiffHandlers {
  onApply: (path: string) => Promise<void>;
  /** Discard a staged diff, or revert an edit the agent already applied. */
  onRollback: (path: string) => Promise<void>;
  /** Accept an agent-applied edit: drop it from the list, leave the file as-is. */
  onKeep: (path: string) => void;
}

function renderDiffLines(diffText: string): string {
  return diffText
    .split('\n')
    .map((line) => {
      let cls = '';
      if (line.startsWith('+++') || line.startsWith('---')) cls = 'dl-head';
      else if (line.startsWith('+')) cls = 'dl-add';
      else if (line.startsWith('-')) cls = 'dl-del';
      else if (line.startsWith('@@')) cls = 'dl-hunk';
      return `<span${cls ? ` class="${cls}"` : ''}>${esc(line) || ' '}</span>`;
    })
    .join('');
}

/**
 * Pending-change review list. One instance per mount point (the editor
 * layout's panel and the agent layout's aside each own one), so layouts no
 * longer steal a shared DOM node from each other.
 */
export class DiffViewer {
  private element: HTMLElement;
  private handlers: DiffHandlers;
  private lastKey = '';
  private busyPaths = new Set<string>();
  private diffs: DiffPayload[] = [];

  constructor(handlers: DiffHandlers) {
    this.handlers = handlers;
    this.element = document.createElement('div');
    this.element.className = 'changes';
    this.element.addEventListener('click', (e) => {
      const btn = (e.target as HTMLElement).closest<HTMLButtonElement>('[data-act]');
      if (!btn || btn.disabled) return;
      const act = btn.dataset.act;
      if (act === 'apply-all') {
        void this.runAll();
        return;
      }
      const path = btn.dataset.path;
      if (!path) return;
      if (act === 'keep') this.handlers.onKeep(path);
      else void this.run(path, act === 'apply');
    });
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public render(diffs: DiffPayload[], rev: number): void {
    this.diffs = diffs;
    const key = `${rev}|${Array.from(this.busyPaths).join(',')}`;
    if (key === this.lastKey) return;
    this.lastKey = key;

    if (diffs.length === 0) {
      this.element.innerHTML = `<div class="empty">${icon('check')}<h3>No pending changes</h3><p>Edits the agent proposes appear here for review.</p></div>`;
      return;
    }

    const stagedCount = diffs.filter((d) => !d.applied).length;
    const head = `<div class="changes-head"><span class="grow">${diffs.length} file${diffs.length === 1 ? '' : 's'} changed</span>${
      stagedCount > 1 ? `<button type="button" class="btn btn-primary btn-sm" data-act="apply-all">Apply all</button>` : ''
    }</div>`;

    const cards = diffs
      .map((d) => {
        const busy = this.busyPaths.has(d.path);
        const dis = busy ? ' disabled' : '';
        const p = esc(d.path);
        const edits = d.edits && d.edits > 1 ? ` · ${d.edits} edits` : '';
        const actions = d.applied
          ? `<span class="change-note">${icon('check')}Applied by the agent${edits}</span>
            <button type="button" class="btn" data-act="discard" data-path="${p}"${dis} title="Restore the file from the backup taken before the edit">${icon('undo')}Revert</button>
            <button type="button" class="btn btn-ghost" data-act="keep" data-path="${p}"${dis} title="Keep the edit and remove it from this list">Keep</button>`
          : `<button type="button" class="btn btn-primary" data-act="apply" data-path="${p}"${dis}>${icon('check')}Apply</button>
            <button type="button" class="btn" data-act="discard" data-path="${p}"${dis}>${icon('undo')}Discard</button>`;
        return `<div class="change-card${d.applied ? ' applied' : ''}">
          <div class="change-head">
            ${fileSwatch(d.path)}
            <div class="path"><span class="file">${esc(basename(d.path))}</span><span class="dir">${esc(dirname(d.path))}</span></div>
            <span class="stat-add">+${d.additions}</span><span class="stat-del">−${d.deletions}</span>
          </div>
          <pre class="diff">${renderDiffLines(d.diff_text)}</pre>
          <div class="change-actions">${actions}</div>
        </div>`;
      })
      .join('');

    this.element.innerHTML = head + cards;
  }

  private async run(path: string, apply: boolean): Promise<void> {
    this.busyPaths.add(path);
    this.lastKey = '';
    this.render(this.diffs, -1);
    try {
      await (apply ? this.handlers.onApply(path) : this.handlers.onRollback(path));
    } finally {
      this.busyPaths.delete(path);
      this.lastKey = '';
    }
  }

  private async runAll(): Promise<void> {
    for (const d of this.diffs.filter((x) => !x.applied)) {
      await this.run(d.path, true);
    }
  }
}
