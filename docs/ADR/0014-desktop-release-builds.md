# ADR 0014: Desktop Release Builds

- **Status**: Accepted
- **Date**: 2026-11-05
- **Related**: `docs/ADR/0001-runtime-and-deployment.md`, `docs/ADR/0009-desktop-ipc-and-state.md`, `docs/ADR/0013-first-run-onboarding.md`, `docs/ROADMAP.md` milestone 1.7

## Context

The desktop app builds locally with `npm run tauri build`, but CI has never
produced installers, so there is no distribution path for the Phase 1 exit
criterion "a new user can install the app and complete first-run onboarding
offline". The project has no code-signing certificates, Apple notarization
credentials, or auto-update infrastructure, and Phase 1 does not need them.

Tauri v2 already bundles the required formats from one project: NSIS and MSI on
Windows, DMG on macOS, and deb/AppImage on Linux. The window, icons, identifier,
and version are configured in `desktop/src-tauri/tauri.conf.json`.

## Decision

### 1. Release workflow

`.github/workflows/release.yml` runs on manual dispatch and on `v*` tags:

- **Windows** (`windows-latest`): NSIS `.exe` and `.msi`.
- **macOS** (`macos-latest`): a universal `.dmg` built with
  `--target universal-apple-darwin` for both Apple Silicon and Intel.
- **Linux** (`ubuntu-22.04`): `.deb` and `.AppImage`. Ubuntu 22.04 keeps the
  glibc and WebKit baseline compatible with older distributions.

Manual runs build the bundles and upload them as workflow artifacts. Tag runs
additionally create or update a **draft** GitHub Release through
`tauri-apps/tauri-action`; a maintainer reviews the assets and publishes it. A
tag must match the app version, so `v0.1.0` requires
`"version": "0.1.0"` in `tauri.conf.json`; the workflow fails otherwise.

### 2. Unsigned by default

Builds are not signed or notarized in this milestone, and the workflow does not
generate auto-updater metadata (`uploadUpdaterJson` and updater signatures are
disabled). macOS shows a Gatekeeper warning and Windows shows a SmartScreen
prompt on first launch; both are documented for users. Distribution relies on
GitHub Releases, so assets are covered by GitHub's transport security, but the
binaries themselves carry no publisher signature.

### 3. Deferred work

- Code signing and Apple notarization.
- Auto-updates, signing keys, and a stable update channel.
- RPM packages, checksum sidecars beyond GitHub, and automated release notes.
- Reproducible-build verification.

## Alternatives considered

- **Build only locally.** No installable artifact for users and no reproducible
  release path; rejected.
- **Sign and notarize from the start.** Requires Windows certificates, Apple
  Developer credentials, and secret management. Deferred until distribution
  requires it.
- **Ship an auto-updater now.** Adds key management and an update channel before
  there is a signed release to update from; deferred.
- **Single platform.** The roadmap targets Windows, macOS, and Linux; a
  three-platform matrix is the minimum.
- **Keep `"targets": "all"` for CI.** The project config stays permissive for
  local builds, but the workflow selects explicit bundles per platform to avoid
  missing-tool failures for formats a runner does not support.

## Consequences

- A tag produces draft installers for all three platforms; publishing is a
  manual, reviewed step.
- Unsigned builds shift friction to users; the README and desktop guide explain
  the warnings.
- Release builds are slow, so they run only on tags and manual dispatch, not on
  pull requests. The existing desktop CI still builds and type-checks the shell.
- Signing, notarization, and auto-updates remain follow-up work.
