# R.I.D.E

**R.I.D.E** (Rust Intelligent Development Environment) is a lightweight, local-first code editor designed for developers running language models on their own hardware. It is written in Rust, starts in roughly 500 milliseconds, and runs within a 60 MB memory footprint.

---

## Why R.I.D.E?

Most modern AI-assisted editors rely on Electron and consume between 800 MB and 2.5 GB of system memory at idle. If you run a local model like Qwen 2.5 Coder 7B or Llama 3.1 8B through Ollama or LM Studio, that model needs between 5 GB and 9 GB of combined VRAM and system memory. On a common 16 GB development laptop, running both an Electron editor and a local model pushes the operating system into heavy disk swap, causing stutter and interface lag.

By keeping the editor lightweight, your system maintains the headroom to run local inference smoothly.

| Metric | R.I.D.E | VS Code (with Copilot) | Cursor / Windsurf |
|---|---|---|---|
| Idle RAM | ~25 MB | ~450 MB | ~850 MB |
| Active Inference RAM | < 60 MB | ~1.2 GB | ~2.1 GB |
| Network Dependency | None (Localhost only) | External Cloud Required | External Cloud Required |
| MoE Weight Streaming | Native NVMe streaming (via Rabbit) | Not supported | Not supported |
| Cold Startup | < 550 ms | 2.8 s | 3.5 s |
| Release Binary Size | 1.82 MB | ~220 MB | ~350 MB |

---

## Core Features

### Automatic Local Model Detection
On launch, local model servers are discovered automatically without requiring configuration files or manual port entry. An asynchronous prober checks known local ports concurrently on `localhost`:
- **Ollama**: Queries port 11434 (`/api/tags`), extracts model metadata, and connects to `/api/generate` for streaming.
- **LM Studio / OpenAI-compatible backends**: Queries port 1234 (`/v1/models`) and handles streaming chat completions.
- **Colibri & Rabbit**: Connects to port 5000 or 8080 to support local Mixture-of-Experts inference.
- **llama.cpp**: Checks port 8080 (`/props` and health endpoints).
- **vLLM / LocalAI**: Checks port 8000 and reads active context limits.

When a local server starts or stops while the editor is running, the background discovery loop detects the state change and updates the connection badge in the status bar.

### Large Model Support with Colibri and Rabbit
Running a 100B+ parameter Mixture-of-Experts model traditionally requires workstation-grade hardware with multiple graphics cards. The editor integrates the weight-streaming architecture introduced by Colibri and its Rust implementation, Rabbit.

Instead of loading the entire model into system memory, attention layers and dense components stay in RAM, while cold expert weights remain on an NVMe solid-state drive. During inference, active experts are memory-mapped and streamed into memory on demand. An integrated disk monitor tracks real-time read throughput and memory usage during token generation.

### Extensible Agent Skills
You can extend what the local model knows and what tools it can call by adding skill directories. Skills can live in your home directory (`~/.ride/skills/`) for global availability or in a project repository (`.ride/skills/`) for team-specific conventions.

Each skill contains a `SKILL.md` file with a YAML header and instruction body:
- **Token budgeting**: Local models often have 8k or 16k context limits. Prompt space is budgeted automatically: pinned skills load first, while triggered skills activate only when user prompts match defined keywords. Inactive skills are compressed into single-line summaries to conserve context window space.
- **Custom tool scripts**: Skills can declare executable scripts (written in Python, PowerShell, Bash, or compiled binaries) that the model can invoke to perform project tasks like running linters or querying local databases.
- **Deadlock protection**: Background process readers drain child process outputs concurrently, preventing pipe stalls when scripts emit large logs.

### Agentic Coding Engine
The editor provides a built-in tool runtime designed to work with local 7B to 14B parameter models:
- **Resilient format parser**: Smaller local models sometimes produce malformed JSON when calling tools. R.I.D.E accepts standard OpenAI function calls, Markdown JSON blocks, and XML tags (`<tool_call name=... >`).
- **File operations**: Includes `read_file` with bounded line ranges, atomic `write_file`, and whitespace-tolerant `edit_file` replacements.
- **Code search**: Fast code search powered by the same grep engine used in ripgrep.
- **Safe terminal execution**: Read-only and whitelisted commands (`git status`, `cargo check`, `cargo test`, …) run directly. Any other command, including anything destructive, pauses the agent and shows a **Run shell command?** card with **Allow once** and **Deny**.
- **Reversible edits**: The agent writes files with a backup snapshot. Each edit appears under **Changes** marked "Applied by the agent", where **Revert** restores the file and **Keep** removes it from the list.
- **Stop anytime**: While a turn runs, the send button becomes a stop button (or press `Esc` in the prompt). Stopping cancels model inference and denies any pending approval.

### Switchable Interface Layouts
Three interface layouts are available to match how you work:
1. **Editor** (`Ctrl+1`): A three-pane layout with a file explorer, workspace search, multi-tab code editor, and a resizable agent panel with chat and change review.
2. **Agent** (`Ctrl+2`): A terminal-first layout centered on the model's reasoning stream and tool log, with slash commands (`/clear`, `/diff`, `/editor`, `/help`) and a session and changes side panel.
3. **Zen** (`Ctrl+3`): A minimal, distraction-free editing canvas. `Ctrl+K` opens an inline edit bar on the selection, and `Ctrl+Enter` accepts the suggestion as an undoable change.

### Keyboard Shortcuts

| Keys | Action |
|---|---|
| `Ctrl+K` | Command palette (inline edit in Zen) |
| `Ctrl+P` | Command palette |
| `Ctrl+S` / `Ctrl+O` / `Ctrl+N` / `Ctrl+W` | Save / open by path / new file / close tab |
| `Ctrl+B` / `Ctrl+J` / `Ctrl+L` | Toggle side bar / toggle agent panel / focus chat |
| `Ctrl+Shift+E` / `Ctrl+Shift+F` | Explorer / workspace search |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Editor / Agent / Zen layout |
| `Esc` (in the prompt, while the agent works) | Stop the agent turn |

The theme (dark or light) can be switched from the command palette.

---

## Workspace Structure

The codebase is organized as a Cargo workspace with six focused crates:

```
crates/
  ride-core/       Shared configuration, layout definitions, unified error types, and event bus
  ride-discovery/  Concurrent local endpoint detection and SSE streaming client
  ride-moe/        Colibri and Rabbit process supervisor and NVMe throughput monitor
  ride-skills/     SKILL.md manifest parser, dual-scope registry, and token budgeter
  ride-agent/      Multi-format tool parser, execution runtime, and approval gates
  ride-desktop/    Application entry point and desktop event coordination
```

---

## Getting Started

### Prerequisites
- Rust 1.80 or newer with Cargo
- Visual Studio 2022 C++ Build Tools (on Windows) or standard GCC/Clang tooling (on Linux/macOS)
- Any running local inference provider (Ollama, LM Studio, or Rabbit)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/your-username/ride.git
cd ride

# Run the test suite (116 tests across all crates)
cargo test --workspace

# Build an optimized release binary
cargo build --release -p ride-desktop
```

The compiled binary will be placed at `target/release/ride-desktop`.

---

## Authoring an Agent Skill

Create a directory inside `.ride/skills/<skill-name>/` and add a `SKILL.md`:

```markdown
---
name: rust-best-practices
description: Coding guidelines for writing safe, idiomatic Rust.
version: 1.0.0
triggers: ["rust", "cargo", "clippy"]
max_tokens: 250
pinned: false
---

# Rust Guidelines
- Prefer borrowing (&str, &[T]) over heap allocation where feasible.
- Use explicit error types with thiserror instead of raw string errors.
- Ensure all public functions have descriptive doc comments.
```

When you ask a question containing any of the trigger words, the editor automatically injects the skill instructions into the model's system prompt within your specified token budget.

---

## Privacy & Telemetry

R.I.D.E is engineered with **zero cloud telemetry**. Your code, keystrokes, configurations, and AI prompts never leave your local machine:
- **No Remote Analytics:** No tracking, telemetry, or remote telemetry servers.
- **Localhost Loopback Only:** Local inference (Ollama, LM Studio, ColibriMoE, Llama.cpp) communicates exclusively over `127.0.0.1`.
- **Full Privacy Policy:** Review our complete [Privacy Policy](file:///g:/Projects/R.I.D.E/PRIVACY_POLICY.md) for full compliance details (Microsoft Store, GDPR, CCPA).

---

## License

This project is licensed under the MIT License.
