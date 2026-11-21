import * as api from '../api';
import type { DirEntryInfo } from '../types';
import { icon } from '../icons';
import { esc, fileSwatch } from '../util';

/**
 * Lazily-loaded workspace explorer backed by the `list_dir` command.
 * Directory contents are fetched on first expand and cached until refresh.
 */
export class FileTree {
  private element: HTMLElement;
  private children = new Map<string, DirEntryInfo[]>();
  private expanded = new Set<string>(['']);
  private loading = new Set<string>();
  private errors = new Map<string, string>();
  private activePath = '';
  private modified = new Set<string>();
  private rootName = 'Workspace';
  private onOpen: (path: string) => void;

  constructor(onOpen: (path: string) => void) {
    this.onOpen = onOpen;
    this.element = document.createElement('div');
    this.element.className = 'tree';
    this.element.setAttribute('role', 'tree');
    this.element.setAttribute('aria-label', 'Workspace files');

    this.element.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest<HTMLElement>('[data-path]');
      if (!row) return;
      const path = row.dataset.path!;
      if (row.dataset.dir === '1') {
        void this.toggle(path);
      } else {
        this.onOpen(path);
      }
    });

    this.element.addEventListener('keydown', (e) => {
      const row = (e.target as HTMLElement).closest<HTMLElement>('[data-path]');
      if (!row) return;
      const rows = Array.from(this.element.querySelectorAll<HTMLElement>('[data-path]'));
      const i = rows.indexOf(row);
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        rows[Math.min(i + 1, rows.length - 1)]?.focus();
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        rows[Math.max(i - 1, 0)]?.focus();
      } else if (e.key === 'ArrowRight' && row.dataset.dir === '1' && !this.expanded.has(row.dataset.path!)) {
        e.preventDefault();
        void this.toggle(row.dataset.path!);
      } else if (e.key === 'ArrowLeft' && row.dataset.dir === '1' && this.expanded.has(row.dataset.path!)) {
        e.preventDefault();
        void this.toggle(row.dataset.path!);
      }
    });
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public setRootName(name: string): void {
    this.rootName = name;
    this.render();
  }

  public async refresh(): Promise<void> {
    const open = Array.from(this.expanded);
    this.children.clear();
    this.errors.clear();
    await Promise.all(open.map((p) => this.load(p)));
    this.render();
  }

  public collapseAll(): void {
    this.expanded = new Set(['']);
    this.render();
  }

  public setActive(path: string): void {
    if (path === this.activePath) return;
    this.activePath = path;
    // Reveal: expand every ancestor directory of the active file.
    const parts = path.split('/');
    let acc = '';
    const toLoad: string[] = [];
    for (let i = 0; i < parts.length - 1; i++) {
      acc = acc ? `${acc}/${parts[i]}` : parts[i];
      if (!this.expanded.has(acc)) {
        this.expanded.add(acc);
        toLoad.push(acc);
      }
    }
    if (toLoad.length) {
      void Promise.all(toLoad.map((p) => this.load(p))).then(() => this.render());
    } else {
      this.render();
    }
  }

  public setModified(paths: string[]): void {
    const next = new Set(paths);
    if (next.size === this.modified.size && paths.every((p) => this.modified.has(p))) return;
    this.modified = next;
    this.render();
  }

  public async init(): Promise<void> {
    await this.load('');
    this.render();
  }

  private async toggle(path: string): Promise<void> {
    if (this.expanded.has(path)) {
      this.expanded.delete(path);
      this.render();
      this.focusRow(path);
      return;
    }
    this.expanded.add(path);
    if (!this.children.has(path)) {
      this.render();
      await this.load(path);
    }
    this.render();
    this.focusRow(path);
  }

  private focusRow(path: string): void {
    this.element.querySelector<HTMLElement>(`[data-path="${CSS.escape(path)}"]`)?.focus();
  }

  private async load(path: string): Promise<void> {
    if (this.loading.has(path)) return;
    this.loading.add(path);
    try {
      const entries = await api.listDir(path || undefined);
      this.children.set(path, entries ?? []);
      this.errors.delete(path);
    } catch (err) {
      this.errors.set(path, String(err));
    } finally {
      this.loading.delete(path);
    }
  }

  private render(): void {
    const rows: string[] = [];
    const rootOpen = this.expanded.has('');
    rows.push(
      `<button type="button" class="tree-root" data-path="" data-dir="1" aria-expanded="${rootOpen}">${icon(rootOpen ? 'chevronDown' : 'chevronRight')}<span>${esc(this.rootName)}</span></button>`
    );
    if (rootOpen) this.renderLevel('', 0, rows);
    this.element.innerHTML = rows.join('');
  }

  private renderLevel(dir: string, depth: number, rows: string[]): void {
    const err = this.errors.get(dir);
    if (err) {
      rows.push(`<div class="tree-note" style="--depth:${depth}">${esc(err)}</div>`);
      return;
    }
    const entries = this.children.get(dir);
    if (!entries) {
      rows.push(`<div class="tree-note" style="padding-left:calc(26px + ${depth} * 14px)">Loading…</div>`);
      return;
    }
    if (entries.length === 0 && dir === '') {
      rows.push(`<div class="tree-note">This folder is empty.</div>`);
    }
    for (const entry of entries) {
      if (entry.is_dir) {
        const open = this.expanded.has(entry.path);
        rows.push(
          `<button type="button" class="tree-row" role="treeitem" style="--depth:${depth}" data-path="${esc(entry.path)}" data-dir="1" aria-expanded="${open}">${icon('chevronRight', 'chev')}<span class="name">${esc(entry.name)}</span></button>`
        );
        if (open) this.renderLevel(entry.path, depth + 1, rows);
      } else {
        const active = entry.path === this.activePath;
        const mod = this.modified.has(entry.path);
        rows.push(
          `<button type="button" class="tree-row${active ? ' active' : ''}" role="treeitem" style="--depth:${depth}" data-path="${esc(entry.path)}" aria-selected="${active}"><span style="width:12px" aria-hidden="true"></span>${fileSwatch(entry.path)}<span class="name">${esc(entry.name)}</span>${mod ? '<span class="mark" title="Unsaved changes">M</span>' : ''}</button>`
        );
      }
    }
  }
}
