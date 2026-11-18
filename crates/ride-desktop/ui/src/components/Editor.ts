import { basicSetup, EditorView } from 'codemirror';
import { rust } from '@codemirror/lang-rust';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorState } from '@codemirror/state';

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
            commands::apply_diff,
            commands::rollback_diff,
            commands::get_active_layout,
            commands::set_active_layout,
            commands::get_moe_status,
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
tauri = { version = "2", features = ["devtools"] }
`,
  },
];

export class EditorComponent {
  private element: HTMLElement;
  private tabs: EditorTab[] = [...DEFAULT_FILES];
  private activeTabIndex: number = 0;
  private editorView: EditorView | null = null;
  private editorContainer: HTMLElement | null = null;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'editor-component-container';
    this.renderSkeleton();
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public setContent(content: string): void {
    if (this.editorView) {
      this.editorView.dispatch({
        changes: { from: 0, to: this.editorView.state.doc.length, insert: content },
      });
      if (this.tabs[this.activeTabIndex]) {
        this.tabs[this.activeTabIndex].content = content;
      }
    }
  }

  public getContent(): string {
    return this.editorView ? this.editorView.state.doc.toString() : '';
  }

  private renderSkeleton(): void {
    this.element.innerHTML = `
      <div class="editor-tab-bar"></div>
      <div class="editor-cm-wrapper"></div>
    `;

    this.renderTabs();
    this.initCodeMirror();
  }

  private renderTabs(): void {
    const tabBar = this.element.querySelector<HTMLElement>('.editor-tab-bar');
    if (!tabBar) return;

    tabBar.innerHTML = this.tabs
      .map((tab, idx) => {
        const isActive = idx === this.activeTabIndex;
        return `
        <div class="editor-tab ${isActive ? 'active' : ''}" data-index="${idx}" title="${tab.path}">
          <span class="tab-icon">🦀</span>
          <span class="tab-title">${tab.name}</span>
          <span class="tab-close" data-index="${idx}">✕</span>
        </div>
      `;
      })
      .join('');

    tabBar.querySelectorAll<HTMLElement>('.editor-tab').forEach((el) => {
      el.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        if (target.classList.contains('tab-close')) return;
        const idx = Number(el.getAttribute('data-index'));
        this.switchTab(idx);
      });
    });

    tabBar.querySelectorAll<HTMLElement>('.tab-close').forEach((el) => {
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
    if (this.editorView && currentTab) {
      this.editorView.dispatch({
        changes: { from: 0, to: this.editorView.state.doc.length, insert: currentTab.content },
      });
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

    const startState = EditorState.create({
      doc: initialContent,
      extensions: [
        basicSetup,
        rust(),
        oneDark,
        EditorView.updateListener.of((update) => {
          if (update.docChanged && this.tabs[this.activeTabIndex]) {
            this.tabs[this.activeTabIndex].content = update.state.doc.toString();
          }
        }),
      ],
    });

    this.editorView = new EditorView({
      state: startState,
      parent: this.editorContainer,
    });
  }
}
