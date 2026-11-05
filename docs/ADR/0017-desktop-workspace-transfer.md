# ADR 0017: Desktop Workspace Transfer

- **Status**: Accepted
- **Date**: 2026-11-05
- **Related**: `docs/ADR/0004-import-export-package.md`, `docs/ADR/0012-local-model-configuration.md`, `docs/ADR/0013-first-run-onboarding.md`

## Context

The `.cowpkg` format from ADR-0004 was only reachable through the CLI. The
desktop supported neither export nor import, which was the remaining code item
of Phase 1 milestones 1.6 and 1.7.

Two constraints shaped the design:

- A package preserves the original channel, message, task, and run ids. Import
  into a database that already contains those rows therefore collides on primary
  keys; the CLI has always imported into a separate database.
- `LocalClient` opens one SQLite database and binds to its workspace. The
  desktop had no way to open a second workspace.

## Decision

### 1. One database per workspace

The default workspace lives in `app_data_dir()/cowbot.db`. Importing a package
creates a new database at `app_data_dir()/workspaces/<uuid>.db`, imports there
(`cowbot_package::import_workspace`), and initializes a client on it. A failed
import removes the new database file, and import never modifies an existing
workspace.

### 2. Registry and active selection

`config.json` gains:

- `active_workspace_database`: the database path in use.
- `workspaces`: `{ database, name }` entries for the switcher.

Both store paths relative to the app data directory. The backend resolves only
plain relative paths and rejects absolute paths, drive prefixes, and parent
components, so a configuration file cannot point outside app data. At startup
the desktop opens the active database (falling back to `cowbot.db` when the
entry is missing or invalid) and refreshes that entry's name from the session.
Entries whose files no longer exist are dropped.

### 3. Switching restarts the process

`switch_workspace` persists the selection and calls `AppHandle::restart()`;
`import_workspace` registers the new database, makes it active, and restarts.
Both refuse to run while a crew has a task in progress. Restarting avoids
swapping a shared client, re-subscribing event streams, and keeping connections
to the previous database open. The UI therefore has no partial-refresh path.

### 4. Export

`export_workspace` exports the active workspace, appends `.cowpkg` when the
chosen name has another extension, and writes through a temporary file so a
failed write does not leave a truncated package.

### 5. Dialogs and UI

The desktop uses `tauri-plugin-dialog` for native open/save dialogs. Settings →
Workspace lists registered workspaces with active and missing states, offers
Switch for other available entries, and exposes Export and Import actions.
Cancelling a dialog is a no-op.

## Alternatives considered

- **Import into the current database.** Rejected: package ids collide with
  existing rows, and rewriting ids would change the package contract.
- **Regenerate ids during import.** Required for same-database import, but it
  changes ADR-0004 semantics and needs reference rewriting across channels,
  messages, tasks, runs, and agents.
- **Swap the client in place.** Avoids the restart, but needs a shared mutable
  client, event-stream re-subscription, and careful handling of in-flight work.
  Deferred; restarting is simpler and safer for 0.1.x.
- **Import only on a fresh install.** Matches the exit criterion but provides no
  way to move between existing workspaces; rejected.

## Consequences

- Switching briefly restarts CowBot. This is documented in the settings UI.
- An imported workspace keeps the exporter's owner identity; it is a separate
  local database, and nothing is merged with existing data.
- Each database is self-contained. Deleting its file removes the workspace from
  the registry on the next start or list.
- Remote workspace operations remain Phase 2 work.
