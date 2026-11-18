import { basicSetup, EditorView } from 'codemirror';
import { keymap } from '@codemirror/view';
import { rust } from '@codemirror/lang-rust';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorState } from '@codemirror/state';
import * as api from '../api';

export interface EditorTab {
  name: string;
  path: string;
  content: string;
  isModified?: boolean;
}

const DEFAULT_FILES: EditorTab[] = [
  {
    name: 'main.rs',
    path: 'crates/ride-desktop/src/main.rs',
    content: `// R.I.D.E Desktop — Rust Intelligent Development Environment
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::DesktopState;
use std::sync::Mutex;
use tauri::Emitter;
use tracing::info;

fn main() {
    tracing_subscriber::fmt::init();

    info!(
        "Starting {} v{}",
        ride_core::RIDE_APP_NAME,
        ride_core::RIDE_VERSION
    );

    tauri::Builder::default()
        .manage(Mutex::new(DesktopState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::get_model_status,
            commands::send_agent_message,
            commands::get_active_skills,
            commands::toggle_skill,
            commands::get_diff_preview,
            commands::stage_diff_preview,
            commands::apply_diff,
            commands::rollback_diff,
            commands::get_active_layout,
            commands::set_active_layout,
            commands::get_moe_status,
            commands::save_file,
            commands::read_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running R.I.D.E desktop application");
}
`,
  },
  {
    name: 'commands.rs',
    path: 'crates/ride-desktop/src/commands.rs',
    content: `/// Tauri IPC commands for the R.I.D.E desktop application.
use ride_agent::{AgentSession, AgentSessionConfig, DiffEngine, DiffManager, UnifiedDiff};
use ride_core::UiLayout;
use ride_discovery::ModelDetector;
use ride_moe::{MoeEngineConfig, MoeEngineManager, MoeEngineStatus};
use ride_skills::SkillRegistry;
use serde::{Deserialize, Serialize};

#[tauri::command]
pub async fn get_model_status() -> Result<Vec<ProviderStatus>, String> {
    let detector = ModelDetector::default();
    let statuses = detector.scan_all().await;
    let models = detector.discover_all_models().await;
    // ...
    Ok(vec![])
}
`,
  },
  {
    name: 'Cargo.toml',
    path: 'crates/ride-desktop/Cargo.toml',
    content: `[package]
name = "ride-desktop"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
build = "build.rs"

[dependencies]
ride-core = { workspace = true }
ride-discovery = { workspace = true }
ride-moe = { workspace = true }
ride-skills = { workspace = true }
ride-agent = { workspace = true }
tauri = { version = "2", features = ["devtools", "custom-protocol"] }
`,
  },
];

export class EditorComponent {
  private element: HTMLElement;
  private tabs: EditorTab[] = [...DEFAULT_FILES];
  private activeTabIndex: number = 0;
  private editorView: EditorView | null = null;
  private editorContainer: HTMLElement | null = null;
  private filePickerInput: HTMLInputElement | null = null;
  private onSaveCallback?: (path: string) => void;
  private isProgrammaticUpdate: boolean = false;

  constructor(onSave?: (path: string) => void) {
    this.onSaveCallback = onSave;
    this.element = document.createElement('div');
    this.element.className = 'editor-component-container';
    this.renderSkeleton();
    this.loadRealDiskFiles();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  private setEditorContent(content: string): void {
    if (!this.editorView) return;
    this.isProgrammaticUpdate = true;
    try {
      this.editorView.dispatch({
        changes: { from: 0, to: this.editorView.state.doc.length, insert: content },
      });
    } finally {
      this.isProgrammaticUpdate = false;
    }
  }

  public setContent(content: string): void {
    this.setEditorContent(content);
    if (this.tabs[this.activeTabIndex]) {
      this.tabs[this.activeTabIndex].content = content;
      this.tabs[this.activeTabIndex].isModified = false;
      this.renderTabs();
    }
  }

  public getContent(): string {
    return this.editorView ? this.editorView.state.doc.toString() : '';
  }

  public getActiveTab(): EditorTab | null {
    return this.tabs[this.activeTabIndex] || null;
  }

  public createNewFile(name?: string, path?: string): void {
    const num = this.tabs.filter((t) => t.name.startsWith('untitled')).length + 1;
    const fileName = name || `untitled-${num}.rs`;
    const filePath = path || fileName;
    const newTab: EditorTab = {
      name: fileName,
      path: filePath,
      content: '// New buffer — edit code and press Ctrl+S to save\n',
      isModified: false,
    };
    this.tabs.push(newTab);
    this.switchTab(this.tabs.length - 1);
  }

  public async saveActiveFile(): Promise<boolean> {
    const activeTab = this.tabs[this.activeTabIndex];
    if (!activeTab) return false;

    if (activeTab.path.startsWith('untitled')) {
      const userPath = window.prompt('Save file as (enter path):', activeTab.path);
      if (!userPath || !userPath.trim()) return false;
      activeTab.path = userPath.trim();
      activeTab.name = activeTab.path.split(/[\/\\]/).pop() || activeTab.path;
    }

    const contentToSave = this.getContent();
    activeTab.content = contentToSave;

    try {
      await api.saveFile(activeTab.path, contentToSave);
      activeTab.isModified = false;
      this.renderTabs();
      this.flashSaveFeedback(true);
      if (this.onSaveCallback) {
        this.onSaveCallback(activeTab.path);
      }
      return true;
    } catch (err) {
      console.error(`[Editor] Failed to save ${activeTab.path}:`, err);
      this.flashSaveFeedback(false);
      return false;
    }
  }

  public async openFile(path: string, explicitContent?: string): Promise<void> {
    const existingIndex = this.tabs.findIndex((t) => t.path === path);
    if (existingIndex !== -1) {
      this.switchTab(existingIndex);
      return;
    }

    let fileContent = explicitContent;
    if (fileContent === undefined) {
      try {
        fileContent = await api.readFile(path);
      } catch (err) {
        console.warn(`[Editor] Could not read file from disk (${path}):`, err);
        fileContent = `// File: ${path}\n// Newly opened editor buffer\n`;
      }
    }

    const fileName = path.split(/[\/\\]/).pop() || path;
    const newTab: EditorTab = {
      name: fileName,
      path,
      content: fileContent,
      isModified: false,
    };

    this.tabs.push(newTab);
    this.switchTab(this.tabs.length - 1);
  }

  private async loadRealDiskFiles(): Promise<void> {
    for (let i = 0; i < this.tabs.length; i++) {
      const tab = this.tabs[i];
      try {
        const diskContent = await api.readFile(tab.path);
        if (diskContent) {
          tab.content = diskContent;
          if (i === this.activeTabIndex && this.editorView) {
            this.setEditorContent(diskContent);
            tab.isModified = false;
            this.renderTabs();
          }
        }
      } catch {
        // Fall back quietly to embedded template
      }
    }
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="editor-tab-bar">
        <div class="editor-tabs-scroll"></div>
        <div class="editor-tab-actions">
          <button class="editor-action-btn btn-new" title="New File (Ctrl+N)">
            <span class="btn-icon">✚</span>
            <span class="btn-text">New</span>
          </button>
          <button class="editor-action-btn btn-save" title="Save File (Ctrl+S)">
            <span class="btn-icon">💾</span>
            <span class="btn-text">Save</span>
          </button>
          <button class="editor-action-btn btn-open" title="Open File (Ctrl+O)">
            <span class="btn-icon">📂</span>
            <span class="btn-text">Open</span>
          </button>
          <button class="editor-action-btn btn-open-path" title="Open from Path...">
            <span class="btn-icon">📁</span>
            <span class="btn-text">Path</span>
          </button>
          <input type="file" class="editor-file-picker" style="display: none;" />
        </div>
      </div>
      <div class="editor-cm-wrapper"></div>
    `;

    this.filePickerInput = this.element.querySelector<HTMLInputElement>('.editor-file-picker');
    this.renderTabs();
    this.initCodeMirror();
    this.attachActionEvents();
  }

  private renderTabs(): void {
    const tabsContainer = this.element.querySelector<HTMLElement>('.editor-tabs-scroll');
    if (!tabsContainer) return;

    tabsContainer.innerHTML = this.tabs
      .map((tab, idx) => {
        const isActive = idx === this.activeTabIndex;
        const icon = tab.name.endsWith('.rs') ? '🦀' : tab.name.endsWith('.toml') ? '⚙️' : '📄';
        return `
        <div class="editor-tab ${isActive ? 'active' : ''}" data-index="${idx}" title="${tab.path}">
          <span class="tab-icon">${icon}</span>
          <span class="tab-title">${tab.name}${tab.isModified ? ' <span class="tab-dirty">●</span>' : ''}</span>
          <span class="tab-close" data-index="${idx}" title="Close tab">✕</span>
        </div>
      `;
      })
      .join('');

    tabsContainer.querySelectorAll<HTMLElement>('.editor-tab').forEach((el) => {
      el.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        if (target.classList.contains('tab-close')) return;
        const idx = Number(el.getAttribute('data-index'));
        this.switchTab(idx);
      });
    });

    tabsContainer.querySelectorAll<HTMLElement>('.tab-close').forEach((el) => {
      el.addEventListener('click', (e) => {
        e.stopPropagation();
        const idx = Number(el.getAttribute('data-index'));
        this.closeTab(idx);
      });
    });
  }

  private switchTab(index: number): void {
    if (index < 0 || index >= this.tabs.length) return;
    this.activeTabIndex = index;
    this.renderTabs();

    const currentTab = this.tabs[index];
    if (currentTab) {
      this.setEditorContent(currentTab.content);
    }
  }

  private closeTab(index: number): void {
    if (this.tabs.length <= 1) return; // Keep at least one tab
    this.tabs.splice(index, 1);
    if (this.activeTabIndex >= this.tabs.length) {
      this.activeTabIndex = this.tabs.length - 1;
    }
    this.renderTabs();
    this.switchTab(this.activeTabIndex);
  }

  private initCodeMirror(): void {
    this.editorContainer = this.element.querySelector<HTMLElement>('.editor-cm-wrapper');
    if (!this.editorContainer) return;

    const initialContent = this.tabs[this.activeTabIndex]?.content || '';

    const saveKeymap = keymap.of([
      {
        key: 'Mod-s',
        run: () => {
          void this.saveActiveFile();
          return true;
        },
      },
    ]);

    const startState = EditorState.create({
      doc: initialContent,
      extensions: [
        basicSetup,
        saveKeymap,
        rust(),
        oneDark,
        EditorView.updateListener.of((update) => {
          if (!this.isProgrammaticUpdate && update.docChanged && this.tabs[this.activeTabIndex]) {
            const currentTab = this.tabs[this.activeTabIndex];
            currentTab.content = update.state.doc.toString();
            if (!currentTab.isModified) {
              currentTab.isModified = true;
              this.renderTabs();
            }
          }
        }),
      ],
    });

    this.editorView = new EditorView({
      state: startState,
      parent: this.editorContainer,
    });
  }

  private attachActionEvents(): void {
    const newBtn = this.element.querySelector<HTMLButtonElement>('.btn-new');
    if (newBtn) {
      newBtn.addEventListener('click', () => {
        this.createNewFile();
      });
    }

    const saveBtn = this.element.querySelector<HTMLButtonElement>('.btn-save');
    if (saveBtn) {
      saveBtn.addEventListener('click', () => {
        void this.saveActiveFile();
      });
    }

    const openBtn = this.element.querySelector<HTMLButtonElement>('.btn-open');
    if (openBtn && this.filePickerInput) {
      openBtn.addEventListener('click', () => {
        this.filePickerInput?.click();
      });

      this.filePickerInput.addEventListener('change', (e: Event) => {
        const input = e.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = (event) => {
          const content = (event.target?.result as string) || '';
          const filePath = (file as any).path || file.name;
          void this.openFile(filePath, content);
          input.value = '';
        };
        reader.readAsText(file);
      });
    }

    const openPathBtn = this.element.querySelector<HTMLButtonElement>('.btn-open-path');
    if (openPathBtn) {
      openPathBtn.addEventListener('click', () => {
        const path = window.prompt('Enter file path to open (e.g. crates/ride-desktop/Cargo.toml):');
        if (path && path.trim()) {
          void this.openFile(path.trim());
        }
      });
    }
  }

  private flashSaveFeedback(success: boolean): void {
    const saveBtn = this.element.querySelector<HTMLButtonElement>('.btn-save');
    if (!saveBtn) return;

    const originalText = '<span class="btn-icon">💾</span> <span class="btn-text">Save</span>';
    if (success) {
      saveBtn.innerHTML = '<span class="btn-icon">✓</span> <span class="btn-text">Saved</span>';
      saveBtn.classList.add('saved');
    } else {
      saveBtn.innerHTML = '<span class="btn-icon">✕</span> <span class="btn-text">Failed</span>';
    }

    setTimeout(() => {
      saveBtn.innerHTML = originalText;
      saveBtn.classList.remove('saved');
    }, 1500);
  }
}
