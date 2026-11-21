import type { UiLayout } from '../types';
import { icon } from '../icons';

export interface TitleBarHandlers {
  onLayout: (layout: UiLayout) => void;
  onCommandCenter: () => void;
  onOpenFolder: () => void;
}

const LAYOUTS: Array<{ id: UiLayout; label: string; key: string }> = [
  { id: 'vs_code', label: 'Editor', key: 'Ctrl+1' },
  { id: 'claude_code', label: 'Agent', key: 'Ctrl+2' },
  { id: 'codex', label: 'Zen', key: 'Ctrl+3' },
];

export class TitleBar {
  private element: HTMLElement;
  private workspaceEl: HTMLElement;
  private segButtons: HTMLButtonElement[];

  constructor(handlers: TitleBarHandlers) {
    this.element = document.createElement('header');
    this.element.className = 'titlebar';
    this.element.innerHTML = `
      <div class="tb-brand">
        <span class="tb-logo" aria-hidden="true">R</span>
        <button type="button" class="tb-workspace" data-act="folder" title="Open folder…">${icon('folder')}<span>Workspace</span></button>
      </div>
      <div class="tb-center">
        <button type="button" class="command-center" data-act="palette">
          ${icon('search')}<span class="label">Search commands or ask the agent</span><span class="kbd">Ctrl K</span>
        </button>
      </div>
      <div class="tb-right">
        <div class="seg" role="group" aria-label="Layout">
          ${LAYOUTS.map((l) => `<button type="button" data-layout="${l.id}" aria-pressed="false" title="${l.label} layout (${l.key})">${l.label}</button>`).join('')}
        </div>
      </div>
    `;
    this.workspaceEl = this.element.querySelector('.tb-workspace span')!;
    this.segButtons = Array.from(this.element.querySelectorAll<HTMLButtonElement>('[data-layout]'));

    this.element.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      const layout = target.closest<HTMLElement>('[data-layout]')?.dataset.layout as UiLayout | undefined;
      if (layout) handlers.onLayout(layout);
      const act = target.closest<HTMLElement>('[data-act]')?.dataset.act;
      if (act === 'palette') handlers.onCommandCenter();
      if (act === 'folder') handlers.onOpenFolder();
    });
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public setLayout(layout: UiLayout): void {
    this.segButtons.forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.layout === layout)));
  }

  public setWorkspace(name: string, root: string): void {
    this.workspaceEl.textContent = name;
    this.workspaceEl.parentElement!.title = `${root} — click to open another folder`;
  }
}
