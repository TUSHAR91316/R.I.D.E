import { icon, type IconName } from '../icons';
import { esc } from '../util';

export interface CommandItem {
  id: string;
  group: string;
  label: string;
  sublabel?: string;
  icon: IconName;
  keybinding?: string;
  run: () => void;
}

/** Ctrl+K palette: filter commands, or send the typed text to the agent. */
export class CommandPalette {
  private overlay: HTMLElement;
  private input: HTMLInputElement;
  private list: HTMLElement;
  private items: CommandItem[] = [];
  private matched: CommandItem[] = [];
  private selected = 0;
  private previousFocus: HTMLElement | null = null;
  private onAsk: (prompt: string) => void;

  constructor(onAsk: (prompt: string) => void) {
    this.onAsk = onAsk;
    this.overlay = document.createElement('div');
    this.overlay.className = 'palette-overlay';
    this.overlay.hidden = true;
    this.overlay.innerHTML = `
      <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
        <div class="palette-input-row">
          ${icon('search')}
          <label class="sr-only" for="palette-input">Command</label>
          <input id="palette-input" type="text" placeholder="Type a command, or ask the agent…" spellcheck="false" autocomplete="off" role="combobox" aria-controls="palette-list" aria-expanded="true">
          <span class="kbd">Esc</span>
        </div>
        <div class="palette-list" id="palette-list" role="listbox"></div>
      </div>
    `;
    this.input = this.overlay.querySelector('input')!;
    this.list = this.overlay.querySelector('.palette-list')!;
    document.body.appendChild(this.overlay);

    this.overlay.addEventListener('mousedown', (e) => {
      if (e.target === this.overlay) this.close();
    });
    this.input.addEventListener('input', () => {
      this.selected = 0;
      this.render();
    });
    this.input.addEventListener('keydown', (e) => this.onKey(e));
    this.list.addEventListener('click', (e) => {
      const el = (e.target as HTMLElement).closest<HTMLElement>('[data-idx]');
      if (el) {
        this.selected = Number(el.dataset.idx);
        this.execute();
      }
    });
    this.list.addEventListener('mousemove', (e) => {
      const el = (e.target as HTMLElement).closest<HTMLElement>('[data-idx]');
      if (el && Number(el.dataset.idx) !== this.selected) {
        this.selected = Number(el.dataset.idx);
        this.updateSelection();
      }
    });
  }

  public setCommands(items: CommandItem[]): void {
    this.items = items;
  }

  public get isOpen(): boolean {
    return !this.overlay.hidden;
  }

  public open(initial = ''): void {
    this.previousFocus = document.activeElement as HTMLElement | null;
    this.overlay.hidden = false;
    this.input.value = initial;
    this.selected = 0;
    this.render();
    this.input.focus();
  }

  public close(): void {
    if (this.overlay.hidden) return;
    this.overlay.hidden = true;
    this.previousFocus?.focus?.();
  }

  public toggle(): void {
    if (this.isOpen) this.close();
    else this.open();
  }

  private score(item: CommandItem, q: string): number {
    const label = item.label.toLowerCase();
    if (label.startsWith(q)) return 3;
    if (label.includes(q)) return 2;
    if (item.group.toLowerCase().includes(q) || item.sublabel?.toLowerCase().includes(q)) return 1;
    // subsequence match ("tglag" → "Toggle agent panel")
    let i = 0;
    for (const ch of label) if (ch === q[i]) i++;
    return i === q.length ? 0.5 : 0;
  }

  private render(): void {
    const q = this.input.value.trim().toLowerCase();
    this.matched = q
      ? this.items
          .map((item) => ({ item, s: this.score(item, q) }))
          .filter((x) => x.s > 0)
          .sort((a, b) => b.s - a.s)
          .map((x) => x.item)
      : this.items;

    const rows: string[] = [];
    let lastGroup = '';
    this.matched.forEach((item, idx) => {
      if (!q && item.group !== lastGroup) {
        rows.push(`<div class="palette-group">${esc(item.group)}</div>`);
        lastGroup = item.group;
      }
      rows.push(this.row(idx, item.icon, item.label, item.sublabel, item.keybinding));
    });
    if (q) {
      rows.push(this.row(this.matched.length, 'sparkle', `Ask the agent: “${this.input.value.trim()}”`, 'Send this as a prompt to the local model'));
    }
    if (this.selected > this.count() - 1) this.selected = Math.max(0, this.count() - 1);
    this.list.innerHTML = rows.join('');
    this.updateSelection();
  }

  private row(idx: number, ic: IconName, label: string, sub?: string, kb?: string): string {
    return `<button type="button" class="palette-item" role="option" data-idx="${idx}" id="pi-${idx}">
      ${icon(ic)}<span class="pi-text"><span class="pi-label">${esc(label)}</span>${sub ? `<span class="pi-sub">${esc(sub)}</span>` : ''}</span>
      ${kb ? `<span class="kbd">${esc(kb)}</span>` : ''}
    </button>`;
  }

  private count(): number {
    return this.matched.length + (this.input.value.trim() ? 1 : 0);
  }

  private updateSelection(): void {
    this.list.querySelectorAll<HTMLElement>('.palette-item').forEach((el) => {
      const on = Number(el.dataset.idx) === this.selected;
      el.classList.toggle('active', on);
      el.setAttribute('aria-selected', String(on));
      if (on) {
        el.scrollIntoView({ block: 'nearest' });
        this.input.setAttribute('aria-activedescendant', el.id);
      }
    });
  }

  private onKey(e: KeyboardEvent): void {
    const n = this.count();
    if (e.key === 'Escape') {
      e.preventDefault();
      this.close();
    } else if (e.key === 'ArrowDown' && n > 0) {
      e.preventDefault();
      this.selected = (this.selected + 1) % n;
      this.updateSelection();
    } else if (e.key === 'ArrowUp' && n > 0) {
      e.preventDefault();
      this.selected = (this.selected - 1 + n) % n;
      this.updateSelection();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      this.execute();
    }
  }

  private execute(): void {
    const item = this.matched[this.selected];
    const q = this.input.value.trim();
    this.close();
    if (item) item.run();
    else if (q) this.onAsk(q);
  }
}
