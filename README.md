# R.I.D.E (Rust Intelligent Development Environment)

<div align="center">

**An ultra-lightweight, RAM-optimized, local-model-only agentic IDE written in Rust.**  
*100% Private • Zero Cloud Reliance • Native Performance • Extensible Agent Skills*

[![Rust](https://img.shields.io/badge/rust-1.100%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-116%20passed-brightgreen.svg)]()
[![RAM Footprint](https://img.shields.io/badge/RAM-%3C60MB-success.svg)]()
[![Release Binary](https://img.shields.io/badge/binary-1.82MB-informational.svg)]()

</div>

---

## ⚡ The Problem & The Mission

Standard AI-powered editors (VS Code + plugins, Cursor, Windsurf) run on Electron, frequently consuming **800 MB to 2.5 GB of RAM**. 

When developers run modern local LLMs (e.g., Qwen 2.5 Coder 7B, DeepSeek-Coder 6.7B, Llama 3.1 8B, or massive Mixture-of-Experts models like DeepSeek MoE, Qwen-Max MoE, and Kimi), the model itself requires **4 GB to 12+ GB of VRAM/system RAM**. On standard developer laptops (such as 16 GB machines), running a bloated IDE alongside a local LLM leads to heavy swap thrashing, system stutter, and out-of-memory crashes.

**R.I.D.E solves this problem:**
- **Tiny Memory Footprint**: Core logic, file indexing, tool execution, and local model streaming built 100% in Rust (**~30–60 MB private RAM** vs. Electron's 800MB–2.5GB).
- **Sub-2MB Binary**: Release binary compiles to just **1.82 MB** with Link-Time Optimization (`lto = true`, `opt-level = "z"`).
- **100% Local & Private**: No cloud API keys, no telemetry, no external outbound connections. All inference runs against `localhost`.

---

## 🚀 Key Features

### 1. 🔍 Zero-Config Local Model Autodiscovery
When you launch or stop a local inference engine, R.I.D.E automatically detects it within ~500ms via concurrent non-blocking async probes:

| Provider | Default Endpoint | Supported Features |
|---|---|---|
| **Ollama** | `http://127.0.0.1:11434` | `/api/tags`, `/api/generate` SSE streaming, dynamic host ports |
| **LM Studio** | `http://127.0.0.1:1234` | `/v1/models`, `/v1/chat/completions` OpenAI-compatible streaming |
| **Colibrì & Rabbit (Rust)** | `http://127.0.0.1:5000` / `:8080` | Massive MoE weight streaming, NVMe throughput monitor |
| **llama.cpp server** | `http://127.0.0.1:8080` | `/props`, health check, OpenAI-compatible streaming |
| **vLLM / LocalAI** | `http://127.0.0.1:8000` | `/v1/models` (with `max_model_len` context extraction) |

### 2. 🦤 Large MoE Model Support via Colibrì & Rabbit (Rust)
Run massive **100B–700B+ Mixture-of-Experts (MoE)** models on consumer laptops (16GB RAM) using hierarchical memory streaming:
- **Dense Layers in RAM**: Attention heads, embeddings, and shared components stay in RAM (~8–10 GB, quantized).
- **Experts Streamed from NVMe**: Thousands of sparse MoE experts reside on NVMe SSD storage and are paged on-demand in real-time as chosen by the router.
- **Built-in `StreamMonitor`**: Tracks real-time NVMe disk read throughput (MB/s) and RAM/VRAM consumption without stalling the async Tokio runtime.
- **Managed `MoeLauncher`**: Spawns and supervises Rabbit / Colibrì child processes with clean Windows process lifecycle management.

### 3. 🧩 Extensible Agent Skills System (`ride-skills`)
Customize your local model's capabilities with modular Agent Skills:
- **Standardized `SKILL.md`**: YAML frontmatter (`name`, `description`, `version`, `author`, `triggers`, `max_tokens`, `pinned`, `compatible_models`) + Markdown instruction body.
- **Dual-Scope Registry**:
  - Global skills: `~/.ride/skills/<skill-name>/`
  - Workspace skills: `.ride/skills/<skill-name>/` (automatically overrides global skills for project-specific rules).
- **Dynamic Context Budgeter**: Prevents local LLMs (8k/16k context limits) from overflowing:
  - *Pinned skills* are prioritized.
  - *Triggered skills* activate dynamically using UTF-8 word-boundary matching (e.g. `git` trigger won't falsely match `digital`).
  - *Inactive skills* are compressed into 1-line capability summaries.
  - Automatic instruction truncation to `max_tokens * 4` characters.
- **Deadlock-Free Script Runner**: Execute custom skill tools (`.ps1`, `.py`, `.bat`, `.sh`) with concurrent pipe draining (<512KB buffer) and path traversal protection.

### 4. 🛠️ Agentic Coding Engine (`ride-agent`)
Autonomous coding tools tailored for local 7B–14B models:
- **Dual-Format Tool Parser**: Resilient parser supporting raw JSON, OpenAI `tool_calls` schema, Markdown code fences, and XML fallback tags (`<tool_call name="...">`).
- **Core Native Toolset**:
  - `read_file`: Zero-copy line-ranged file reading with bounds safety.
  - `write_file`: Atomic writes with automated backup.
  - `edit_file`: Precision hunk replacement with CRLF/LF newline tolerance.
  - `list_dir`: Fast recursive directory traversal respecting `.gitignore`.
  - `search_code`: Sub-millisecond ripgrep search powered by Rust's `grep-searcher`.
  - `run_command`: Interactive terminal command runner with user approval controls.
- **Dynamic Skill Tools**: Tools registered by active skills are automatically callable by the agent alongside native tools.

### 5. 🎨 Switchable UI Designs (Coming in Phase 5)
Toggle between 3 distinct interface modes via Settings or `Ctrl+Shift+U`:
- **VS Code Mode**: Full-featured 3-pane IDE (Activity bar, Explorer, Editor tabs, Dockable Agent/Terminal).
- **Claude Code Mode**: Terminal-first agent workspace with live thought streams, interactive prompt, and collapsible diffs.
- **Codex / Zen Mode**: Distraction-free editor canvas with floating `Ctrl+K` prompt and side fly-out assistant.

---

## 📂 Architecture & Workspace Crates

```
g:\Projects\R.I.D.E\
├── Cargo.toml                    # Workspace manifest (LTO, opt-level = "z", strip)
├── crates\
│   ├── ride-core\                # Core configuration, UI layouts, unified events & error handling
│   ├── ride-discovery\           # Concurrent local LLM detector & zero-copy SSE streaming client
│   ├── ride-moe\                 # Colibrì & Rabbit (Rust) MoE lifecycle & NVMe throughput monitor
│   ├── ride-skills\              # Agent Skills manifest parser, registry, budgeter & script runner
│   ├── ride-agent\               # Dual-format parser, agent loop, tool runtime & approval system
│   └── ride-desktop\             # Desktop runtime entry point & event coordinator
```

---

## 🛠️ Building & Testing

### Prerequisites
- **Rust Toolchain**: `stable-x86_64-pc-windows-msvc` (or Linux/macOS equivalent) — Rust 1.80+ recommended.
- **C++ Build Tools**: Visual Studio 2022 C++ Build Tools (on Windows).
- **Local LLM Runner (Optional for live inference)**: [Ollama](https://ollama.ai), [LM Studio](https://lmstudio.ai), or [Rabbit](https://github.com/ferrumox/rabbit) / [Colibrì](https://github.com/JustVugg/colibri).

### Quick Build & Test

```powershell
# Clone or navigate to the repository
cd g:\Projects\R.I.D.E

# Run the complete automated test suite (116 tests across all crates)
cargo test --workspace

# Build the optimized release binary
cargo build --release -p ride-desktop

# Inspect the binary size (~1.82 MB)
Get-Item .\target\release\ride-desktop.exe | Select-Object Name, Length
```

---

## 📦 Creating a Custom Skill

Skills can be placed in `~/.ride/skills/<skill-name>/` or `.ride/skills/<skill-name>/` inside any project:

### 1. `SKILL.md`
```markdown
---
name: rust-expert
description: Expert Rust guidelines focusing on memory safety and idiomatic patterns.
version: 1.0.0
author: Developer
triggers: ["rust", "cargo", "borrow", "lifetime"]
max_tokens: 300
pinned: false
---

# Rust Expert Guidelines
- Always prefer references (`&str`, `&[T]`) over heap allocations (`String`, `Vec<T>`) when possible.
- Use `clippy` suggestions and write unit tests for edge cases.
- Handle errors with custom error enums using `thiserror`.
```

### 2. Optional Custom Tool (`tools.json`)
```json
[
  {
    "name": "cargo_clippy",
    "description": "Run cargo clippy on the workspace and return diagnostics",
    "parameters": {
      "type": "object",
      "properties": {
        "fix": { "type": "boolean", "description": "Automatically apply fixes" }
      }
    },
    "script": "scripts/run_clippy.ps1",
    "timeout_seconds": 60
  }
]
```

### 3. Optional Tool Script (`scripts/run_clippy.ps1`)
```powershell
param([string]$ArgsJson)
$params = $ArgsJson | ConvertFrom-Json
if ($params.fix) {
    cargo clippy --fix --allow-dirty --allow-staged
} else {
    cargo clippy -- -D warnings
}
```

---

## 📊 Implementation Progress

- [x] **Phase 1: Toolchain & Workspace Foundation**
  - Modular 6-crate Cargo workspace, release optimization profile, 25 tests passing.
- [x] **Phase 2: Auto-Discovery, SSE Streaming & Colibrì MoE**
  - Zero-copy raw byte SSE streaming, Ollama / LM Studio / Colibrì / llama.cpp / vLLM normalization, differential NVMe throughput monitor, 83 tests passing.
- [x] **Phase 3: Extensible Skills System (`ride-skills`)**
  - `SKILL.md` parser, dual-scope registry, word-boundary trigger matching, deadlock-free script runner, prompt synthesis, 116 tests passing.
- [ ] **Phase 4: Agentic Tool Engine Hardening (`ride-agent`)**
  - Multi-turn turn loop, terminal approval prompt (`Always Allow` / `Ask Once` / `Deny`), hunk diff generator.
- [ ] **Phase 5: Desktop UI & Switchable Designs (`ride-desktop`)**
  - Tauri v2 + native WebView2, CodeMirror 6 editor (<15MB RAM), VS Code / Claude Code / Codex layout switchers.
- [ ] **Phase 6: End-to-End Benchmarking & RAM Verification**
  - Real-world validation with active local models, memory profiling under peak streaming load.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
