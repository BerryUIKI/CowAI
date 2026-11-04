# CowAI Desktop

Tauri v2 shell with a React + TypeScript frontend. The Rust backend is a
standalone Cargo project (outside the root workspace) and reaches the local
core exclusively through `cow-client`. All data lives in `cowai.db` under the
platform app-data directory, so the desktop app and the CLI can use separate
databases without sharing state.

The current UI has three views: **Channels** (channels and live messages),
**Tasks** (create tasks, inspect assignment/run history, cancel), and
**Approvals** (approve or reject tool calls requested by running agents).
Starting task execution from the desktop is a follow-up slice.

## Prerequisites

- Node.js 20+ and npm
- Rust toolchain
- On Windows, the WebView2 runtime (already present on current Windows 10/11)

## Run

```bash
npm install
npm run tauri dev
```

The dev server runs on `http://localhost:1420`; the Tauri window opens once the
backend compiles.

## Build

```bash
npm run build                    # type-check and bundle the frontend
npm run tauri build              # produce platform bundles
```

## Quality checks

```bash
npm run build
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

The root workspace gates (`cargo fmt/clippy/test --workspace`) intentionally do
not include `src-tauri`; run the desktop checks above when changing the shell.
