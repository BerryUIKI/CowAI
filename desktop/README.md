# CowBot Desktop

Tauri v2 shell with a React + TypeScript frontend. The Rust backend is a
standalone Cargo project (outside the root workspace) and reaches the local
core exclusively through `cowbot-client`. All data lives in `cowbot.db` under the
platform app-data directory, so the desktop app and the CLI can use separate
databases without sharing state. On first launch after the CowBot rename, an
existing `cowai.db` from the previous `com.cowai.desktop` directory is copied
into the new app-data directory automatically.

On first launch, a three-step setup guide introduces the local workspace, lets
the user connect a model provider, and adds a starter team from a built-in
template. It can be skipped at any point; completion is recorded in the same
`config.json`, and a configuration written before onboarding existed is treated
as completed so upgrades do not show the guide again.

The current UI has four views: **Channels** (channels and live messages),
**Tasks** (create tasks, inspect assignment/run history, cancel, and run a crew
on a submitted task), **Approvals** (approve or reject tool calls requested by
running agents), and **Settings** (model providers, agents, and templates).

Model providers are OpenAI-compatible endpoints kept in `config.json` under the
same app-data directory as `cowbot.db`. The Settings view can add, edit, test,
and switch providers; the active one is used by the next crew run, and no
active provider means the offline mock model. API keys are stored in plain text
in that file and are never sent back to the frontend. Environment variables do
not configure the desktop (the CLI keeps its flags and environment variables).

## Prerequisites

- Node.js 22.13+ and pnpm (`corepack enable pnpm`, or `npm install -g pnpm`)
- Rust toolchain
- On Windows, the WebView2 runtime (already present on current Windows 10/11)

## Run

```bash
pnpm install
pnpm run tauri dev
```

The dev server runs on `http://localhost:1420`; the Tauri window opens once the
backend compiles.

## Build

```bash
pnpm run build                    # type-check and bundle the frontend
pnpm run tauri build              # produce platform bundles
```

## Release builds

The [release workflow](../.github/workflows/release.yml) builds installers on manual dispatch and on `v*` tags:

- **Windows:** NSIS `setup.exe` and `.msi`
- **macOS:** universal `.dmg` for Apple Silicon and Intel
- **Linux:** `.deb` and `.AppImage`

Manual runs upload the bundles as workflow artifacts. Tag runs create or update a draft GitHub Release; a maintainer publishes it after review. The tag must match the version in `src-tauri/tauri.conf.json` (`v0.1.0` for `0.1.0`). GitHub offers manual dispatch only once the workflow exists on the default branch; tag builds run from the tagged commit.

Builds are unsigned: macOS may require allowing the app in System Settings, and Windows may show a SmartScreen prompt. Signing, notarization, and auto-updates are follow-up work (ADR-0014).

Build bundles locally with:

```bash
pnpm run tauri build -- --bundles nsis,msi      # Windows
pnpm run tauri build -- --bundles dmg           # macOS
pnpm run tauri build -- --bundles deb,appimage  # Linux
```

Bundles are written under `src-tauri/target/release/bundle/`, or under the target-specific directory when `--target` is used.

## Quality checks

```bash
pnpm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

The root workspace gates (`cargo fmt/clippy/test --workspace`) intentionally do
not include `src-tauri`; run the desktop checks above when changing the shell.
