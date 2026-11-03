# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

CowAI is an open-source, high-performance multi-agent collaboration platform with code execution and automation capabilities, built in Rust. It utilizes an event-driven message bus for decoupled multi-agent communication and modular tool sandboxing.

## Project Status & Workflow

**Current phase: functional architecture design and documentation.** Do not start new feature implementation until the design docs under `docs/` are agreed with the user. The agreed deliverable order is: align on functional architecture → write the documentation → continue development.

- **Implemented baseline**: the workspace crates below, a working sequential agent crew, a local subprocess code sandbox, an OpenAI-compatible + mock LLM client, SQLite persistence via `cow-store`, and the `cowai` CLI (`run`, `tools`). Tests, `clippy -D warnings`, and `cargo fmt --check` pass.
- **Known gaps** (not yet designed/built): hierarchical orchestration, persistence (tasks/messages/agent memory), human-in-the-loop tool approval, isolated sandbox backends (Docker/WASM), REST/WebSocket/Web UI surface, config file support, streaming responses, retries/rate limiting/token accounting, and channel-based collaboration actually wired into the CLI.
- **Documentation plan** (to be written with the user): `docs/PRD.md` (product requirements), `docs/ARCHITECTURE.md` (functional + system architecture), `docs/ADR/` (decision records), `docs/ROADMAP.md`, `docs/TEMPLATE_FORMAT.md`, and `docs/REFERENCES.md` (external design references).
- **Commit style**: small, atomic commits; frequent pushes and small PRs.
- **Language**: all repository artifacts — code, comments, docs, commit messages, and branch names — must be English; conversation with the user may be Chinese.
- **Git workflow**: GitFlow with `main` as the release branch and `dev` as the integration branch (see below).

## Git Workflow (GitFlow)

- `main` — release-ready history only.
- `dev` — integration branch for completed features; all feature work merges here first (the branch is `dev`, not `develop`).
- `feature/<name>` — branched from `dev`, merged back into `dev` with `--no-ff`.
- `release/<version>` — branched from `dev` when preparing a release, merged into both `main` and `dev`.
- `hotfix/<name>` — branched from `main` for urgent fixes, merged into both `main` and `dev`.
- Commit messages are small, atomic, and English; pull requests target `dev` unless they are release or hotfix PRs.

## Language

All repository artifacts — code, comments, documentation, commit messages, and branch names — must be written in English. Conversation with the user may be in Chinese; do not add mixed-language sections to docs.

## Workspace Architecture

The project is structured as a Cargo workspace under `crates/`:

- **`crates/cow-core`**: Core domain types and shared errors (`CowError`). Defines `AgentProfile`, `Task`, `Channel`/`ChannelMessage`, the `Tool` trait, and the `Event`/`EventPayload` bus message model.
- **`crates/cow-bus`**: Tokio broadcast-backed `EventBus` with subscriber handles and a bounded in-memory event history.
- **`crates/cow-tools`**: `ToolRegistry` plus the built-in tools (`CodeExecutionTool`, `ReadFileTool`, `WriteFileTool`) and the pluggable `SandboxBackend` trait, implemented by `SubprocessSandbox`.
- **`crates/cow-llm`**: `LlmProvider` abstraction with an OpenAI-compatible HTTP client (works with OpenAI, DeepSeek, Ollama, etc.) and a `MockLlmProvider` for offline tests and dry runs.
- **`crates/cow-agent`**: Agent runtime loop (LLM call → tool calls → observations), `Crew` orchestration (`ProcessType::Sequential`), and preset roles in `StandardRoles` (Planner, Coder, Reviewer).
- **`crates/cow-store`**: SQLite persistence built on `sqlx`: embedded migrations, connection management, and repository methods for identity, collaboration, agents/templates, tasks/runs/tool calls/approvals, and audit events.
- **`crates/cow-service`**: Application services for the local runtime: idempotent local owner/workspace bootstrap, channel/message operations that persist first and then publish events, and template management with built-in templates, validation, CRUD, and `.cowtemplate` import/export.
- **`crates/cow-cli`**: The `cowai` binary exposing `run` (crew execution, `--dry-run` for mock LLM) and `tools` (registry inspection + sandbox smoke test).

## Common Development Commands

### Building
```bash
cargo build --workspace
cargo build --release --workspace
```

### Running the CLI
```bash
cargo run -p cow-cli -- --help
cargo run -p cow-cli -- tools
cargo run -p cow-cli -- run --task "Analyze and write code" --dry-run
```

### Testing
```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p cow-core
cargo test -p cow-bus
cargo test -p cow-tools

# Run a single specific test
cargo test -p cow-bus -- test_bus_publish_and_receive --exact
```

### Linting & Formatting
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo fmt
```
