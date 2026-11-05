# 0011: CowBot software rename

- **Status**: Accepted and implemented
- **Date**: 2026-11-04
- **Related**: `docs/ADR/0010-cowbot-product-name.md`, `docs/ADR/0004-import-export-package.md`, `docs/ADR/0008-cli-and-local-execution.md`, `docs/ADR/0009-desktop-ipc-and-state.md`

## Context

ADR 0010 adopted CowBot for product prose and documentation assets while deliberately leaving executable identifiers unchanged. The repository has since moved to `BerryUIKI/CowBot`, and the owner asked for the software-side rename to be implemented. The rename covers the workspace crates, the CLI, local storage names, the desktop identity, and application assets.

## Decision

| Surface | Before | After |
| --- | --- | --- |
| Workspace crates | `cow-core`, `cow-bus`, `cow-tools`, `cow-llm`, `cow-agent`, `cow-store`, `cow-client`, `cow-service`, `cow-cli` | `cowbot-core`, `cowbot-bus`, `cowbot-tools`, `cowbot-llm`, `cowbot-agent`, `cowbot-store`, `cowbot-client`, `cowbot-service`, `cowbot-cli` |
| CLI binary | `cowai` | `cowbot` |
| Database environment variable | `COWAI_DB` | `COWBOT_DB` |
| Default CLI database | `cowai.db` | `cowbot.db` |
| Desktop product and window name | CowAI | CowBot |
| Desktop identifier | `com.cowai.desktop` | `com.cowbot.desktop` |
| Tauri event channel | `cowai-event` | `cowbot-event` |
| Desktop package names | `cowai-desktop` | `cowbot-desktop` |
| Brand asset filenames | `cowai-*` | `cowbot-*` |
| Local owner email | `local@cowai.local` | `local@cowbot.local` |

The `.cowtemplate` and `.cowpkg` extensions, and internal type names derived from the `Cow` prefix such as `CowError`, stay unchanged. The planned package manifest `format` identifier in ADR 0004 becomes `cowbot-package`; the extension remains `.cowpkg`.

## Compatibility and migration

- **CLI**: clean break. Only `COWBOT_DB` and `cowbot.db` are recognized. Existing users rename their database (`mv cowai.db cowbot.db`) or pass `--db`. No automatic lookup of the old file happens, so a stale `cowai.db` cannot silently diverge.
- **Desktop**: the identifier change moves the app-data directory. On first launch, the app copies `com.cowai.desktop/cowai.db` and its `-wal`/`-shm` sidecars into `com.cowbot.desktop/cowbot.db` when the target database does not exist. The copy is best-effort and never blocks startup; a failure is logged and a fresh database is created. Existing data is copied, not moved, so a downgrade still finds the old file.
- **Schema**: existing databases need no migration. `crates/cowbot-store/migrations/0001_init.sql` is untouched so recorded checksums remain valid.
- **Install identity**: the bundle identifier changes the OS-level application identity. Existing desktop installs may appear as a separate application and can be uninstalled after the new build preserves their data.
- **History**: earlier ADRs keep the names that were current when they were written. This record is the authoritative mapping for the rename.

## Consequences

- Commands, help output, storage defaults, desktop branding, and source paths consistently use `cowbot`.
- CLI scripts that set `COWAI_DB` must switch to `COWBOT_DB` or `--db`.
- The one-time desktop copy can be removed once pre-rename installs are no longer supported.
