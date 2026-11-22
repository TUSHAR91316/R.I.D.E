import { basicSetup } from 'codemirror';
import { EditorView, keymap } from '@codemirror/view';
import { Compartment, EditorState, type Extension, type Text } from '@codemirror/state';
import { HighlightStyle, StreamLanguage, syntaxHighlighting, type LanguageSupport } from '@codemirror/language';
import { indentWithTab } from '@codemirror/commands';
import { tags as t } from '@lezer/highlight';
import * as api from '../api';
import { icon, rideLogoSvg } from '../icons';
import { askText, basename, confirmDialog, esc, fileKind, fileSwatch, kindLabel, toast, type FileKind } from '../util';

export interface EditorTab {
  name: string;
  path: string;
  state: EditorState;
  /** Document as last loaded/saved — used for dirty detection. */
  savedDoc: Text;
  isModified: boolean;
  isUntitled: boolean;
  scrollTop: number;
}

export interface CursorInfo {
  line: number;
  col: number;
  selected: number;
  language: string;
}

type Listener = () => void;

// ── Theme: driven entirely by the CSS tokens in style.css ──────────────────

const graphiteTheme = EditorView.theme({
  '&': {
    height: '100%',
    color: 'var(--syn-text)',
    backgroundColor: 'var(--bg-editor)',
    fontSize: '13px',
  },
  // CodeMirror's default focus ring is a dotted outline around the whole editor.
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': {
    fontFamily: 'var(--font-mono)',
    lineHeight: '1.62',
  },
  '.cm-content': { caretColor: 'var(--accent)', padding: '6px 0' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--accent)', borderLeftWidth: '2px' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
    backgroundColor: 'var(--selection) !important',
  },
  '.cm-activeLine': { backgroundColor: 'color-mix(in srgb, var(--bg-hover) 55%, transparent)' },
  '.cm-gutters': {
    backgroundColor: 'var(--bg-editor)',
    color: 'var(--text-faint)',
    border: 'none',
    paddingLeft: '8px',
  },
  '.cm-lineNumbers .cm-gutterElement': { padding: '0 12px 0 8px', minWidth: '36px' },
  '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--text)' },
  '.cm-foldGutter .cm-gutterElement': { color: 'var(--text-dim)' },
  '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': {
    backgroundColor: 'var(--bg-active)',
    outline: '1px solid var(--border-strong)',
    color: 'inherit',
  },
  '.cm-searchMatch': { backgroundColor: 'var(--accent-soft)', outline: '1px solid var(--accent)' },
  '.cm-selectionMatch': { backgroundColor: 'var(--bg-active)' },
  '.cm-panels': { backgroundColor: 'var(--bg-side)', color: 'var(--text)', borderColor: 'var(--border)' },
  '.cm-panel input, .cm-panel button': { fontFamily: 'var(--font-ui)' },
  '.cm-tooltip': {
    backgroundColor: 'var(--bg-elev)',
    border: '1px solid var(--border-strong)',
    borderRadius: '8px',
    boxShadow: 'var(--shadow-pop)',
  },
  '.cm-tooltip-autocomplete > ul > li[aria-selected]': {
    backgroundColor: 'var(--accent-soft)',
    color: 'var(--text)',
  },
  '.cm-foldPlaceholder': { backgroundColor: 'var(--bg-hover)', border: 'none', color: 'var(--text-muted)' },
});

const graphiteHighlight = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.moduleKeyword, t.operatorKeyword, t.modifier], color: 'var(--syn-keyword)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.labelName], color: 'var(--syn-fn)' },
  { tag: [t.string, t.special(t.string), t.regexp, t.character], color: 'var(--syn-string)' },
  { tag: [t.typeName, t.className, t.namespace, t.standard(t.typeName)], color: 'var(--syn-type)' },
  { tag: [t.number, t.bool, t.null, t.atom], color: 'var(--syn-number)' },
  { tag: [t.macroName, t.special(t.variableName)], color: 'var(--syn-macro)' },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: 'var(--syn-comment)', fontStyle: 'italic' },
  { tag: [t.meta, t.annotation, t.processingInstruction], color: 'var(--syn-meta)' },
  { tag: [t.propertyName, t.attributeName], color: 'var(--syn-fn)' },
  { tag: [t.tagName], color: 'var(--syn-keyword)' },
  { tag: t.heading, color: 'var(--syn-fn)', fontWeight: '600' },
  { tag: t.strong, fontWeight: '600' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.link, color: 'var(--syn-fn)', textDecoration: 'underline' },
  { tag: t.invalid, color: 'var(--err)' },
]);

// ── Lazy language loading (keeps the initial bundle small) ─────────────────

const languageCache = new Map<FileKind, Promise<Extension>>();

function loadLanguage(kind: FileKind): Promise<Extension> {
  let pending = languageCache.get(kind);
  if (!pending) {
    pending = (async (): Promise<Extension> => {
      switch (kind) {
        case 'rust':
          return (await import('@codemirror/lang-rust')).rust();
        case 'ts':
          return (await import('@codemirror/lang-javascript')).javascript({ typescript: true, jsx: true });
        case 'js':
          return (await import('@codemirror/lang-javascript')).javascript({ jsx: true });
        case 'json':
          return (await import('@codemirror/lang-json')).json();
        case 'md':
          return (await import('@codemirror/lang-markdown')).markdown() as LanguageSupport;
        case 'css':
          return (await import('@codemirror/lang-css')).css();
        case 'html':
          return (await import('@codemirror/lang-html')).html();
        case 'py':
          return (await import('@codemirror/lang-python')).python();
        case 'toml':
          return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/toml')).toml);
        case 'yaml':
          return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/yaml')).yaml);
        case 'shell':
          return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/shell')).shell);
        default:
          return [];
      }
    })().catch((err) => {
      console.warn(`[Editor] language '${kind}' failed to load`, err);
      return [];
    });
    languageCache.set(kind, pending);
  }
  return pending;
}

const WELCOME = `<div class="editor-empty">
  <div class="logo-xl" aria-hidden="true">${rideLogoSvg(64)}</div>
  <div class="shortcut-grid">
    <span>Command palette</span><span class="kbd">Ctrl K</span>
    <span>Open file by path</span><span class="kbd">Ctrl O</span>
    <span>New file</span><span class="kbd">Ctrl N</span>
    <span>Toggle side bar</span><span class="kbd">Ctrl B</span>
    <span>Toggle agent panel</span><span class="kbd">Ctrl J</span>
    <span>Switch layout</span><span class="kbd">Ctrl 1 · 2 · 3</span>
  </div>
</div>`;

export class EditorComponent {
  private element: HTMLElement;
  private tabsEl!: HTMLElement;
  private crumbsEl!: HTMLElement;
  private hostEl!: HTMLElement;
  private emptyEl!: HTMLElement;
  private view!: EditorView;
  private tabs: EditorTab[] = [];
  private activeIndex = -1;
  private language = new Compartment();
  private baseExtensions: Extension[];
  private listeners = new Set<Listener>();
  private cursorListeners = new Set<(c: CursorInfo) => void>();
  private untitledCounter = 0;
  /** In-flight reads keyed by path, so double-clicks don't open duplicate tabs. */
  private pendingOpens = new Map<string, Promise<void>>();

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'editor';

    this.baseExtensions = [
      basicSetup,
      keymap.of([indentWithTab]),
      graphiteTheme,
      syntaxHighlighting(graphiteHighlight),
      EditorView.updateListener.of((u) => {
        const tab = this.tabs[this.activeIndex];
        if (!tab) return;
        if (u.docChanged) {
          const dirty = !u.state.doc.eq(tab.savedDoc);
          if (dirty !== tab.isModified) {
            tab.isModified = dirty;
            this.renderTabs();
            this.emit();
          }
        }
        if (u.docChanged || u.selectionSet) this.emitCursor(u.state);
      }),
    ];

    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  /** Notified whenever tabs, the active tab or dirty flags change. */
  public onChange(fn: Listener): void {
    this.listeners.add(fn);
  }

  public onCursor(fn: (c: CursorInfo) => void): void {
    this.cursorListeners.add(fn);
  }

  public getActiveTab(): EditorTab | null {
    return this.tabs[this.activeIndex] ?? null;
  }

  public getTabs(): readonly EditorTab[] {
    return this.tabs;
  }

  public hasUnsavedChanges(): boolean {
    return this.tabs.some((tab) => tab.isModified);
  }

  public focus(): void {
    if (this.activeIndex >= 0) this.view.focus();
  }

  /** Selected text, or the whole document when nothing is selected. */
  public getSelectionOrDoc(): {
    text: string;
    isSelection: boolean;
    fromLine: number;
    toLine: number;
    from: number;
    to: number;
  } {
    const state = this.view.state;
    const sel = state.selection.main;
    if (this.activeIndex < 0) return { text: '', isSelection: false, fromLine: 0, toLine: 0, from: 0, to: 0 };
    if (sel.empty) {
      return { text: state.doc.toString(), isSelection: false, fromLine: 1, toLine: state.doc.lines, from: 0, to: state.doc.length };
    }
    return {
      text: state.sliceDoc(sel.from, sel.to),
      isSelection: true,
      fromLine: state.doc.lineAt(sel.from).number,
      toLine: state.doc.lineAt(sel.to).number,
      from: sel.from,
      to: sel.to,
    };
  }

  public sliceDoc(from: number, to: number): string | null {
    if (this.activeIndex < 0 || to > this.view.state.doc.length) return null;
    return this.view.state.sliceDoc(from, to);
  }

  /** Replace a range in the active document (undoable with Ctrl+Z). */
  public replaceRange(from: number, to: number, text: string): boolean {
    if (this.activeIndex < 0) return false;
    const len = this.view.state.doc.length;
    if (from > len || to > len) return false;
    this.view.dispatch({ changes: { from, to, insert: text }, selection: { anchor: from + text.length } });
    this.view.focus();
    return true;
  }

  public createNewFile(): void {
    this.untitledCounter += 1;
    const name = `untitled-${this.untitledCounter}`;
    this.addTab(name, '', true);
  }

  public async openFile(path: string, explicitContent?: string, line?: number): Promise<void> {
    const existing = this.tabs.findIndex((tab) => tab.path === path);
    if (existing !== -1) {
      this.switchTab(existing);
      if (line) this.revealLine(line);
      return;
    }

    const inflight = this.pendingOpens.get(path);
    if (inflight) {
      await inflight;
      if (line) this.revealLine(line);
      return;
    }

    const task = (async () => {
      let content = explicitContent;
      if (content === undefined) {
        try {
          content = await api.readFile(path);
        } catch (err) {
          toast(`Could not open ${basename(path)}: ${String(err)}`, 'err', 4000);
          return;
        }
      }
      this.addTab(path, content, false);
      if (line) this.revealLine(line);
    })();

    this.pendingOpens.set(path, task);
    try {
      await task;
    } finally {
      this.pendingOpens.delete(path);
    }
  }

  public async saveActiveFile(): Promise<boolean> {
    const tab = this.getActiveTab();
    if (!tab) return false;
    this.syncActiveState();

    if (tab.isUntitled) {
      const target = await askText('Save as', {
        message: 'Path relative to the workspace root, or an absolute path.',
        value: `${tab.name}.rs`,
        confirmLabel: 'Save',
      });
      if (!target) return false;
      tab.path = target.replace(/\\/g, '/');
      tab.name = basename(tab.path);
      tab.isUntitled = false;
      void this.applyLanguage(tab);
    }

    try {
      await api.saveFile(tab.path, tab.state.doc.toString());
      tab.savedDoc = tab.state.doc;
      tab.isModified = false;
      this.renderTabs();
      this.renderCrumbs();
      this.emit();
      toast(`Saved ${tab.name}`, 'ok', 1600);
      return true;
    } catch (err) {
      toast(`Save failed: ${String(err)}`, 'err', 5000);
      return false;
    }
  }

  public async closeActiveTab(): Promise<void> {
    if (this.activeIndex >= 0) await this.closeTab(this.activeIndex);
  }

  private findTabIndex(path: string): number {
    const norm = path.replace(/\\/g, '/');
    return this.tabs.findIndex((tab) => tab.path === norm || norm.endsWith('/' + tab.path) || tab.path.endsWith('/' + norm));
  }

  /** True if `path` is open with unsaved edits. */
  public isDirty(path: string): boolean {
    const idx = this.findTabIndex(path);
    return idx !== -1 && this.tabs[idx].isModified;
  }

  /** Reload a file from disk if it is open and unmodified (e.g. after an applied diff). */
  public async reloadIfClean(path: string): Promise<void> {
    const idx = this.findTabIndex(path);
    if (idx === -1 || this.tabs[idx].isModified) return;
    try {
      const content = await api.readFile(this.tabs[idx].path);
      const tab = this.tabs[idx];
      if (idx === this.activeIndex) {
        this.view.dispatch({ changes: { from: 0, to: this.view.state.doc.length, insert: content } });
        tab.state = this.view.state;
      } else {
        tab.state = tab.state.update({ changes: { from: 0, to: tab.state.doc.length, insert: content } }).state;
      }
      tab.savedDoc = tab.state.doc;
      tab.isModified = false;
      this.renderTabs();
      this.emit();
    } catch {
      /* file may have been removed; leave the buffer as-is */
    }
  }

  // ── Internals ────────────────────────────────────────────────────────────

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="tabs">
        <div class="tabs-scroll" role="tablist" aria-label="Open editors"></div>
        <div class="tab-actions">
          <button type="button" class="icon-btn" data-act="new" title="New file (Ctrl+N)" aria-label="New file">${icon('fileAdd')}</button>
          <button type="button" class="icon-btn" data-act="save" title="Save (Ctrl+S)" aria-label="Save">${icon('save')}</button>
        </div>
      </div>
      <div class="breadcrumbs" aria-label="Breadcrumbs"></div>
      <div class="cm-host"></div>
    `;
    this.tabsEl = this.element.querySelector('.tabs-scroll')!;
    this.crumbsEl = this.element.querySelector('.breadcrumbs')!;
    this.hostEl = this.element.querySelector('.cm-host')!;

    this.emptyEl = document.createElement('div');
    this.emptyEl.style.display = 'contents';
    this.emptyEl.innerHTML = WELCOME;

    this.view = new EditorView({
      state: EditorState.create({ doc: '', extensions: this.extensionsFor() }),
      parent: this.hostEl,
    });

    // One delegated listener for the whole tab strip (no per-render rebinding).
    this.tabsEl.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      const tabEl = target.closest<HTMLElement>('.tab');
      if (!tabEl) return;
      const idx = Number(tabEl.dataset.index);
      if (target.closest('.tab-close')) {
        e.stopPropagation();
        void this.closeTab(idx);
      } else {
        this.switchTab(idx);
      }
    });
    this.tabsEl.addEventListener('auxclick', (e) => {
      const tabEl = (e.target as HTMLElement).closest<HTMLElement>('.tab');
      if (tabEl && e.button === 1) {
        e.preventDefault();
        void this.closeTab(Number(tabEl.dataset.index));
      }
    });
    this.element.querySelector('.tab-actions')!.addEventListener('click', (e) => {
      const act = (e.target as HTMLElement).closest<HTMLElement>('[data-act]')?.dataset.act;
      if (act === 'new') this.createNewFile();
      if (act === 'save') void this.saveActiveFile();
    });

    this.renderTabs();
    this.renderCrumbs();
    this.updateEmptyState();
  }

  private extensionsFor(): Extension[] {
    return [...this.baseExtensions, this.language.of([])];
  }

  private addTab(path: string, content: string, untitled: boolean): void {
    this.syncActiveState();
    const state = EditorState.create({ doc: content, extensions: this.extensionsFor() });
    const tab: EditorTab = {
      name: basename(path),
      path: path.replace(/\\/g, '/'),
      state,
      savedDoc: state.doc,
      isModified: false,
      isUntitled: untitled,
      scrollTop: 0,
    };
    this.tabs.push(tab);
    this.switchTab(this.tabs.length - 1, true);
    void this.applyLanguage(tab);
  }

  private async applyLanguage(tab: EditorTab): Promise<void> {
    const ext = await loadLanguage(tab.isUntitled ? 'text' : fileKind(tab.path));
    if (!this.tabs.includes(tab)) return;
    const effects = this.language.reconfigure(ext);
    if (this.tabs[this.activeIndex] === tab) {
      this.view.dispatch({ effects });
      tab.state = this.view.state;
    } else {
      tab.state = tab.state.update({ effects }).state;
    }
  }

  /** Persist the live view state into the active tab before switching away. */
  private syncActiveState(): void {
    const tab = this.tabs[this.activeIndex];
    if (tab) {
      tab.state = this.view.state;
      tab.scrollTop = this.view.scrollDOM.scrollTop;
    }
  }

  private switchTab(index: number, skipSync = false): void {
    if (index < 0 || index >= this.tabs.length) return;
    if (!skipSync && index === this.activeIndex) {
      this.view.focus();
      return;
    }
    if (!skipSync) this.syncActiveState();
    this.activeIndex = index;
    const tab = this.tabs[index];
    // Each tab owns its EditorState, so undo history never leaks across files.
    this.view.setState(tab.state);
    const top = tab.scrollTop;
    requestAnimationFrame(() => {
      this.view.scrollDOM.scrollTop = top;
    });
    this.renderTabs();
    this.renderCrumbs();
    this.updateEmptyState();
    this.emitCursor(tab.state);
    this.emit();
    this.view.focus();
  }

  private async closeTab(index: number): Promise<void> {
    const tab = this.tabs[index];
    if (!tab) return;
    if (index === this.activeIndex) this.syncActiveState();
    if (tab.isModified) {
      const discard = await confirmDialog(`Close ${tab.name} without saving?`, {
        message: 'Your unsaved changes will be lost.',
        confirmLabel: 'Discard changes',
      });
      if (!discard) return;
    }
    const idx = this.tabs.indexOf(tab);
    if (idx === -1) return;
    this.tabs.splice(idx, 1);

    if (this.tabs.length === 0) {
      this.activeIndex = -1;
      this.view.setState(EditorState.create({ doc: '', extensions: this.extensionsFor() }));
      this.renderTabs();
      this.renderCrumbs();
      this.updateEmptyState();
      this.emit();
      return;
    }
    if (idx === this.activeIndex) {
      this.activeIndex = -1;
      this.switchTab(Math.min(idx, this.tabs.length - 1), true);
    } else {
      if (idx < this.activeIndex) this.activeIndex -= 1;
      this.renderTabs();
      this.emit();
    }
  }

  private revealLine(line: number): void {
    const doc = this.view.state.doc;
    const ln = doc.line(Math.max(1, Math.min(line, doc.lines)));
    this.view.dispatch({
      selection: { anchor: ln.from, head: ln.to },
      effects: EditorView.scrollIntoView(ln.from, { y: 'center' }),
    });
    this.view.focus();
  }

  private renderTabs(): void {
    this.tabsEl.innerHTML = this.tabs
      .map((tab, idx) => {
        const active = idx === this.activeIndex;
        return `<div class="tab${active ? ' active' : ''}${tab.isModified ? ' dirty' : ''}" role="tab" aria-selected="${active}" data-index="${idx}" title="${esc(tab.path)}">
          ${fileSwatch(tab.path)}<span>${esc(tab.name)}</span>
          <span class="tab-dirty" aria-label="Unsaved changes"></span>
          <button type="button" class="tab-close" aria-label="Close ${esc(tab.name)}">${icon('close')}</button>
        </div>`;
      })
      .join('');
    this.tabsEl.querySelector('.tab.active')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }

  private renderCrumbs(): void {
    const tab = this.getActiveTab();
    if (!tab) {
      this.crumbsEl.innerHTML = '';
      return;
    }
    const parts = tab.path.split('/').filter(Boolean);
    this.crumbsEl.innerHTML = parts
      .map((p, i) => `<span class="${i === parts.length - 1 ? 'last' : ''}">${esc(p)}</span>`)
      .join('<span class="sep" aria-hidden="true">›</span>');
  }

  private updateEmptyState(): void {
    const empty = this.activeIndex < 0;
    this.view.dom.style.display = empty ? 'none' : '';
    if (empty && !this.emptyEl.isConnected) this.hostEl.appendChild(this.emptyEl);
    if (!empty && this.emptyEl.isConnected) this.emptyEl.remove();
  }

  private emit(): void {
    this.listeners.forEach((fn) => fn());
  }

  private emitCursor(state: EditorState): void {
    if (this.cursorListeners.size === 0) return;
    const tab = this.getActiveTab();
    const sel = state.selection.main;
    const line = state.doc.lineAt(sel.head);
    const info: CursorInfo = {
      line: line.number,
      col: sel.head - line.from + 1,
      selected: Math.abs(sel.to - sel.from),
      language: tab ? kindLabel(tab.isUntitled ? 'text' : fileKind(tab.path)) : '',
    };
    this.cursorListeners.forEach((fn) => fn(info));
  }
}
