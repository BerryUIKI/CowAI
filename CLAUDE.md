# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

CowBot is an open-source, high-performance multi-agent collaboration platform with code execution and automation capabilities, built in Rust. It utilizes an event-driven message bus for decoupled multi-agent communication and modular tool sandboxing.

## Project Status & Workflow

**Current phase: Phase 1 (local/offline MVP) implementation under GitFlow.** The design docs are agreed; milestones 1.1–1.5 are implemented on `dev`. Continue with the remaining roadmap milestones, keeping the documentation in sync as each one lands.

- **Implemented baseline**: the workspace crates below, a working sequential agent crew, a local subprocess code sandbox, an OpenAI-compatible + mock LLM client, SQLite persistence via `cowbot-store`, a transport-neutral `cowbot-client` with a local backend, the `cowbot` CLI (`init`, `run`, `tools`), and a Tauri v2 desktop shell with React channels, tasks, and approvals views. Tests, `clippy -D warnings`, and `cargo fmt --check` pass.
- **Known gaps** (not yet built): `cowbot-client` remote mode, desktop agents/templates management and settings, starting task execution from the desktop, `.cowpkg` workspace packaging, a wake/re-entry scheduler for delegated assignments (budget enforcement itself lives in `AssignmentService`), hierarchical orchestration, agent long-term memory, isolated sandbox backends (Docker/WASM), REST/WebSocket surface, config file support, streaming responses, retries/rate limiting/token accounting, and channel-based collaboration wired into the CLI.
- **Documentation**: `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/ADR/` (0001–0011), `docs/TEMPLATE_FORMAT.md`, and `docs/REFERENCES.md` (external design references). Update them with every milestone.
- **Commit style**: small, atomic commits; frequent pushes and small PRs.
- **Language**: all repository artifacts — code, comments, docs, commit messages, and branch names — must be English; conversation with the user may be Chinese.
- **Git workflow**: GitFlow with `main` as the release branch and `dev` as the integration branch (see below).

## Git Workflow (GitFlow)

- **Strict rule**: NEVER push directly to `dev` or `main`. Both branches have GitHub branch protection enabled with pull request requirements and force-push/deletion blocked.
- `main` — release-ready history only; protected.
- `dev` — integration branch for completed features; protected. All feature pull requests must target `dev`.
- `feature/<name>` — branched from `dev`, pushed to origin, and merged into `dev` via Pull Request (`gh pr create --base dev`).
- `release/<version>` — branched from `dev` when preparing a release, merged into both `main` and `dev` via Pull Request.
- `hotfix/<name>` — branched from `main` for urgent fixes, merged into both `main` and `dev` via Pull Request.
- Commit messages are small, atomic, and English; always use PRs to merge into `dev`.

## Language

All repository artifacts — code, comments, documentation, commit messages, and branch names — must be written in English. Conversation with the user may be in Chinese; do not add mixed-language sections to docs.

## Workspace Architecture

The project is structured as a Cargo workspace under `crates/`:

- **`crates/cowbot-core`**: Core domain types and shared errors (`CowError`). Defines `AgentProfile`, `Task`, `Channel`/`ChannelMessage`, the `Tool` trait, and the `Event`/`EventPayload` bus message model.
- **`crates/cowbot-bus`**: Tokio broadcast-backed `EventBus` with subscriber handles and a bounded in-memory event history.
- **`crates/cowbot-tools`**: `ToolRegistry` plus the built-in tools (`CodeExecutionTool`, `ReadFileTool`, `WriteFileTool`), the pluggable `SandboxBackend` trait implemented by `SubprocessSandbox`, and the deterministic `StaticRiskClassifier`.
- **`crates/cowbot-llm`**: `LlmProvider` abstraction with an OpenAI-compatible HTTP client (works with OpenAI, DeepSeek, Ollama, etc.) and a `MockLlmProvider` for offline tests and dry runs.
- **`crates/cowbot-agent`**: Agent runtime loop (LLM call → tool calls → observations), an optional `ToolGate` consulted before each tool execution, `Crew` orchestration (`ProcessType::Sequential`), and preset roles in `StandardRoles` (Planner, Coder, Reviewer).
- **`crates/cowbot-store`**: SQLite persistence built on `sqlx`: embedded migrations, connection management, and repository methods for identity, collaboration, agents/templates, tasks/assignments/runs/tool calls/approvals, and audit events.
- **`crates/cowbot-client`**: Transport-neutral `Client` trait and the local-mode `LocalClient`, which opens SQLite, bootstraps the local workspace, runs startup reconciliation, and exposes session, channel/message, template, agent, task/assignment/approval, history (runs, tool calls, assignment events, audit), and event-stream operations without leaking store or service types.
- **`crates/cowbot-service`**: Application services for the local runtime: idempotent local owner/workspace bootstrap, channel/message operations that persist first and then publish events, the assignment runtime (tree lifecycle, event log, budgets, stall guard, restart reconciliation), the approval service (risk policy, pending approvals, audit), a sequential crew runner built on assignments, and template management with built-in templates, validation, CRUD, and `.cowtemplate` import/export.
- **`crates/cowbot-cli`**: The `cowbot` binary. `init` opens the local database through `cowbot-client` and prints the session; `run` executes the Planner/Coder/Reviewer crew through `LocalClient::run_crew` with persisted history, `--db`, `--dry-run`, and an interactive `--approval` policy; `tools` inspects the registry and smoke-tests the sandbox.

## Desktop App

The desktop app lives under `desktop/` and is not part of the Cargo workspace:

- **`desktop/`**: React 19 + TypeScript + Vite frontend. It has three views — Channels (list, create, timeline, composer), Tasks (list, creation form with agent bootstrap from templates, detail with assignments/events/runs/tool calls and cancellation), and Approvals (pending list with approve/reject) — and refreshes from Tauri events.
- **`desktop/src-tauri/`**: Standalone Tauri v2 Cargo project. Opens `LocalClient` on `app_data_dir()/cowbot.db`, exposes Tauri commands that wrap the `Client` trait (session, channels/messages, agents/templates, tasks, assignments, runs, tool calls, approvals), and forwards client events on the `cowbot-event` channel.
- **Architecture**: see `docs/ADR/0009-desktop-ipc-and-state.md`. The frontend never touches SQLite or `cowbot-service`.
- **Gates**: root `cargo fmt/clippy/test --workspace` intentionally excludes `src-tauri`; run `npm run build` and `cargo clippy --manifest-path desktop/src-tauri/Cargo.toml --all-targets -- -D warnings` when changing the shell.

## Common Development Commands

### Building
```bash
cargo build --workspace
cargo build --release --workspace
```

### Running the CLI
```bash
cargo run -p cowbot-cli -- --help
cargo run -p cowbot-cli -- tools
cargo run -p cowbot-cli -- init --db cowbot.db
cargo run -p cowbot-cli -- run --task "Analyze and write code" --dry-run
```

### Running the Desktop App
```bash
cd desktop
npm install
npm run tauri dev        # dev window + Vite server
npm run build            # type-check and bundle the frontend
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

### Testing
```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p cowbot-core
cargo test -p cowbot-bus
cargo test -p cowbot-tools

# Run a single specific test
cargo test -p cowbot-bus -- test_bus_publish_and_receive --exact
```

### Linting & Formatting
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo fmt
```
