# UI redesign: progress and leftover work

Status as of 2026-11-21 (second session). Nothing is committed yet. All changes are in the working tree on `master`.

## Goal

Give R.I.D.E a VS Code / Codex-quality UI across its three layouts (Editor, Agent, Zen), and fix bugs and performance problems along the way.

## Design reference

- `design/Main.html`: Editor layout (VS Code style)
- `design/Agent.html`: Agent layout (terminal-first)
- `design/Zen.html`: Zen layout (Codex-style, inline Ctrl+K edit)
- `design/source/`: original canvas sources (`*.dc.html`, `canvas.json`)

All three pages open directly in a browser. The visual system is called "Graphite": dark neutral surfaces, one orange accent (`#E8743B`), Inter-style UI font and JetBrains Mono/Cascadia for code, and inline SVG icons (no emoji).

## Done

### Rust backend (`crates/ride-desktop`)
- **Workspace root detection** (`commands.rs`): walks up from the current directory to the Cargo `[workspace]` root (falls back to `.git`, and the `RIDE_WORKSPACE` env var overrides). The process cwd is moved to that root at startup (`main.rs` setup).
  - Fixes a bug: under `tauri dev`, relative paths like `crates/ride-desktop/src/main.rs` failed because cwd was `crates/ride-desktop`.
- **New commands**, registered in `main.rs`: `get_workspace_info`, `open_workspace`, `list_dir` (directories first, skips `.git`/`target`/`node_modules`), `search_workspace` (reuses `ride_agent::Toolset::search_code_full`, capped at 300 hits).
- `read_file` / `save_file` now resolve paths against the workspace and use `tokio::fs`, so they no longer block the async runtime.
- `send_agent_message`: dropped `event_listener.abort()`, which threw away buffered agent events at the end of each turn. It now drops the session and drains events with a 500 ms timeout.
- Added 3 tests: workspace detection, dir listing order/ignore rules, absolute-path resolution. `cargo test -p ride-desktop` gave **26 passed**.

### Frontend (`crates/ride-desktop/ui/src`)
The UI layer was rewritten. `npx tsc --noEmit` passes.
- `style.css`: a new token-based design system with dark and light themes, all colours as CSS variables (CodeMirror uses them too).
- `icons.ts`: an SVG icon set. `util.ts` holds escaping, toasts, in-app dialogs (these replace `window.prompt`), a safe markdown renderer, and copy buttons.
- `components/TitleBar.ts`: command center (Ctrl+K), layout switcher, Open Folder.
- `components/Editor.ts`:
  - **Each tab has its own `EditorState`.** Fixes a bug where undo history leaked across files: Ctrl+Z after switching tabs could restore another file's content.
  - Lazy-loaded syntax highlighting for Rust, TS/JS, JSON, Markdown, CSS, HTML, Python, TOML, YAML and shell.
  - Graphite theme, cursor position reporting, confirm before closing a dirty tab, middle-click close, breadcrumbs, welcome screen when no tabs are open, `reloadIfClean()` after a diff is applied.
  - Removed the hard-coded fake file contents.
  - Fixes a bug where Ctrl+S saved twice (CodeMirror keymap plus global handler).
- `components/FileTree.ts`: a real explorer backed by `list_dir`. Lazy expand, keyboard navigation, reveals the active file, marks modified files. This replaces the hard-coded tree.
- `components/SearchPanel.ts`: real workspace search with match-case and regex toggles, debounced, with stale-request protection and highlighted matches.
- `components/AgentChat.ts`:
  - Re-renders only when its revision number changes. Previously it re-rendered every 3 s, which collapsed expanded sections and jumped the scroll position.
  - Keeps expanded "thoughts" open across re-renders and only auto-scrolls if you were already at the bottom.
  - Adds a model picker, an active-file context chip, suggestions and an auto-growing composer.
- `components/DiffViewer.ts`: one instance per layout (layouts no longer take each other's DOM node). Adds Apply all and per-file busy states.
- `components/SkillsManager.ts`: typing in the filter no longer loses focus, because only the list re-renders.
- `components/CommandPalette.ts`: grouped commands with keybindings and fuzzy matching. Fixes a NaN selection when arrowing through an empty list.
- `components/StatusBar.ts`: shows provider and latency, model, live turn timer, pending changes and Ln/Col/language. Removed the fake "45.8 tok/s" and "42 MB" figures.
- `layouts/VsCodeLayout.ts`: activity bar, resizable side bar and agent panel (widths persisted), and a clickable models panel for picking a model.
- `layouts/ClaudeCodeLayout.ts`: tool and reasoning stream, slash commands (`/clear`, `/diff`, `/editor`, `/help`), and a session/skills/changes aside.
  - Fixes a bug where `/clear` only cleared the DOM and the next update brought the messages back.
- `layouts/CodexLayout.ts`: Zen canvas. Ctrl+K opens an inline edit bar on the selection; Accept (Ctrl+Enter) applies the edit as an undoable change and is refused if the code changed meanwhile.
- `main.ts`:
  - Renders are coalesced into one `requestAnimationFrame`, and only the visible layout renders.
  - Provider polling went from every 3 s (a full localhost port scan each time) to every 15 s, plus backend discovery events, and re-renders only when something changed.
  - Removed the fabricated "reasoning" steps that were injected into every reply.
  - Light/dark theme toggle and warning on close with unsaved changes.
- `package.json`: added `@codemirror/lang-{javascript,json,markdown,css,html,python}`, `@codemirror/legacy-modes`, `@codemirror/language`, `@lezer/highlight`. Removed the unused `@codemirror/theme-one-dark`.

## Done in the second session (2026-11-21)

- **Parallel Rust pass:** it left no changes outside `ride-desktop`. `cargo test --workspace` passes (now 184 tests, 29 in `ride-desktop`).
- **Agent approvals:**
  - The desktop session used to run with `ApprovalPolicy::AlwaysAllow`, so the agent could run any shell command unprompted. It now uses `AllowWhitelisted`: read-only and whitelisted commands run directly, and everything else waits for the user.
  - A `TerminalApprovalGate` with an mpsc channel is installed *after* `with_event_sender`, because that builder rebuilds the gate.
  - Requests are parked in `DesktopState.pending_approvals` and answered by the new `respond_approval` command.
  - The UI shows a "Run shell command?" card with Allow once / Deny in both the chat panel and the Agent stream (`renderApprovals` in `AgentChat.ts`).
- **Cancel:** `cancel_agent_turn` cancels the turn's `CancellationToken` (kept in `DesktopState.active_turn`) and denies pending approvals. A second concurrent `send_agent_message` is now refused.
  - In the UI the send button turns into a stop button while busy, and Esc in the prompt also stops the turn.
  - Fixes a bug: `TurnCancelled` arrives as a bare string (serde unit variant) and was previously ignored.
- **Event-only diffs:** these are not unstaged proposals. The agent's `write_file`/`edit_file` have already written them to disk with a backup snapshot. They are now shown as "Applied by the agent", with **Revert** (rolls back one snapshot per edit through the shared `DiffManager`) and **Keep**. Apply all skips them. The editor reloads clean tabs when an agent edit lands.
- **Build:** `vite.config.ts` still referenced the removed `@codemirror/theme-one-dark` in `manualChunks`, which broke `npm run build`. The chunk is now just the editor core, and `dist/` is rebuilt (not committed yet).
- **Smaller fixes:** removed CodeMirror's default dotted focus outline, removed the unused `dark-theme` body class, and added a "Stop Agent Turn" palette command.
- **Preview config:** added `.claude/launch.json` (`ride-ui` runs `npm run dev` in `ui/`). The browser mocks in `api.ts` now simulate a full turn: say "run"/"test"/"cargo" to get an approval, and "edit"/"fix" to get an applied diff.
- **README:** layout names, agent safety behaviour and the shortcut table are updated.
- **Checked in the browser preview:** Editor, Agent and Zen layouts in dark and light themes, with approve → edit → revert and stop-while-awaiting-approval.

## Leftover work (next session)

1. **Run the real app** (`cargo tauri dev` from `crates/ride-desktop`) against a live Ollama or LM Studio model. Check that a non-whitelisted command (for example `cargo build`) raises the approval card, and that Stop interrupts streaming inference. Only the browser mocks have been exercised so far.
2. **Fonts:** Inter and JetBrains Mono are not bundled (Segoe UI and Cascadia are used as fallbacks). Optionally add woff2 files under `ui/public/fonts`; the app must stay offline.
3. **Approval policy setting:** the policy is hard-coded to `AllowWhitelisted` in `run_agent_turn`. Consider exposing it (and the whitelist) in the UI.
4. **Commit**, for example as `feat(desktop): Graphite UI redesign, agent approvals & stop, real explorer/search`.

## Keyboard shortcuts (implemented)

| Keys | Action |
|---|---|
| Ctrl+K | Command palette (inline edit in Zen) |
| Ctrl+P | Command palette |
| Ctrl+S / Ctrl+O / Ctrl+N / Ctrl+W | Save / open by path / new / close tab |
| Ctrl+B / Ctrl+J / Ctrl+L | Toggle side bar / toggle agent panel / focus chat |
| Ctrl+Shift+E / Ctrl+Shift+F | Explorer / workspace search |
| Ctrl+1 / 2 / 3 | Editor / Agent / Zen layout |
| Esc (in prompt, while busy) | Stop the agent turn |
