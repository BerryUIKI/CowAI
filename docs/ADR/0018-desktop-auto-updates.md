# ADR 0018: Desktop Automatic Updates

- **Status**: Accepted
- **Date**: 2026-11-06
- **Related**: `docs/ADR/0014-desktop-release-builds.md`, `docs/ADR/0017-desktop-workspace-transfer.md`, `docs/ROADMAP.md` milestone 1.7

## Context

CowBot 0.1.1 ships unsigned installers through GitHub Releases with no update
path. ADR-0014 deferred auto-updates, signing keys, and a stable update channel:
users must notice a new release and install it manually, and the app cannot tell
them one exists.

Tauri v2 provides an updater plugin that checks a static JSON manifest,
downloads the matching bundle, verifies a minisign signature, and installs it.
That signature is independent of Windows Authenticode and Apple notarization, so
it also protects the project's unsigned builds.

## Decision

### 1. Update channel

GitHub Releases is the stable update channel. Every `v*` tag build publishes a
`latest.json` asset:

- `windows-x86_64` points at the NSIS `setup.exe`.
- `darwin-aarch64`, `darwin-x86_64`, and `darwin-universal` point at the same
  universal `.app.tar.gz`.
- `linux-x86_64` points at the AppImage.

The desktop checks
`https://github.com/BerryUIKI/CowBot/releases/latest/download/latest.json`,
which resolves to the newest published, non-prerelease release. Draft and
prerelease builds are invisible to the updater, so an update only becomes
available when a maintainer publishes a release.

### 2. Signing keys

The public key is committed in `desktop/src-tauri/tauri.conf.json`. The matching
private key and its password live in the `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` GitHub Actions secrets, never in the
repository, with a backup kept outside it. Losing either one prevents automatic
updates for installed clients; recovering requires a manual release signed with
the old key or a reinstall.

### 3. Release artifacts and manifest

`bundle.createUpdaterArtifacts` makes the bundler create a macOS updater archive
and sign the NSIS, MSI, AppImage, and deb bundles. The release workflow uploads
the signatures as release assets. A final `updater-json` job runs after all
three platform builds and writes `latest.json` from those signatures, so the
manifest always contains every platform. It does not use tauri-action's own
updater JSON upload because its read-modify-write of a single release asset is
racy across the three build jobs.

### 4. Desktop behavior

- CowBot checks for updates once at startup when "Check for updates
  automatically" is enabled. The preference defaults to on, is stored in
  `config.json`, and survives workspace switches.
- The startup check is silent: failures do not interrupt the user.
- An available update appears as a banner in the main pane and in Settings >
  About. Downloading and installing always require a click; installing is
  refused while a crew is running.
- Progress is shown in place. After installation the user restarts CowBot; on
  Windows the installer exits the app automatically.
- Settings > About shows the current version, the stable channel, the last check
  time, and a "View release notes" link to the tagged GitHub release.
- The only new network traffic is one HTTPS request to GitHub per launch when
  automatic checks are enabled. No workspace data is sent, and checks can be
  disabled.

### 5. Supported install formats

- **Windows:** NSIS. An MSI install is updated by installing the new NSIS
  setup.
- **macOS:** the app installed from the DMG. Builds remain unsigned, so macOS
  may ask the user to allow CowBot again after an update.
- **Linux:** AppImage. A deb install is updated by downloading the new package.
- The first release that contains the updater (0.1.2) must be installed
  manually by users of 0.1.1 and earlier, because those versions have no update
  code.

## Alternatives considered

- **Check-only notifications.** Polling the GitHub API for the newest tag and
  linking to the release page would avoid signing keys, but it does not deliver
  in-app updates and still needs per-platform download choices.
- **A custom update server.** More control over rollout and metrics, but it adds
  a hosted service dependency that local-first Phase 1 does not need.
- **Silent download and install.** Replacing the app while a crew runs is
  unsafe; the team chose an explicit download and install step.
- **Operating-system package repositories.** deb, Homebrew, and winget channels
  can complement the updater later, but each has its own review and release
  lag.
- **tauri-action's matrix updater JSON.** Simpler, but its shared release asset
  is racy across jobs; the dedicated final job is deterministic.

## Consequences

- Releases require the signing secrets; the workflow fails early if they are
  absent.
- Maintainers must verify `latest.json` and the signature assets after each tag
  build, and publish the draft release to make the update visible.
- The public key cannot change without a manually distributed release signed by
  the old key, so key backups are an operational requirement.
- Auto-update makes GitHub Releases a runtime dependency of the desktop app;
  offline use is unaffected, and automatic checks can be turned off.
- Code signing and notarization remain separate, deferred work; the updater
  signature does not replace them.
