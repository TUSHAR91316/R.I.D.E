import type { DiffPayload } from '../types';

export class DiffViewer {
  private element: HTMLElement;
  private onApply: (path: string) => Promise<void>;
  private onRollback: (path: string) => Promise<void>;

  constructor(
    onApply: (path: string) => Promise<void>,
    onRollback: (path: string) => Promise<void>
  ) {
    this.onApply = onApply;
    this.onRollback = onRollback;
    this.element = document.createElement('div');
    this.element.className = 'diff-viewer-container';
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public render(diffs: DiffPayload[]): void {
    if (diffs.length === 0) {
      this.element.innerHTML = `
        <div class="diff-empty-state">
          <div class="empty-icon">✓</div>
          <h3>All Clean</h3>
          <p>No pending file diffs awaiting approval.</p>
        </div>
      `;
      return;
    }

    this.element.innerHTML = `
      <div class="diff-viewer-header">
        <div class="diff-title">
          <span class="icon">±</span>
          <span>Pending Changes (${diffs.length} ${diffs.length === 1 ? 'file' : 'files'})</span>
        </div>
      </div>
      <div class="diff-list">
        ${diffs
          .map((d, index) => {
            return `
            <div class="diff-card" data-path="${this.escapeHtml(d.path)}">
              <div class="diff-card-header">
                <div class="diff-path-info">
                  <span class="file-icon">📄</span>
                  <span class="diff-path">${this.escapeHtml(d.path)}</span>
                  <span class="diff-stats">
                    <span class="stat-add">+${d.additions}</span>
                    <span class="stat-del">-${d.deletions}</span>
                  </span>
                </div>
                <div class="diff-card-actions">
                  <button class="diff-btn apply-btn" data-index="${index}" data-path="${this.escapeHtml(d.path)}" title="Apply this change to disk">
                    ✓ Apply
                  </button>
                  <button class="diff-btn rollback-btn" data-index="${index}" data-path="${this.escapeHtml(d.path)}" title="Discard change and restore backup">
                    ↺ Rollback
                  </button>
                </div>
              </div>
              <div class="diff-content">
                <pre class="diff-code">${this.renderDiffLines(d.diff_text)}</pre>
              </div>
            </div>
          `;
          })
          .join('')}
      </div>
    `;

    this.attachEvents();
  }

  private renderDiffLines(diffText: string): string {
    const lines = diffText.split('\n');
    return lines
      .map((line) => {
        let lineClass = 'diff-line-context';
        if (line.startsWith('+') && !line.startsWith('+++')) {
          lineClass = 'diff-line-add';
        } else if (line.startsWith('-') && !line.startsWith('---')) {
          lineClass = 'diff-line-del';
        } else if (line.startsWith('@@')) {
          lineClass = 'diff-line-hunk';
        } else if (line.startsWith('---') || line.startsWith('+++')) {
          lineClass = 'diff-line-header';
        }
        return `<span class="${lineClass}">${this.escapeHtml(line)}</span>`;
      })
      .join('\n');
  }

  private attachEvents(): void {
    const applyButtons = this.element.querySelectorAll<HTMLButtonElement>('.apply-btn');
    applyButtons.forEach((btn) => {
      btn.addEventListener('click', async (e) => {
        e.preventDefault();
        const path = btn.getAttribute('data-path');
        if (path) {
          btn.disabled = true;
          btn.innerText = 'Applying...';
          await this.onApply(path);
        }
      });
    });

    const rollbackButtons = this.element.querySelectorAll<HTMLButtonElement>('.rollback-btn');
    rollbackButtons.forEach((btn) => {
      btn.addEventListener('click', async (e) => {
        e.preventDefault();
        const path = btn.getAttribute('data-path');
        if (path) {
          btn.disabled = true;
          btn.innerText = 'Rolling back...';
          await this.onRollback(path);
        }
      });
    });
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.innerText = text;
    return div.innerHTML;
  }
}
