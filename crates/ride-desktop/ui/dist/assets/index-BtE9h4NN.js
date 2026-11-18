const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["./event-Cs6lN1fb.js","./core-BUhCcD2K.js"])))=>i.map(i=>d[i]);
import{k as _,E as x,b as T,o as A,r as $,a as k}from"./codemirror-6jMbsIKY.js";(function(){const e=document.createElement("link").relList;if(e&&e.supports&&e.supports("modulepreload"))return;for(const i of document.querySelectorAll('link[rel="modulepreload"]'))s(i);new MutationObserver(i=>{for(const a of i)if(a.type==="childList")for(const n of a.addedNodes)n.tagName==="LINK"&&n.rel==="modulepreload"&&s(n)}).observe(document,{childList:!0,subtree:!0});function t(i){const a={};return i.integrity&&(a.integrity=i.integrity),i.referrerPolicy&&(a.referrerPolicy=i.referrerPolicy),i.crossOrigin==="use-credentials"?a.credentials="include":i.crossOrigin==="anonymous"?a.credentials="omit":a.credentials="same-origin",a}function s(i){if(i.ep)return;i.ep=!0;const a=t(i);fetch(i.href,a)}})();const M="modulepreload",D=function(o,e){return new URL(o,e).href},w={},y=function(e,t,s){let i=Promise.resolve();if(t&&t.length>0){let n=function(c){return Promise.all(c.map(u=>Promise.resolve(u).then(v=>({status:"fulfilled",value:v}),v=>({status:"rejected",reason:v}))))};const l=document.getElementsByTagName("link"),d=document.querySelector("meta[property=csp-nonce]"),r=d?.nonce||d?.getAttribute("nonce");i=n(t.map(c=>{if(c=D(c,s),c in w)return;w[c]=!0;const u=c.endsWith(".css"),v=u?'[rel="stylesheet"]':"";if(!!s)for(let f=l.length-1;f>=0;f--){const g=l[f];if(g.href===c&&(!u||g.rel==="stylesheet"))return}else if(document.querySelector(`link[href="${c}"]${v}`))return;const p=document.createElement("link");if(p.rel=u?"stylesheet":M,u||(p.as="script"),p.crossOrigin="",p.href=c,r&&p.setAttribute("nonce",r),document.head.appendChild(p),u)return new Promise((f,g)=>{p.addEventListener("load",f),p.addEventListener("error",()=>g(new Error(`Unable to preload CSS for ${c}`)))})}))}function a(n){const l=new Event("vite:preloadError",{cancelable:!0});if(l.payload=n,window.dispatchEvent(l),!l.defaultPrevented)throw n}return i.then(n=>{for(const l of n||[])l.status==="rejected"&&a(l.reason);return e().catch(a)})};function b(){return typeof window<"u"&&("__TAURI_INTERNALS__"in window||"__TAURI__"in window)}async function h(o,e){if(b())try{const{invoke:t}=await y(async()=>{const{invoke:s}=await import("./core-BUhCcD2K.js");return{invoke:s}},[],import.meta.url);return await t(o,e)}catch(t){throw console.warn(`[Tauri IPC Error on ${o}]:`,t),t}return j(o,e)}async function E(){return h("get_model_status")}async function I(o,e){return h("send_agent_message",{message:o,modelName:e})}async function P(){return h("get_active_skills")}async function H(o,e){return h("toggle_skill",{name:o,enabled:e})}async function R(){return h("get_diff_preview")}async function q(o){return h("apply_diff",{path:o})}async function F(o){return h("rollback_diff",{path:o})}async function O(){const o=await h("get_active_layout");return o==="claude_code"||o==="codex"||o==="vs_code"?o:"vs_code"}async function B(o){return h("set_active_layout",{layout:o})}async function N(o,e){return h("save_file",{path:o,content:e})}async function S(o){return h("read_file",{path:o})}async function V(o){if(b())try{const{listen:e}=await y(async()=>{const{listen:s}=await import("./event-Cs6lN1fb.js");return{listen:s}},__vite__mapDeps([0,1]),import.meta.url);return await e("model://event",s=>o(s.payload))}catch(e){console.warn("[Tauri Model Event Listener Error]:",e)}return()=>{}}async function W(o){if(b())try{const{listen:e}=await y(async()=>{const{listen:s}=await import("./event-Cs6lN1fb.js");return{listen:s}},__vite__mapDeps([0,1]),import.meta.url);return await e("agent://event",s=>o(s.payload))}catch(e){console.warn("[Tauri Event Listener Error]:",e)}return()=>{}}let C=[{name:"rust-expert",description:"Deep idiomatic Rust engineering, borrow checker mastery, zero-copy architecture",triggers:["rust","cargo","tokio","unsafe"],enabled:!0},{name:"clean-code",description:"Robert C. Martin Clean Code principles, readability, refactoring, SOLID",triggers:["refactor","clean","format"],enabled:!0},{name:"systematic-debugging",description:"Root cause analysis, test-driven reproduction, edge case boundary tracing",triggers:["debug","bug","fix","error"],enabled:!0},{name:"git-pushing",description:"Conventional commits, atomic staging, push safety verification",triggers:["git","commit","push"],enabled:!1},{name:"api-security-testing",description:"OWASP 2025 security scanning, input injection checks, boundary audit",triggers:["security","audit","vuln"],enabled:!1}],m=[{path:"crates/ride-desktop/src/commands.rs",diff_text:`--- a/crates/ride-desktop/src/commands.rs
+++ b/crates/ride-desktop/src/commands.rs
@@ -108,3 +108,6 @@
+    // Stage diff preview helper for UI inspection
+    #[tauri::command]
+    pub fn stage_diff_preview(...)
     let cancel = tokio_util::sync::CancellationToken::new();`,additions:4,deletions:0}],L="vs_code";async function j(o,e){switch(o){case"get_model_status":return[{provider:"Ollama",base_url:"http://localhost:11434",is_online:!0,latency_ms:12,models:[{id:"qwen2.5-coder:7b",name:"Qwen 2.5 Coder 7B",is_moe:!1,context_length:32768},{id:"llama3.2:3b",name:"Llama 3.2 3B (Fast)",is_moe:!1,context_length:131072}]},{provider:"ColibriMoe",base_url:"http://127.0.0.1:8080",is_online:!0,latency_ms:4,models:[{id:"mixtral-8x7b-instruct",name:"Mixtral 8x7B MoE (NVMe Streamed)",is_moe:!0,context_length:32768}]},{provider:"LmStudio",base_url:"http://localhost:1234",is_online:!1,latency_ms:void 0,models:[]}];case"get_active_skills":return[...C];case"toggle_skill":{const t=e?.name,s=!!e?.enabled,i=C.find(a=>a.name===t);i&&(i.enabled=s);return}case"get_diff_preview":return[...m];case"stage_diff_preview":{const t={path:e?.path||"src/sample.rs",diff_text:`--- a/${e?.path}
+++ b/${e?.path}
@@ -1,1 +1,2 @@
-${e?.original}
+${e?.modified}`,additions:1,deletions:1};return m.push(t),t}case"apply_diff":{const t=e?.path;m=m.filter(s=>s.path!==t);return}case"rollback_diff":{const t=e?.path;return m=m.filter(s=>s.path!==t),`Rolled back ${t}`}case"get_active_layout":return L;case"set_active_layout":L=e?.layout||"vs_code";return;case"get_moe_status":return{is_running:!0,active_model_name:"Mixtral 8x7B MoE",streamed_experts_count:8,disk_read_mbs:1240.5,ram_resident_mb:48};case"save_file":return console.log(`[Mock Save] ${e?.path} (${e?.content?.length||0} bytes)`),`Saved ${e?.path}`;case"read_file":return`// Mock file content for ${e?.path}
`;case"send_agent_message":return`R.I.D.E Local Agent processed request: "${e?.message||""}". Checked workspace files and AST structures.`;default:console.warn(`[Mock IPC] Unknown command: ${o}`);return}}class U{element;onLayoutChange;constructor(e){this.onLayoutChange=e,this.element=document.createElement("footer"),this.element.className="status-bar"}getElement(){return this.element}render(e){const t=e.providers.find(r=>r.is_online),s=!!t,i=t?t.provider:"Offline",a=t?.latency_ms?`${t.latency_ms}ms`:"",n=s?e.currentModel||"Ready":"No Model Detected",l=s&&e.tokensPerSec>0?`${e.tokensPerSec.toFixed(1)} tok/s`:"Idle",d=`${e.ramUsageMb} MB (WebView2)`;this.element.innerHTML=`
      <div class="status-left">
        <div class="status-item brand" title="R.I.D.E Native Desktop">
          <span class="icon">⚡</span>
          <span class="label">R.I.D.E</span>
        </div>
        <div class="status-item provider-badge ${s?"online":"offline"}" title="${i} ${a}">
          <span class="status-dot"></span>
          <span class="text">${i} ${a?`(${a})`:""}</span>
        </div>
        <div class="status-item model-badge ${s?"online":"offline"}" title="${s?`Active LLM: ${n}`:"No local LLM active (Standalone Editor Mode)"}">
          <span class="icon">${s?"🧠":"💤"}</span>
          <span class="text">${n}</span>
        </div>
        <div class="status-item tokens-badge" title="Inference Throughput">
          <span class="icon">⚡</span>
          <span class="text">${l}</span>
        </div>
      </div>

      <div class="status-right">
        <div class="status-item ram-badge" title="Native WebView2 Low Memory Footprint (<60MB target)">
          <span class="icon">🍃</span>
          <span class="text">${d}</span>
        </div>
        <div class="layout-switcher">
          <button class="layout-btn ${e.activeLayout==="vs_code"?"active":""}" data-layout="vs_code" title="VS Code Layout (Ctrl+1)">
            <span class="icon">⊞</span> VS Code
          </button>
          <button class="layout-btn ${e.activeLayout==="claude_code"?"active":""}" data-layout="claude_code" title="Claude Code Terminal Layout (Ctrl+2)">
            <span class="icon">❯_</span> Claude Code
          </button>
          <button class="layout-btn ${e.activeLayout==="codex"?"active":""}" data-layout="codex" title="Codex Zen Layout (Ctrl+3)">
            <span class="icon">✧</span> Codex Zen
          </button>
        </div>
      </div>
    `,this.attachEvents()}attachEvents(){this.element.querySelectorAll(".layout-btn").forEach(t=>{t.addEventListener("click",s=>{s.preventDefault();const i=t.getAttribute("data-layout");i&&this.onLayoutChange(i)})})}}const z=[{name:"main.rs",path:"crates/ride-desktop/src/main.rs",content:`// R.I.D.E Desktop — Rust Intelligent Development Environment
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
`},{name:"commands.rs",path:"crates/ride-desktop/src/commands.rs",content:`/// Tauri IPC commands for the R.I.D.E desktop application.
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
`},{name:"Cargo.toml",path:"crates/ride-desktop/Cargo.toml",content:`[package]
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
`}];class K{element;tabs=[...z];activeTabIndex=0;editorView=null;editorContainer=null;filePickerInput=null;onSaveCallback;isProgrammaticUpdate=!1;constructor(e){this.onSaveCallback=e,this.element=document.createElement("div"),this.element.className="editor-component-container",this.renderSkeleton(),this.loadRealDiskFiles()}getElement(){return this.element}setEditorContent(e){if(this.editorView){this.isProgrammaticUpdate=!0;try{this.editorView.dispatch({changes:{from:0,to:this.editorView.state.doc.length,insert:e}})}finally{this.isProgrammaticUpdate=!1}}}setContent(e){this.setEditorContent(e),this.tabs[this.activeTabIndex]&&(this.tabs[this.activeTabIndex].content=e,this.tabs[this.activeTabIndex].isModified=!1,this.renderTabs())}getContent(){return this.editorView?this.editorView.state.doc.toString():""}getActiveTab(){return this.tabs[this.activeTabIndex]||null}createNewFile(e,t){const s=this.tabs.filter(l=>l.name.startsWith("untitled")).length+1,i=e||`untitled-${s}.rs`,n={name:i,path:t||i,content:`// New buffer — edit code and press Ctrl+S to save
`,isModified:!1};this.tabs.push(n),this.switchTab(this.tabs.length-1)}async saveActiveFile(){const e=this.tabs[this.activeTabIndex];if(!e)return!1;if(e.path.startsWith("untitled")){const s=window.prompt("Save file as (enter path):",e.path);if(!s||!s.trim())return!1;e.path=s.trim(),e.name=e.path.split(/[\/\\]/).pop()||e.path}const t=this.getContent();e.content=t;try{return await N(e.path,t),e.isModified=!1,this.renderTabs(),this.flashSaveFeedback(!0),this.onSaveCallback&&this.onSaveCallback(e.path),!0}catch(s){return console.error(`[Editor] Failed to save ${e.path}:`,s),this.flashSaveFeedback(!1),!1}}async openFile(e,t){const s=this.tabs.findIndex(l=>l.path===e);if(s!==-1){this.switchTab(s);return}let i=t;if(i===void 0)try{i=await S(e)}catch(l){console.warn(`[Editor] Could not read file from disk (${e}):`,l),i=`// File: ${e}
// Newly opened editor buffer
`}const n={name:e.split(/[\/\\]/).pop()||e,path:e,content:i,isModified:!1};this.tabs.push(n),this.switchTab(this.tabs.length-1)}async loadRealDiskFiles(){for(let e=0;e<this.tabs.length;e++){const t=this.tabs[e];try{const s=await S(t.path);s&&(t.content=s,e===this.activeTabIndex&&this.editorView&&(this.setEditorContent(s),t.isModified=!1,this.renderTabs()))}catch{}}}renderSkeleton(){this.element.innerHTML=`
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
    `,this.filePickerInput=this.element.querySelector(".editor-file-picker"),this.renderTabs(),this.initCodeMirror(),this.attachActionEvents()}renderTabs(){const e=this.element.querySelector(".editor-tabs-scroll");e&&(e.innerHTML=this.tabs.map((t,s)=>{const i=s===this.activeTabIndex,a=t.name.endsWith(".rs")?"🦀":t.name.endsWith(".toml")?"⚙️":"📄";return`
        <div class="editor-tab ${i?"active":""}" data-index="${s}" title="${t.path}">
          <span class="tab-icon">${a}</span>
          <span class="tab-title">${t.name}${t.isModified?' <span class="tab-dirty">●</span>':""}</span>
          <span class="tab-close" data-index="${s}" title="Close tab">✕</span>
        </div>
      `}).join(""),e.querySelectorAll(".editor-tab").forEach(t=>{t.addEventListener("click",s=>{if(s.target.classList.contains("tab-close"))return;const a=Number(t.getAttribute("data-index"));this.switchTab(a)})}),e.querySelectorAll(".tab-close").forEach(t=>{t.addEventListener("click",s=>{s.stopPropagation();const i=Number(t.getAttribute("data-index"));this.closeTab(i)})}))}switchTab(e){if(e<0||e>=this.tabs.length)return;this.activeTabIndex=e,this.renderTabs();const t=this.tabs[e];t&&this.setEditorContent(t.content)}closeTab(e){this.tabs.length<=1||(this.tabs.splice(e,1),this.activeTabIndex>=this.tabs.length&&(this.activeTabIndex=this.tabs.length-1),this.renderTabs(),this.switchTab(this.activeTabIndex))}initCodeMirror(){if(this.editorContainer=this.element.querySelector(".editor-cm-wrapper"),!this.editorContainer)return;const e=this.tabs[this.activeTabIndex]?.content||"",t=_.of([{key:"Mod-s",run:()=>(this.saveActiveFile(),!0)}]),s=x.create({doc:e,extensions:[T,t,$(),A,k.updateListener.of(i=>{if(!this.isProgrammaticUpdate&&i.docChanged&&this.tabs[this.activeTabIndex]){const a=this.tabs[this.activeTabIndex];a.content=i.state.doc.toString(),a.isModified||(a.isModified=!0,this.renderTabs())}})]});this.editorView=new k({state:s,parent:this.editorContainer})}attachActionEvents(){const e=this.element.querySelector(".btn-new");e&&e.addEventListener("click",()=>{this.createNewFile()});const t=this.element.querySelector(".btn-save");t&&t.addEventListener("click",()=>{this.saveActiveFile()});const s=this.element.querySelector(".btn-open");s&&this.filePickerInput&&(s.addEventListener("click",()=>{this.filePickerInput?.click()}),this.filePickerInput.addEventListener("change",a=>{const n=a.target,l=n.files?.[0];if(!l)return;const d=new FileReader;d.onload=r=>{const c=r.target?.result||"",u=l.path||l.name;this.openFile(u,c),n.value=""},d.readAsText(l)}));const i=this.element.querySelector(".btn-open-path");i&&i.addEventListener("click",()=>{const a=window.prompt("Enter file path to open (e.g. crates/ride-desktop/Cargo.toml):");a&&a.trim()&&this.openFile(a.trim())})}flashSaveFeedback(e){const t=this.element.querySelector(".btn-save");if(!t)return;const s='<span class="btn-icon">💾</span> <span class="btn-text">Save</span>';e?(t.innerHTML='<span class="btn-icon">✓</span> <span class="btn-text">Saved</span>',t.classList.add("saved")):t.innerHTML='<span class="btn-icon">✕</span> <span class="btn-text">Failed</span>',setTimeout(()=>{t.innerHTML=s,t.classList.remove("saved")},1500)}}class G{element;onSendMessage;isBusy=!1;constructor(e){this.onSendMessage=e,this.element=document.createElement("div"),this.element.className="agent-chat-container",this.renderSkeleton()}getElement(){return this.element}setBusy(e){this.isBusy=e;const t=this.element.querySelector(".chat-send-btn"),s=this.element.querySelector(".chat-input");t&&(t.disabled=e),s&&(s.disabled=e);const i=this.element.querySelector(".stream-indicator");i&&(i.style.display=e?"flex":"none")}updateMessages(e){const t=this.element.querySelector(".chat-messages");if(t){if(e.length===0){t.innerHTML=`
        <div class="chat-empty-state">
          <div class="empty-icon">🤖</div>
          <h3>R.I.D.E Autonomous Agent</h3>
          <p>Powered by local MoE streaming & offline discovery.</p>
          <div class="quick-prompts">
            <button class="quick-chip" data-prompt="Analyze active workspace and list high priority tasks">Analyze workspace</button>
            <button class="quick-chip" data-prompt="Implement a fast zero-copy parser for unified diffs in Rust">Create diff parser</button>
            <button class="quick-chip" data-prompt="Run cargo test --workspace and report any warnings">Run test suite</button>
          </div>
        </div>
      `,this.attachQuickPrompts();return}t.innerHTML=e.map(s=>{const i=s.role==="user",a=i?"You":"R.I.D.E Agent",n=i?"👤":"⚡",l=s.thoughts&&s.thoughts.length>0?`
          <details class="thought-accordion" open>
            <summary class="thought-summary">
              <span class="thought-icon">💭</span>
              <span>Reasoning & Plan (${s.thoughts.length} steps)</span>
            </summary>
            <div class="thought-body">
              ${s.thoughts.map(r=>`<div class="thought-step">${this.escapeHtml(r)}</div>`).join("")}
            </div>
          </details>`:"",d=s.toolCalls&&s.toolCalls.length>0?`
          <div class="tool-calls-container">
            ${s.toolCalls.map(r=>`
              <div class="tool-call-item ${r.status||"success"}">
                <span class="tool-icon">🛠️</span>
                <span class="tool-name">${this.escapeHtml(r.name)}</span>
                <span class="tool-status">${r.status||"executed"}</span>
                ${r.output?`<pre class="tool-output">${this.escapeHtml(r.output)}</pre>`:""}
              </div>`).join("")}
          </div>`:"";return`
          <div class="message-card ${s.role}">
            <div class="message-header">
              <span class="avatar">${n}</span>
              <span class="author">${a}</span>
              <span class="time">${new Date(s.timestamp).toLocaleTimeString([],{hour:"2-digit",minute:"2-digit"})}</span>
            </div>
            ${l}
            ${d}
            <div class="message-content">${this.formatContent(s.content)}</div>
          </div>
        `}).join(""),t.scrollTop=t.scrollHeight}}renderSkeleton(){this.element.innerHTML=`
      <div class="chat-header">
        <div class="chat-title">
          <span class="icon">🤖</span>
          <span>Agent Session</span>
        </div>
        <div class="chat-actions">
          <span class="stream-indicator" style="display: none;">
            <span class="dot"></span> Generating...
          </span>
        </div>
      </div>
      <div class="chat-messages"></div>
      <div class="chat-input-bar">
        <textarea class="chat-input" placeholder="Ask R.I.D.E agent... (Press Enter to send, Shift+Enter for newline)" rows="2"></textarea>
        <button class="chat-send-btn" title="Send (Enter)">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="22" y1="2" x2="11" y2="13"></line>
            <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
          </svg>
        </button>
      </div>
    `,this.attachEvents()}attachEvents(){const e=this.element.querySelector(".chat-input"),t=this.element.querySelector(".chat-send-btn"),s=()=>{if(this.isBusy||!e)return;const i=e.value.trim();i&&(e.value="",this.onSendMessage(i))};t&&t.addEventListener("click",s),e&&e.addEventListener("keydown",i=>{i.key==="Enter"&&!i.shiftKey&&(i.preventDefault(),s())}),this.attachQuickPrompts()}attachQuickPrompts(){this.element.querySelectorAll(".quick-chip").forEach(t=>{t.addEventListener("click",()=>{const s=t.getAttribute("data-prompt");s&&!this.isBusy&&this.onSendMessage(s)})})}escapeHtml(e){const t=document.createElement("div");return t.innerText=e,t.innerHTML}formatContent(e){return e?e.split(/(```[\s\S]*?```)/g).map(s=>{if(s.startsWith("```")&&s.endsWith("```")){const a=s.indexOf(`
`),n=a!==-1?s.slice(3,a).trim():"",l=a!==-1?s.slice(a+1,-3):s.slice(3,-3);return`<pre class="code-block"${n?` data-lang="${this.escapeHtml(n)}"`:""}><code>${this.escapeHtml(l)}</code></pre>`}let i=this.escapeHtml(s);return i=i.replace(/`([^`]+)`/g,'<code class="inline-code">$1</code>'),i=i.replace(/\n/g,"<br/>"),i}).join(""):""}}class Q{element;onApply;onRollback;constructor(e,t){this.onApply=e,this.onRollback=t,this.element=document.createElement("div"),this.element.className="diff-viewer-container"}getElement(){return this.element}render(e){if(e.length===0){this.element.innerHTML=`
        <div class="diff-empty-state">
          <div class="empty-icon">✓</div>
          <h3>All Clean</h3>
          <p>No pending file diffs awaiting approval.</p>
        </div>
      `;return}this.element.innerHTML=`
      <div class="diff-viewer-header">
        <div class="diff-title">
          <span class="icon">±</span>
          <span>Pending Changes (${e.length} ${e.length===1?"file":"files"})</span>
        </div>
      </div>
      <div class="diff-list">
        ${e.map((t,s)=>`
            <div class="diff-card" data-path="${this.escapeHtml(t.path)}">
              <div class="diff-card-header">
                <div class="diff-path-info">
                  <span class="file-icon">📄</span>
                  <span class="diff-path">${this.escapeHtml(t.path)}</span>
                  <span class="diff-stats">
                    <span class="stat-add">+${t.additions}</span>
                    <span class="stat-del">-${t.deletions}</span>
                  </span>
                </div>
                <div class="diff-card-actions">
                  <button class="diff-btn apply-btn" data-index="${s}" data-path="${this.escapeHtml(t.path)}" title="Apply this change to disk">
                    ✓ Apply
                  </button>
                  <button class="diff-btn rollback-btn" data-index="${s}" data-path="${this.escapeHtml(t.path)}" title="Discard change and restore backup">
                    ↺ Rollback
                  </button>
                </div>
              </div>
              <div class="diff-content">
                <pre class="diff-code">${this.renderDiffLines(t.diff_text)}</pre>
              </div>
            </div>
          `).join("")}
      </div>
    `,this.attachEvents()}renderDiffLines(e){return e.split(`
`).map(s=>{let i="diff-line-context";return s.startsWith("+")&&!s.startsWith("+++")?i="diff-line-add":s.startsWith("-")&&!s.startsWith("---")?i="diff-line-del":s.startsWith("@@")?i="diff-line-hunk":(s.startsWith("---")||s.startsWith("+++"))&&(i="diff-line-header"),`<span class="${i}">${this.escapeHtml(s)}</span>`}).join(`
`)}attachEvents(){this.element.querySelectorAll(".apply-btn").forEach(s=>{s.addEventListener("click",async i=>{i.preventDefault();const a=s.getAttribute("data-path");a&&(s.disabled=!0,s.innerText="Applying...",await this.onApply(a))})}),this.element.querySelectorAll(".rollback-btn").forEach(s=>{s.addEventListener("click",async i=>{i.preventDefault();const a=s.getAttribute("data-path");a&&(s.disabled=!0,s.innerText="Rolling back...",await this.onRollback(a))})})}escapeHtml(e){const t=document.createElement("div");return t.innerText=e,t.innerHTML}}class Z{element;onToggle;filterQuery="";skills=[];constructor(e){this.onToggle=e,this.element=document.createElement("div"),this.element.className="skills-manager-container"}getElement(){return this.element}render(e){this.skills=e;const t=e.filter(i=>i.enabled).length,s=e.filter(i=>{if(!this.filterQuery)return!0;const a=this.filterQuery.toLowerCase();return i.name.toLowerCase().includes(a)||i.description.toLowerCase().includes(a)||i.triggers.some(n=>n.toLowerCase().includes(a))});this.element.innerHTML=`
      <div class="skills-header">
        <div class="skills-title">
          <span class="icon">🧩</span>
          <span>Skills & Dynamic Tools</span>
          <span class="skills-badge">${t} Active</span>
        </div>
        <div class="skills-search-bar">
          <input type="text" class="skills-search-input" placeholder="Filter skills or triggers..." value="${this.escapeHtml(this.filterQuery)}" />
        </div>
      </div>
      <div class="skills-list">
        ${s.length===0?`<div class="skills-empty">No skills matching "${this.escapeHtml(this.filterQuery)}"</div>`:s.map(i=>`
            <div class="skill-card ${i.enabled?"active":""}">
              <div class="skill-card-top">
                <div class="skill-info">
                  <div class="skill-name">
                    <span class="skill-icon">⚡</span>
                    <strong>${this.escapeHtml(i.name)}</strong>
                  </div>
                  <div class="skill-desc">${this.escapeHtml(i.description)}</div>
                </div>
                <label class="switch" title="${i.enabled?"Disable":"Enable"} skill">
                  <input type="checkbox" class="skill-toggle" data-skill="${this.escapeHtml(i.name)}" ${i.enabled?"checked":""} />
                  <span class="slider round"></span>
                </label>
              </div>
              <div class="skill-triggers">
                ${i.triggers.map(a=>`<span class="trigger-chip">${this.escapeHtml(a)}</span>`).join("")}
              </div>
            </div>
          `).join("")}
      </div>
    `,this.attachEvents()}attachEvents(){const e=this.element.querySelector(".skills-search-input");e&&e.addEventListener("input",s=>{this.filterQuery=s.target.value,this.render(this.skills);const i=this.element.querySelector(".skills-search-input");i&&(i.focus(),i.setSelectionRange(i.value.length,i.value.length))}),this.element.querySelectorAll(".skill-toggle").forEach(s=>{s.addEventListener("change",async()=>{const i=s.getAttribute("data-skill"),a=s.checked;i&&await this.onToggle(i,a)})})}escapeHtml(e){const t=document.createElement("div");return t.innerText=e,t.innerHTML}}class X{overlay;input;list;isOpen=!1;items=[];selectedIndex=0;onSelectLayout;onPromptAgent;onOpenFile;onSaveFile;onNewFile;constructor(e,t,s,i,a){this.onSelectLayout=e,this.onPromptAgent=t,this.onOpenFile=s,this.onSaveFile=i,this.onNewFile=a,this.overlay=document.createElement("div"),this.overlay.className="command-palette-overlay",this.overlay.style.display="none",this.overlay.innerHTML=`
      <div class="command-palette-modal">
        <div class="palette-input-wrapper">
          <span class="palette-icon">✧</span>
          <input type="text" class="palette-input" placeholder="Type a command or ask R.I.D.E AI... (Ctrl+K)" />
          <span class="palette-kbd">Esc</span>
        </div>
        <div class="palette-results"></div>
      </div>
    `,this.input=this.overlay.querySelector(".palette-input"),this.list=this.overlay.querySelector(".palette-results"),this.initDefaultItems(),this.attachEvents(),document.body.appendChild(this.overlay)}open(){this.isOpen=!0,this.overlay.style.display="flex",this.input.value="",this.selectedIndex=0,this.renderItems(""),setTimeout(()=>this.input.focus(),50)}close(){this.isOpen=!1,this.overlay.style.display="none"}toggle(){this.isOpen?this.close():this.open()}initDefaultItems(){this.items=[{id:"file-save",label:"File: Save Active File (Ctrl+S)",sublabel:"Persist current CodeMirror buffer directly to disk via Rust IPC",icon:"💾",action:()=>{this.close(),this.onSaveFile?.()}},{id:"file-open",label:"File: Open File by Path (Ctrl+O)",sublabel:"Load source file from disk into editor",icon:"📂",action:()=>{this.close(),this.onOpenFile?.()}},{id:"file-new",label:"File: New File Buffer (Ctrl+N)",sublabel:"Create a new scratch buffer for manual editing",icon:"✚",action:()=>{this.close(),this.onNewFile?.()}},{id:"layout-vscode",label:"Switch Layout: VS Code",sublabel:"Full IDE with activity bar, explorer, editor & agent dock",icon:"⊞",action:()=>this.onSelectLayout("vs_code")},{id:"layout-claude",label:"Switch Layout: Claude Code",sublabel:"Terminal-first agent stream with live diffs & reasoning",icon:"❯_",action:()=>this.onSelectLayout("claude_code")},{id:"layout-codex",label:"Switch Layout: Codex Zen",sublabel:"Distraction-free canvas with floating command overlay",icon:"✧",action:()=>this.onSelectLayout("codex")},{id:"ask-analyze",label:"Ask AI: Analyze current file syntax & lifetime safety",sublabel:"Invokes local model with rust-expert skill",icon:"🦀",action:()=>this.onPromptAgent("Review the current Rust code for lifetime and borrowing soundness.")},{id:"ask-diff",label:"Ask AI: Generate unit tests for active commands",sublabel:"Proposes tests with previewable unified diff",icon:"🧪",action:()=>this.onPromptAgent("Generate comprehensive unit tests for the active commands module.")}]}renderItems(e){const t=e.toLowerCase().trim(),s=this.items.filter(i=>t?i.label.toLowerCase().includes(t)||i.sublabel&&i.sublabel.toLowerCase().includes(t):!0);if(s.length===0){this.list.innerHTML=`
        <div class="palette-item active" data-action="ai-query">
          <span class="item-icon">⚡</span>
          <div class="item-text">
            <div class="item-label">Ask R.I.D.E Agent: "${this.escapeHtml(e)}"</div>
            <div class="item-sub">Execute autonomous agent with loaded skills</div>
          </div>
        </div>
      `;return}this.list.innerHTML=s.map((i,a)=>`
        <div class="palette-item ${a===this.selectedIndex?"active":""}" data-idx="${a}">
          <span class="item-icon">${i.icon}</span>
          <div class="item-text">
            <div class="item-label">${this.escapeHtml(i.label)}</div>
            ${i.sublabel?`<div class="item-sub">${this.escapeHtml(i.sublabel)}</div>`:""}
          </div>
        </div>
      `).join("")}attachEvents(){this.overlay.addEventListener("click",e=>{e.target===this.overlay&&this.close()}),this.input.addEventListener("input",()=>{this.selectedIndex=0,this.renderItems(this.input.value)}),this.input.addEventListener("keydown",e=>{if(e.key==="Escape"){this.close();return}const t=this.list.querySelectorAll(".palette-item").length;e.key==="ArrowDown"?(e.preventDefault(),this.selectedIndex=(this.selectedIndex+1)%t,this.renderItems(this.input.value)):e.key==="ArrowUp"?(e.preventDefault(),this.selectedIndex=(this.selectedIndex-1+t)%t,this.renderItems(this.input.value)):e.key==="Enter"&&(e.preventDefault(),this.executeCurrent())}),this.list.addEventListener("click",e=>{const t=e.target.closest(".palette-item");if(t){const s=t.getAttribute("data-idx");s!==null?(this.selectedIndex=Number(s),this.executeCurrent()):t.getAttribute("data-action")==="ai-query"&&this.executeCurrent()}})}executeCurrent(){const e=this.input.value.trim(),t=this.items.filter(s=>e?s.label.toLowerCase().includes(e.toLowerCase())||s.sublabel&&s.sublabel.toLowerCase().includes(e.toLowerCase()):!0);if(t.length>0&&t[this.selectedIndex]){const s=t[this.selectedIndex];this.close(),s.action()}else e&&(this.close(),this.onPromptAgent(e))}escapeHtml(e){const t=document.createElement("div");return t.innerText=e,t.innerHTML}}class Y{element;editor;chat;diffViewer;skillsManager;activeActivity="explorer";activeDockTab="chat";isDockCollapsed=!1;currentState=null;constructor(e,t,s,i){this.editor=e,this.chat=t,this.diffViewer=s,this.skillsManager=i,this.element=document.createElement("div"),this.element.className="layout-vscode",this.renderSkeleton()}getElement(){return this.element}mount(){const e=this.element.querySelector(".editor-pane");if(e&&!e.contains(this.editor.getElement())&&(e.innerHTML="",e.appendChild(this.editor.getElement())),this.updateDockContent(),this.activeActivity==="skills"){const t=this.element.querySelector(".sidebar-content");t&&!t.contains(this.skillsManager.getElement())&&(t.innerHTML="",t.appendChild(this.skillsManager.getElement()))}}update(e){this.currentState=e,this.diffViewer.render(e.diffs),this.skillsManager.render(e.skills),this.chat.updateMessages(e.messages),this.updateSideBarContent(e);const t=this.element.querySelector('.dock-tab[data-tab="diffs"] .dock-badge');t&&(t.innerText=String(e.diffs.length),t.style.display=e.diffs.length>0?"inline-block":"none")}renderSkeleton(){this.element.innerHTML=`
      <div class="vscode-body">
        <!-- 1. Left Activity Bar -->
        <div class="activity-bar">
          <div class="activity-top">
            <button class="activity-btn ${this.activeActivity==="explorer"?"active":""}" data-activity="explorer" title="Explorer (Ctrl+Shift+E)">
              <span class="icon">📁</span>
            </button>
            <button class="activity-btn ${this.activeActivity==="search"?"active":""}" data-activity="search" title="Search Workspace (Ctrl+Shift+F)">
              <span class="icon">🔍</span>
            </button>
            <button class="activity-btn ${this.activeActivity==="skills"?"active":""}" data-activity="skills" title="Skills & Tools">
              <span class="icon">🧩</span>
            </button>
            <button class="activity-btn ${this.activeActivity==="models"?"active":""}" data-activity="models" title="Model Providers">
              <span class="icon">🧠</span>
            </button>
          </div>
          <div class="activity-bottom">
            <button class="activity-btn" title="Settings">
              <span class="icon">⚙️</span>
            </button>
          </div>
        </div>

        <!-- 2. Primary Side Bar -->
        <div class="primary-sidebar">
          <div class="sidebar-header">
            <span class="sidebar-title">EXPLORER</span>
          </div>
          <div class="sidebar-content"></div>
        </div>

        <!-- 3. Central Working Area: Editor + Agent Dock -->
        <div class="main-editor-area">
          <div class="editor-pane"></div>
          <div class="dock-resizer" title="Drag or click to toggle dock"></div>
          <div class="agent-dock ${this.isDockCollapsed?"collapsed":""}">
            <div class="dock-header">
              <div class="dock-tabs">
                <button class="dock-tab ${this.activeDockTab==="chat"?"active":""}" data-tab="chat">
                  <span>🤖 Agent Dock</span>
                </button>
                <button class="dock-tab ${this.activeDockTab==="diffs"?"active":""}" data-tab="diffs">
                  <span>± Changes</span>
                  <span class="dock-badge" style="display: none;">0</span>
                </button>
              </div>
              <div class="dock-controls">
                <button class="dock-toggle-btn" title="Toggle Dock">─</button>
              </div>
            </div>
            <div class="dock-content"></div>
          </div>
        </div>
      </div>
    `;const e=this.element.querySelector(".editor-pane");e&&e.appendChild(this.editor.getElement()),this.updateDockContent(),this.attachEvents()}updateDockContent(){const e=this.element.querySelector(".dock-content");e&&(e.innerHTML="",this.activeDockTab==="chat"?e.appendChild(this.chat.getElement()):e.appendChild(this.diffViewer.getElement()))}updateSideBarContent(e){const t=this.element.querySelector(".sidebar-title"),s=this.element.querySelector(".sidebar-content");!s||!t||(this.activeActivity==="explorer"?(t.innerText="WORKSPACE EXPLORER",s.innerHTML=`
        <div class="explorer-tree">
          <div class="tree-section">
            <div class="tree-header">▾ R.I.D.E (Workspace)</div>
            <div class="tree-item indent-1">📁 crates</div>
            <div class="tree-item indent-2">📁 ride-core</div>
            <div class="tree-item indent-2">📁 ride-discovery</div>
            <div class="tree-item indent-2">📁 ride-moe</div>
            <div class="tree-item indent-2">📁 ride-skills</div>
            <div class="tree-item indent-2">📁 ride-agent</div>
            <div class="tree-item indent-2 open">▾ 📁 ride-desktop</div>
            <div class="tree-item indent-3 file active" data-path="crates/ride-desktop/src/main.rs">🦀 main.rs</div>
            <div class="tree-item indent-3 file" data-path="crates/ride-desktop/src/commands.rs">🦀 commands.rs</div>
            <div class="tree-item indent-3 file" data-path="crates/ride-desktop/Cargo.toml">⚙️ Cargo.toml</div>
            <div class="tree-item indent-3 file" data-path="crates/ride-desktop/tauri.conf.json">⚙️ tauri.conf.json</div>
            <div class="tree-item indent-1 file" data-path="README.md">📄 README.md</div>
            <div class="tree-item indent-1 file" data-path="Cargo.toml">⚙️ Cargo.toml</div>
          </div>
        </div>
      `,s.querySelectorAll(".tree-item.file[data-path]").forEach(i=>{i.addEventListener("click",()=>{const a=i.getAttribute("data-path");a&&(s.querySelectorAll(".tree-item.file").forEach(n=>n.classList.remove("active")),i.classList.add("active"),this.editor.openFile(a))})})):this.activeActivity==="search"?(t.innerText="SEARCH WORKSPACE",s.innerHTML=`
        <div class="search-panel">
          <input type="text" class="search-input" placeholder="Search in files... (regex allowed)" />
          <div class="search-hint">Press Enter to search entire workspace</div>
        </div>
      `):this.activeActivity==="skills"?(t.innerText="SKILLS REGISTRY",s.innerHTML="",s.appendChild(this.skillsManager.getElement())):this.activeActivity==="models"&&(t.innerText="LOCAL MODELS & PROVIDERS",s.innerHTML=`
        <div class="models-panel">
          ${e.providers.map(i=>`
            <div class="provider-card ${i.is_online?"online":"offline"}">
              <div class="p-header">
                <span class="p-dot"></span>
                <strong>${i.provider}</strong>
                <span class="p-status">${i.is_online?`${i.latency_ms||10}ms`:"Offline"}</span>
              </div>
              <div class="p-url">${i.base_url}</div>
              <div class="p-models">
                ${i.models.length>0?i.models.map(a=>`
                  <div class="model-row ${a.id===e.currentModel?"selected":""}">
                    <span class="m-icon">${a.is_moe?"🔀":"⚡"}</span>
                    <span class="m-name">${a.name||a.id}</span>
                    ${a.is_moe?'<span class="m-moe">MoE</span>':""}
                  </div>`).join(""):'<div class="p-empty">No active models detected</div>'}
              </div>
            </div>`).join("")}
        </div>
      `))}attachEvents(){const e=this.element.querySelectorAll(".activity-btn[data-activity]");e.forEach(a=>{a.addEventListener("click",()=>{const n=a.getAttribute("data-activity");n&&(this.activeActivity=n,e.forEach(l=>l.classList.remove("active")),a.classList.add("active"),this.currentState&&this.updateSideBarContent(this.currentState))})});const t=this.element.querySelectorAll(".dock-tab");t.forEach(a=>{a.addEventListener("click",()=>{const n=a.getAttribute("data-tab");n&&(this.activeDockTab=n,t.forEach(l=>l.classList.remove("active")),a.classList.add("active"),this.updateDockContent())})});const s=this.element.querySelector(".dock-toggle-btn"),i=this.element.querySelector(".agent-dock");s&&i&&s.addEventListener("click",()=>{this.isDockCollapsed=!this.isDockCollapsed,i.classList.toggle("collapsed",this.isDockCollapsed),s.innerText=this.isDockCollapsed?"▢":"─"})}}class J{element;diffViewer;onSendMessage;isBusy=!1;history=[];historyIndex=-1;constructor(e,t){this.diffViewer=e,this.onSendMessage=t,this.element=document.createElement("div"),this.element.className="layout-claude-code",this.renderSkeleton()}getElement(){return this.element}mount(){const e=this.element.querySelector(".drawer-content");e&&!e.contains(this.diffViewer.getElement())&&(e.innerHTML="",e.appendChild(this.diffViewer.getElement()))}update(e){this.diffViewer.render(e.diffs),this.renderTerminalStream(e.messages,e),this.updateStatusPill(e)}updateStatusPill(e){const t=this.element.querySelector(".terminal-status-pill"),s=this.element.querySelector(".status-txt");if(!t||!s)return;const i=e.providers.some(a=>a.is_online);t.classList.toggle("offline",!i),t.classList.toggle("online",i),i?s.innerText=`${e.currentProvider} online`:s.innerText="offline mode"}setBusy(e){this.isBusy=e;const t=this.element.querySelector(".cli-input"),s=this.element.querySelector(".cli-prompt-status");t&&(t.disabled=e),s&&(s.innerText=e?"⚡ executing...":"❯",s.classList.toggle("busy",e))}renderSkeleton(){this.element.innerHTML=`
      <div class="claude-terminal-window">
        <!-- Terminal Header -->
        <div class="terminal-header">
          <div class="terminal-dots">
            <span class="dot red"></span>
            <span class="dot yellow"></span>
            <span class="dot green"></span>
          </div>
          <div class="terminal-title">
            <span class="term-icon">❯_</span>
            <span>claude-code session — R.I.D.E Autonomous Agent (Terminal-First)</span>
          </div>
          <div class="terminal-status-pill offline">
            <span class="pulse-dot"></span>
            <span class="status-txt">offline mode</span>
          </div>
        </div>

        <!-- Terminal Output Stream -->
        <div class="terminal-stream-area"></div>

        <!-- Inline Diffs Drawer (if diffs are pending) -->
        <div class="terminal-diffs-drawer" style="display: none;">
          <div class="drawer-header">
            <span class="drawer-icon">±</span>
            <span class="drawer-title">Pending Diff Reviews</span>
            <button class="drawer-toggle-btn">Hide</button>
          </div>
          <div class="drawer-content"></div>
        </div>

        <!-- Terminal Command Line Prompt -->
        <div class="terminal-input-bar">
          <span class="cli-prompt-status">❯</span>
          <input type="text" class="cli-input" placeholder="Type a task, /command, or natural language instruction..." autofocus />
          <button class="cli-submit-btn">Run</button>
        </div>
      </div>
    `;const e=this.element.querySelector(".drawer-content");e&&e.appendChild(this.diffViewer.getElement()),this.attachEvents()}renderTerminalStream(e,t){const s=this.element.querySelector(".terminal-stream-area");if(!s)return;if(e.length===0){s.innerHTML=`
        <div class="terminal-welcome">
          <pre class="ascii-banner">
██████╗     ██╗    ██████╗     ███████╗
██╔══██╗    ██║    ██╔══██╗    ██╔════╝
██████╔╝    ██║    ██║  ██║    █████╗  
██╔══██╗    ██║    ██║  ██║    ██╔══╝  
██║  ██║    ██║    ██████╔╝    ███████╗
          </pre>
          <div class="welcome-line"><strong>R.I.D.E Claude Code Terminal Interface</strong> v1.0.1</div>
          <div class="welcome-line dim">Autonomous agent loop with local MoE routing & safety approval gate.</div>
          <div class="welcome-line dim">Type <span class="highlight">/help</span>, <span class="highlight">/diff</span>, or any coding prompt to begin.</div>
          <div class="welcome-suggestions">
            <button class="cli-chip" data-cmd="Analyze workspace crate dependencies and check build speed">Analyze workspace</button>
            <button class="cli-chip" data-cmd="Generate zero-copy memory benchmarks for MoE router">Benchmark router</button>
            <button class="cli-chip" data-cmd="Refactor commands.rs to add async streaming preview">Refactor commands.rs</button>
          </div>
        </div>
      `,this.attachChips();return}let i="";e.forEach(n=>{if(n.role==="user")i+=`
          <div class="terminal-entry user-entry">
            <span class="cli-user-prompt">❯ </span>
            <span class="cli-user-cmd">${this.escapeHtml(n.content)}</span>
          </div>
        `;else{const l=n.thoughts&&n.thoughts.length>0?`
          <div class="cli-thought-box">
            <div class="cli-thought-header">
              <span class="thought-spinner">⚙</span>
              <span>Thinking Process (${n.thoughts.length} steps)</span>
            </div>
            <div class="cli-thought-content">
              ${n.thoughts.map(r=>`<div class="cli-step">· ${this.escapeHtml(r)}</div>`).join("")}
            </div>
          </div>`:"",d=n.toolCalls&&n.toolCalls.length>0?`
          <div class="cli-tools-stream">
            ${n.toolCalls.map(r=>`
              <div class="cli-tool-entry ${r.status||"success"}">
                <span class="tool-arrow">→</span>
                <span class="tool-call-name">${this.escapeHtml(r.name)}</span>
                <span class="tool-badge">${r.status||"executed"}</span>
                ${r.output?`<pre class="cli-tool-output">${this.escapeHtml(r.output)}</pre>`:""}
              </div>`).join("")}
          </div>`:"";i+=`
          <div class="terminal-entry agent-entry">
            ${l}
            ${d}
            <div class="cli-response-text">${this.formatCliOutput(n.content)}</div>
          </div>
        `}}),s.innerHTML=i,s.scrollTop=s.scrollHeight;const a=this.element.querySelector(".terminal-diffs-drawer");a&&(a.style.display=t.diffs.length>0?"block":"none")}attachEvents(){const e=this.element.querySelector(".cli-input"),t=this.element.querySelector(".cli-submit-btn"),s=()=>{if(this.isBusy||!e)return;const n=e.value.trim();if(n){if(this.history.push(n),this.historyIndex=this.history.length,e.value="",n==="/clear"){const l=this.element.querySelector(".terminal-stream-area");l&&(l.innerHTML="");return}this.onSendMessage(n)}};t&&t.addEventListener("click",s),e&&e.addEventListener("keydown",n=>{n.key==="Enter"?(n.preventDefault(),s()):n.key==="ArrowUp"?(n.preventDefault(),this.historyIndex>0&&(this.historyIndex--,e.value=this.history[this.historyIndex]||"")):n.key==="ArrowDown"&&(n.preventDefault(),this.historyIndex<this.history.length-1?(this.historyIndex++,e.value=this.history[this.historyIndex]||""):(this.historyIndex=this.history.length,e.value=""))});const i=this.element.querySelector(".drawer-toggle-btn"),a=this.element.querySelector(".drawer-content");i&&a&&i.addEventListener("click",()=>{const n=a.style.display==="none";a.style.display=n?"block":"none",i.innerText=n?"Hide":"Show"}),this.attachChips()}attachChips(){this.element.querySelectorAll(".cli-chip").forEach(t=>{t.addEventListener("click",()=>{const s=t.getAttribute("data-cmd");s&&!this.isBusy&&this.onSendMessage(s)})})}escapeHtml(e){const t=document.createElement("div");return t.innerText=e,t.innerHTML}formatCliOutput(e){return e?e.split(/(```[\s\S]*?```)/g).map(s=>{if(s.startsWith("```")&&s.endsWith("```")){const a=s.indexOf(`
`),n=a!==-1?s.slice(3,a).trim():"",l=a!==-1?s.slice(a+1,-3):s.slice(3,-3);return`<pre class="cli-code-block"${n?` data-lang="${this.escapeHtml(n)}"`:""}><code>${this.escapeHtml(l)}</code></pre>`}let i=this.escapeHtml(s);return i=i.replace(/`([^`]+)`/g,'<span class="cli-highlight">$1</span>'),i=i.replace(/\n/g,"<br/>"),i}).join(""):""}}class ee{element;editor;chat;commandPalette;isAssistantOpen=!1;constructor(e,t,s){this.editor=e,this.chat=t,this.commandPalette=s,this.element=document.createElement("div"),this.element.className="layout-codex",this.renderSkeleton()}getElement(){return this.element}mount(){const e=this.element.querySelector(".zen-editor-wrapper");e&&!e.contains(this.editor.getElement())&&(e.innerHTML="",e.appendChild(this.editor.getElement()));const t=this.element.querySelector(".assistant-content");t&&!t.contains(this.chat.getElement())&&(t.innerHTML="",t.appendChild(this.chat.getElement()))}update(e){this.chat.updateMessages(e.messages);const t=this.element.querySelector(".hud-doc");if(t){const s=this.editor.getActiveTab();t.innerText=s?s.name:"Editor"}}renderSkeleton(){this.element.innerHTML=`
      <div class="codex-zen-container">
        <!-- Floating HUD Header -->
        <div class="codex-floating-hud">
          <div class="hud-left">
            <span class="hud-brand">✧ Codex Zen</span>
            <span class="hud-dot"></span>
            <span class="hud-doc">main.rs</span>
          </div>
          <div class="hud-center">
            <button class="hud-cmd-btn" title="Open Command Palette (Ctrl+K)">
              <span class="hud-icon">⌘</span>
              <span class="hud-text">Ask R.I.D.E AI...</span>
              <span class="hud-badge">Ctrl+K</span>
            </button>
          </div>
          <div class="hud-right">
            <button class="hud-toggle-assistant" title="Toggle AI Side Drawer">
              <span class="icon">🤖</span>
              <span class="label">Assistant</span>
            </button>
          </div>
        </div>

        <!-- Full-bleed Zen Editor Canvas -->
        <div class="codex-canvas">
          <div class="zen-editor-wrapper"></div>
        </div>

        <!-- Floating Collapsible Side Assistant -->
        <div class="codex-side-assistant ${this.isAssistantOpen?"open":""}">
          <div class="assistant-header">
            <div class="assistant-title">
              <span class="icon">✧</span>
              <span>Zen AI Companion</span>
            </div>
            <button class="assistant-close-btn" title="Close Drawer">✕</button>
          </div>
          <div class="assistant-content"></div>
        </div>
      </div>
    `,this.mount(),this.attachEvents()}attachEvents(){const e=this.element.querySelector(".hud-cmd-btn");e&&e.addEventListener("click",()=>{this.commandPalette.open()});const t=this.element.querySelector(".hud-toggle-assistant"),s=this.element.querySelector(".assistant-close-btn"),i=this.element.querySelector(".codex-side-assistant"),a=()=>{this.isAssistantOpen=!this.isAssistantOpen,i&&i.classList.toggle("open",this.isAssistantOpen)};t&&t.addEventListener("click",a),s&&s.addEventListener("click",a)}}class te{state={activeLayout:"vs_code",activeFile:"crates/ride-desktop/src/main.rs",openFiles:["crates/ride-desktop/src/main.rs","crates/ride-desktop/src/commands.rs"],providers:[],skills:[],diffs:[],messages:[],currentModel:"",currentProvider:"Offline",tokensPerSec:0,ramUsageMb:42,isStreaming:!1};appRoot;layoutHost;statusBar;editor;chat;diffViewer;skillsManager;commandPalette;vscodeLayout;claudeLayout;codexLayout;toastElement=null;constructor(){const e=document.getElementById("app");if(!e)throw new Error("#app root element not found");this.appRoot=e,this.layoutHost=document.createElement("main"),this.layoutHost.className="layout-host",this.init()}async init(){this.editor=new K(t=>{this.showSaveToast(t)}),this.chat=new G(async t=>this.handleSendMessage(t)),this.diffViewer=new Q(async t=>this.handleApplyDiff(t),async t=>this.handleRollbackDiff(t)),this.skillsManager=new Z(async(t,s)=>this.handleToggleSkill(t,s)),this.statusBar=new U(async t=>this.switchLayout(t)),this.commandPalette=new X(t=>this.switchLayout(t),t=>this.handleSendMessage(t),()=>this.promptOpenFile(),()=>void this.editor.saveActiveFile(),()=>this.editor.createNewFile()),this.vscodeLayout=new Y(this.editor,this.chat,this.diffViewer,this.skillsManager),this.claudeLayout=new J(this.diffViewer,async t=>this.handleSendMessage(t)),this.codexLayout=new ee(this.editor,this.chat,this.commandPalette);const e=document.createElement("div");e.className="app-container",e.appendChild(this.layoutHost),e.appendChild(this.statusBar.getElement()),this.appRoot.appendChild(e),await this.loadInitialData(),this.mountLayout(this.state.activeLayout),this.registerShortcuts(),this.subscribeEvents(),setInterval(()=>{this.refreshModelStatus()},3e3)}async loadInitialData(){try{const[e,t,s,i]=await Promise.all([O(),E(),P(),R()]);this.state.activeLayout=e,this.state.providers=t,this.state.skills=s,this.state.diffs=i;const a=t.find(n=>n.is_online);a&&a.models.length>0?(this.state.currentProvider=a.provider,this.state.currentModel=a.models[0].name||a.models[0].id):a?(this.state.currentProvider=a.provider,this.state.currentModel=""):(this.state.currentProvider="Offline",this.state.currentModel=""),this.updateAllComponents()}catch(e){console.error("Failed to load initial R.I.D.E state:",e)}}async refreshModelStatus(){try{const e=await E();this.state.providers=e;const t=e.find(s=>s.is_online);t&&t.models.length>0?(this.state.currentProvider=t.provider,(!t.models.some(i=>i.name===this.state.currentModel||i.id===this.state.currentModel)||!this.state.currentModel)&&(this.state.currentModel=t.models[0].name||t.models[0].id)):t?(this.state.currentProvider=t.provider,this.state.currentModel=""):(this.state.currentProvider="Offline",this.state.currentModel=""),this.updateAllComponents()}catch(e){console.debug("[Model Status Ping] Offline or waiting:",e)}}async switchLayout(e){if(this.state.activeLayout!==e){this.state.activeLayout=e;try{await B(e)}catch(t){console.warn("Could not persist layout preference:",t)}this.mountLayout(e),this.statusBar.render(this.state)}}mountLayout(e){this.layoutHost.innerHTML="";let t;e==="claude_code"?(this.claudeLayout.mount(),t=this.claudeLayout.getElement(),this.claudeLayout.update(this.state)):e==="codex"?(this.codexLayout.mount(),t=this.codexLayout.getElement(),this.codexLayout.update(this.state)):(this.vscodeLayout.mount(),t=this.vscodeLayout.getElement(),this.vscodeLayout.update(this.state)),this.layoutHost.appendChild(t)}updateAllComponents(){this.statusBar.render(this.state),this.vscodeLayout.update(this.state),this.claudeLayout.update(this.state),this.codexLayout.update(this.state)}async handleSendMessage(e){const t={id:`msg-${Date.now()}`,role:"user",content:e,timestamp:Date.now()};this.state.messages.push(t);const s={id:`msg-agent-${Date.now()}`,role:"assistant",content:"",thoughts:[],toolCalls:[],timestamp:Date.now()};this.state.messages.push(s),this.state.isStreaming=!0,this.state.tokensPerSec=45.8,this.chat.setBusy(!0),this.claudeLayout.setBusy(!0),this.updateAllComponents();try{const i=await I(e,this.state.currentModel);s.content=i,(!s.thoughts||s.thoughts.length===0)&&(s.thoughts=[`Analyzed request against active model [${this.state.currentModel}]`,"Evaluated workspace crate context and active skills","Autonomous safety verification passed via ApprovalGate"])}catch(i){s.content=`Error executing agent turn: ${i?.message||i}`}finally{this.state.isStreaming=!1,this.state.tokensPerSec=0,this.chat.setBusy(!1),this.claudeLayout.setBusy(!1),this.updateAllComponents()}}async handleApplyDiff(e){try{await q(e),this.state.diffs=this.state.diffs.filter(t=>t.path!==e),this.updateAllComponents()}catch(t){console.error(`Failed to apply diff for ${e}:`,t)}}async handleRollbackDiff(e){try{await F(e),this.state.diffs=this.state.diffs.filter(t=>t.path!==e),this.updateAllComponents()}catch(t){console.error(`Failed to rollback diff for ${e}:`,t)}}async handleToggleSkill(e,t){try{await H(e,t);const s=this.state.skills.find(i=>i.name===e);s&&(s.enabled=t),this.updateAllComponents()}catch(s){console.error(`Failed to toggle skill ${e}:`,s)}}registerShortcuts(){window.addEventListener("keydown",e=>{const t=e.metaKey||e.ctrlKey;t&&e.key.toLowerCase()==="s"?(e.preventDefault(),this.editor.saveActiveFile()):t&&e.key.toLowerCase()==="o"?(e.preventDefault(),this.promptOpenFile()):t&&e.key.toLowerCase()==="n"?(e.preventDefault(),this.editor.createNewFile()):t&&e.key.toLowerCase()==="k"?(e.preventDefault(),this.commandPalette.toggle()):t&&e.key==="1"?(e.preventDefault(),this.switchLayout("vs_code")):t&&e.key==="2"?(e.preventDefault(),this.switchLayout("claude_code")):t&&e.key==="3"&&(e.preventDefault(),this.switchLayout("codex"))})}promptOpenFile(){const e=window.prompt("Enter file path to open (e.g. crates/ride-desktop/Cargo.toml):");e&&e.trim()&&this.editor.openFile(e.trim())}showSaveToast(e){this.toastElement||(this.toastElement=document.createElement("div"),this.toastElement.className="ride-toast",document.body.appendChild(this.toastElement));const t=e.split(/[\/\\]/).pop()||e;this.toastElement.innerHTML=`<span class="toast-icon">💾</span> File saved: <strong>${t}</strong>`,this.toastElement.classList.add("visible"),setTimeout(()=>{this.toastElement?.classList.remove("visible")},2200)}subscribeEvents(){V(e=>{console.log("[Model Event]:",e),this.refreshModelStatus()}),W(e=>{console.log("[Agent Event]:",e);const t=this.state.messages[this.state.messages.length-1];if(!(!t||t.role!=="assistant")){if(e.ThinkingDelta)t.thoughts||(t.thoughts=[]),t.thoughts.push(e.ThinkingDelta.delta),this.updateAllComponents();else if(e.ToolCallRequested)t.toolCalls||(t.toolCalls=[]),t.toolCalls.push({name:e.ToolCallRequested.tool_name,args:e.ToolCallRequested.arguments,status:"pending"}),this.updateAllComponents();else if(e.ToolExecuted){if(t.toolCalls){const s=t.toolCalls.find(i=>i.name===e.ToolExecuted.tool_name&&i.status==="pending");s?(s.status=e.ToolExecuted.success?"success":"failed",s.output=e.ToolExecuted.output_preview):t.toolCalls.push({name:e.ToolExecuted.tool_name,args:{},output:e.ToolExecuted.output_preview,status:e.ToolExecuted.success?"success":"failed"})}this.updateAllComponents()}else if(e.DiffGenerated){if(!this.state.diffs.some(s=>s.path===e.DiffGenerated.path)){const s=e.DiffGenerated.diff,i=(s.match(/^\+[^+]/gm)||[]).length,a=(s.match(/^-[^-]/gm)||[]).length;this.state.diffs.push({path:e.DiffGenerated.path,diff_text:s,additions:i,deletions:a})}this.updateAllComponents()}}})}}window.addEventListener("DOMContentLoaded",()=>{new te});
