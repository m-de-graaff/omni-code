# Architecture

Omni Code is a Rust-based terminal AI IDE organized as a Cargo workspace
with 10 crates under `crates/`. Each crate has a single responsibility.

## Crate Dependency Graph

```
                    omni-cli  (binary entry point)
             /    /    |    \     \      \
      omni-term  omni-loader  omni-ai  omni-lsp  omni-vcs
       / |  \      |    \       |        |         |
omni-view |  omni-syntax  |   omni-event-+----+----+
      |   |       |       |       |
      +---+-------+-------+-------+
                   |
               omni-core  (leaf, minimal deps)
```

## Crate Responsibilities

### omni-core
Text primitives. Zero external deps beyond `ropey`.

- `Text` — rope-backed buffer with insert/remove/display
- `Selection` / `Range` — multi-cursor positions
- `Transaction` / `Operation` — atomic edit operations (insert/delete/retain)
- `History` — undo/redo stacks of inverse transactions

### omni-event
Central pub/sub event system.

- `Action` — enum of cross-component intents (Quit, OpenFile, Save, FocusNext, etc.)
- `EventBus` — tokio broadcast channel for global action dispatch
- `Hook` / `HookRegistry` — extensible action interception

### omni-syntax
Syntax highlighting via tree-sitter.

- `SyntaxTree` — wrapper around `tree_sitter::Tree`
- `HighlightConfig` / `HighlightEvent` — highlight query management
- `LanguageConfig` — per-language settings (extensions, comments, indent)

### omni-view
Frontend-agnostic editor state. **No TUI dependencies.**

- `Document` — text buffer + selection + history + path + modified flag
- `View` — viewport into a document (scroll, dimensions)
- `ViewTree` — slotmap-backed split layout (horizontal/vertical)

### omni-lsp
LSP client using `lsp-types` + custom async JSON-RPC transport.

- `LspClient` — manages a single language server process
- `Transport` — stdin/stdout JSON-RPC layer
- `ServerRegistry` — tracks available and active servers

### omni-ai
Multi-provider AI integration.

- `AiProvider` trait — `complete()` + `name()` + `is_available()`
- `OllamaProvider` / `OpenAiProvider` / `AnthropicProvider`
- `Message` / `Role` — chat message types
- `StreamEvent` — streaming response chunks

### omni-vcs
Git integration via gitoxide (pure Rust, no libgit2).

- `Repository` — open/validate a git repo
- `FileStatus` — working tree status (modified/staged/untracked)
- `DiffHunk` — line-level diff representation

### omni-loader
Configuration and resource loading.

- `EditorConfig` — TOML-based editor settings
- `Theme` — color theme definition
- `GrammarManager` — tree-sitter grammar discovery
- `paths` — platform-specific config/data/log directories

### omni-term
Terminal UI layer. Depends on ratatui + crossterm.

- `Component` trait — the contract for all UI elements:
  - `handle_key()`, `handle_mouse()`, `render()`, `cursor()`, `focusable()`, `init()`
- `EventResult` — Consumed / Ignored / Action / Callback
- `Compositor` — layered component stack (Helix pattern):
  - Events propagate front-to-back (popups first)
  - Rendering goes back-to-front (base layer first)
- `Context` — shared app state passed during event handling
- `CursorKind` — Block / Bar / Underline / Hidden
- `event_loop::run()` — main loop: poll → dispatch → render at ~60fps

### omni-cli
Binary entry point (`omni`).

- CLI argument parsing via clap
- Subsystem initialization (tracing, event bus, config, terminal)
- Launches the event loop

## Data Flow

```
Terminal Event (crossterm)
    → event_loop::run()
    → Compositor::handle_event()  (front-to-back dispatch)
    → Component::handle_key() / handle_mouse()
    → returns EventResult
        Consumed  → stop propagation
        Ignored   → try next layer
        Action    → handle_action() (global dispatch)
        Callback  → mutate compositor (push/pop layers)

Render cycle:
    event_loop → terminal.draw()
    → Compositor::render()  (back-to-front)
    → each Component::render(frame, area)
    → topmost cursor wins
```

## Key Design Decisions

- **omni-view is frontend-agnostic**: no ratatui dep — could support a GUI frontend later
- **Helix compositor pattern**: simple Vec<Box<dyn Component>> stack with directional dispatch
- **color-eyre over anyhow**: colored backtraces + panic hooks for TUI apps
- **tracing over log**: structured logging to file (stdout is the TUI)
- **ropey for text**: battle-tested rope (used by Helix, Lapce), O(1) clone
- **gitoxide over libgit2**: pure Rust, no C dependency
- **lsp-types + custom transport**: tower-lsp is for servers, not clients
