import type { UiLayout } from '../types';

export interface CommandItem {
  id: string;
  label: string;
  sublabel?: string;
  icon: string;
  action: () => void;
}

export class CommandPalette {
  private overlay: HTMLElement;
  private input: HTMLInputElement;
  private list: HTMLElement;
  private isOpen: boolean = false;
  private items: CommandItem[] = [];
  private selectedIndex: number = 0;
  private onSelectLayout: (layout: UiLayout) => void;
  private onPromptAgent: (prompt: string) => void;
  private onOpenFile?: () => void;
  private onSaveFile?: () => void;
  private onNewFile?: () => void;

  constructor(
    onSelectLayout: (layout: UiLayout) => void,
    onPromptAgent: (prompt: string) => void,
    onOpenFile?: () => void,
    onSaveFile?: () => void,
    onNewFile?: () => void
  ) {
    this.onSelectLayout = onSelectLayout;
    this.onPromptAgent = onPromptAgent;
    this.onOpenFile = onOpenFile;
    this.onSaveFile = onSaveFile;
    this.onNewFile = onNewFile;

    this.overlay = document.createElement('div');
    this.overlay.className = 'command-palette-overlay';
    this.overlay.style.display = 'none';

    this.overlay.innerHTML = `
      <div class="command-palette-modal">
        <div class="palette-input-wrapper">
          <span class="palette-icon">✧</span>
          <input type="text" class="palette-input" placeholder="Type a command or ask R.I.D.E AI... (Ctrl+K)" />
          <span class="palette-kbd">Esc</span>
        </div>
        <div class="palette-results"></div>
      </div>
    `;

    this.input = this.overlay.querySelector<HTMLInputElement>('.palette-input')!;
    this.list = this.overlay.querySelector<HTMLElement>('.palette-results')!;

    this.initDefaultItems();
    this.attachEvents();
    document.body.appendChild(this.overlay);
  }

  public open(): void {
    this.isOpen = true;
    this.overlay.style.display = 'flex';
    this.input.value = '';
    this.selectedIndex = 0;
    this.renderItems('');
    setTimeout(() => this.input.focus(), 50);
  }

  public close(): void {
    this.isOpen = false;
    this.overlay.style.display = 'none';
  }

  public toggle(): void {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  private initDefaultItems(): void {
    this.items = [
      {
        id: 'file-save',
        label: 'File: Save Active File (Ctrl+S)',
        sublabel: 'Persist current CodeMirror buffer directly to disk via Rust IPC',
        icon: '💾',
        action: () => {
          this.close();
          this.onSaveFile?.();
        },
      },
      {
        id: 'file-open',
        label: 'File: Open File by Path (Ctrl+O)',
        sublabel: 'Load source file from disk into editor',
        icon: '📂',
        action: () => {
          this.close();
          this.onOpenFile?.();
        },
      },
      {
        id: 'file-new',
        label: 'File: New File Buffer (Ctrl+N)',
        sublabel: 'Create a new scratch buffer for manual editing',
        icon: '✚',
        action: () => {
          this.close();
          this.onNewFile?.();
        },
      },
      {
        id: 'layout-vscode',
        label: 'Switch Layout: VS Code',
        sublabel: 'Full IDE with activity bar, explorer, editor & agent dock',
        icon: '⊞',
        action: () => this.onSelectLayout('vs_code'),
      },
      {
        id: 'layout-claude',
        label: 'Switch Layout: Claude Code',
        sublabel: 'Terminal-first agent stream with live diffs & reasoning',
        icon: '❯_',
        action: () => this.onSelectLayout('claude_code'),
      },
      {
        id: 'layout-codex',
        label: 'Switch Layout: Codex Zen',
        sublabel: 'Distraction-free canvas with floating command overlay',
        icon: '✧',
        action: () => this.onSelectLayout('codex'),
      },
      {
        id: 'ask-analyze',
        label: 'Ask AI: Analyze current file syntax & lifetime safety',
        sublabel: 'Invokes local model with rust-expert skill',
        icon: '🦀',
        action: () => this.onPromptAgent('Review the current Rust code for lifetime and borrowing soundness.'),
      },
      {
        id: 'ask-diff',
        label: 'Ask AI: Generate unit tests for active commands',
        sublabel: 'Proposes tests with previewable unified diff',
        icon: '🧪',
        action: () => this.onPromptAgent('Generate comprehensive unit tests for the active commands module.'),
      },
    ];
  }

  private renderItems(query: string): void {
    const q = query.toLowerCase().trim();
    const matched = this.items.filter((item) => {
      if (!q) return true;
      return (
        item.label.toLowerCase().includes(q) ||
        (item.sublabel && item.sublabel.toLowerCase().includes(q))
      );
    });

    if (matched.length === 0) {
      // Dynamic AI prompt option
      this.list.innerHTML = `
        <div class="palette-item active" data-action="ai-query">
          <span class="item-icon">⚡</span>
          <div class="item-text">
            <div class="item-label">Ask R.I.D.E Agent: "${this.escapeHtml(query)}"</div>
            <div class="item-sub">Execute autonomous agent with loaded skills</div>
          </div>
        </div>
      `;
      return;
    }

    this.list.innerHTML = matched
      .map((item, idx) => {
        const isSelected = idx === this.selectedIndex;
        return `
        <div class="palette-item ${isSelected ? 'active' : ''}" data-idx="${idx}">
          <span class="item-icon">${item.icon}</span>
          <div class="item-text">
            <div class="item-label">${this.escapeHtml(item.label)}</div>
            ${item.sublabel ? `<div class="item-sub">${this.escapeHtml(item.sublabel)}</div>` : ''}
          </div>
        </div>
      `;
      })
      .join('');
  }

  private attachEvents(): void {
    this.overlay.addEventListener('click', (e) => {
      if (e.target === this.overlay) {
        this.close();
      }
    });

    this.input.addEventListener('input', () => {
      this.selectedIndex = 0;
      this.renderItems(this.input.value);
    });

    this.input.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        this.close();
        return;
      }

      const itemsCount = this.list.querySelectorAll('.palette-item').length;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.selectedIndex = (this.selectedIndex + 1) % itemsCount;
        this.renderItems(this.input.value);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.selectedIndex = (this.selectedIndex - 1 + itemsCount) % itemsCount;
        this.renderItems(this.input.value);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        this.executeCurrent();
      }
    });

    this.list.addEventListener('click', (e) => {
      const itemEl = (e.target as HTMLElement).closest<HTMLElement>('.palette-item');
      if (itemEl) {
        const idx = itemEl.getAttribute('data-idx');
        if (idx !== null) {
          this.selectedIndex = Number(idx);
          this.executeCurrent();
        } else if (itemEl.getAttribute('data-action') === 'ai-query') {
          this.executeCurrent();
        }
      }
    });
  }

  private executeCurrent(): void {
    const q = this.input.value.trim();
    const matched = this.items.filter((item) => {
      if (!q) return true;
      return (
        item.label.toLowerCase().includes(q.toLowerCase()) ||
        (item.sublabel && item.sublabel.toLowerCase().includes(q.toLowerCase()))
      );
    });

    if (matched.length > 0 && matched[this.selectedIndex]) {
      const selected = matched[this.selectedIndex];
      this.close();
      selected.action();
    } else if (q) {
      this.close();
      this.onPromptAgent(q);
    }
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.innerText = text;
    return div.innerHTML;
  }
}
