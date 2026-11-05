# ADR 0009: Desktop Shell IPC and State

- **Status**: Accepted
- **Date**: 2026-11-04
- **Related**: `docs/ADR/0001-runtime-and-deployment.md`, `docs/ADR/0007-client-api.md`, `docs/ARCHITECTURE.md` §3.9
- **Superseded in part**: [0012](0012-local-model-configuration.md) replaces the provider-selection paragraph in §6; the desktop now reads providers from its local `config.json` instead of the environment.

## Context

ADR-0001 fixes Tauri for the desktop app and `cow-client` as the only data boundary. The first desktop slice must boot the local runtime, render channels and messages, and react to live events without letting the frontend touch SQLite or `cow-service`.

## Decision

### 1. Stack and repository layout

The desktop app lives under `desktop/`:

- `desktop/` — React 19 + TypeScript + Vite frontend.
- `desktop/src-tauri/` — Tauri v2 Rust backend, a **standalone Cargo project** (`[workspace]` empty), depending on `cow-client`/`cow-core` by path.

Keeping `src-tauri` outside the root workspace keeps the established `cargo fmt/clippy/test --workspace` gates fast and free of Tauri toolchain requirements. Desktop checks run separately (`npm run build`, `cargo clippy --manifest-path src-tauri/Cargo.toml ...`).

### 2. Command boundary

The frontend reaches the core only through Tauri commands that wrap the `Client` trait:

- `get_session`
- `list_channels`
- `create_channel`
- `list_messages`
- `post_message`

Commands return domain types from `cow-core` and `cow-client` (`ClientSession` gained `Serialize`). Errors cross as strings for now; a structured error envelope can replace them when the UI needs programmatic handling.

### 3. State and lifecycle

The Tauri setup hook opens `LocalClient` on `app_data_dir()/cowai.db`, so migrations, local bootstrap, and assignment/approval startup reconciliation run at app launch. The client is stored as `Arc<LocalClient>` in managed `AppState`. The desktop database is separate from the CLI's `cowai.db` in the working directory.

### 4. Events

The backend subscribes to `Client::subscribe` once at startup and forwards every event on a single Tauri channel, `cowai-event`. The frontend refreshes channels on `ChannelCreated` and messages on `MessagePosted`; it does not interpret task or approval payloads yet.

### 5. UI scope

The first slice renders the workspace, channel list, message timeline, and composer. Tasks, approvals, agents/templates, and settings are follow-up slices. Future execution work keeps `LocalClient::run_crew` on the Rust side; the UI will start runs through commands, never by building a crew in the frontend.

### 6. Desktop execution

The `run_task` command starts the standard Planner/Coder/Reviewer crew with the
submitted task's description as the objective. It returns as soon as the run is
spawned on `tauri::async_runtime`; the outcome arrives later on a second Tauri
event channel, `cowbot-run-finished`, carrying `{ task_id, ok, error }`. The
existing `cowbot-event` channel continues to carry the crew's domain events.

The crew's LLM provider and tool registry are built once at startup and shared
through `AppState`: `OPENAI_API_KEY` or `OPENAI_BASE_URL` selects the
OpenAI-compatible provider, otherwise the mock provider runs offline. File tools
are rooted at the app data directory.

A task id is tracked in a `running` set for the duration of its run; a second
`run_task` for the same id is rejected. Cancelling an in-flight crew is not
supported yet — the UI disables Cancel while a task is running.

## Consequences

- The desktop app and CLI share `cow-client` but keep independent SQLite files.
- Adding a UI feature means adding a command and a typed wrapper, keeping the boundary explicit.
- The command list, event channel, and app identifier are stable contracts for the frontend.
- Remote mode later swaps the managed client implementation without changing the frontend command names.
