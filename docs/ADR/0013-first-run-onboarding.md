# ADR 0013: First-Run Onboarding

- **Status**: Accepted
- **Date**: 2026-11-05
- **Related**: `docs/ADR/0009-desktop-ipc-and-state.md`, `docs/ADR/0012-local-model-configuration.md`, `docs/ROADMAP.md` milestone 1.7

## Context

Local bootstrap silently creates the local owner and a "Default Workspace", then
the desktop opens on an empty channel list with no agents and no model provider.
Milestone 1.7 requires first-run onboarding, and the Phase 1 exit criteria
require a new user to complete it offline. The pieces needed already exist: the
provider list (ADR-0012), built-in templates, and the `instantiate_agents`
command. Onboarding should connect them without blocking the app or requiring a
network connection.

## Decision

### 1. Trigger and state

- The desktop reads onboarding state through `get_onboarding_state` during
  startup and shows the guide while `onboarding_completed` is false.
- The flag lives in the same local `config.json` as model providers (ADR-0012).
  A missing file starts onboarding; the flag is local runtime configuration and
  is never exported in `.cowpkg` packages.
- Files written before this field existed are treated as completed, so an upgrade
  does not show the guide to an existing user. Serde uses a field default of
  `true`, while `AppConfig::default()` — used when no file exists — sets it to
  `false`.

### 2. Flow

Three steps, shown as a full-screen overlay over the workspace:

1. **Welcome** — explains that the workspace is local, that tool calls wait for
   approval, and that models can be switched at any time.
2. **Model** — embeds the same provider management component used by Settings
   (`ModelProviderSettings`). This step is optional; the only network activity is
   an explicit connection test.
3. **Team** — lists built-in templates and instantiates agents through the
   existing `instantiate_agents` command. This step is optional.

"Skipping" is explicit: a header action completes onboarding from any step, and
"Finish setup" completes it on the last step. `complete_onboarding` sets the flag,
persists the configuration, and the overlay closes. The main workspace remains
fully available after onboarding; Settings remains the place to add providers,
agents, or templates later.

### 3. Boundaries

- The overlay is frontend-only and calls the existing Tauri commands; no new
  data model or storage engine is introduced.
- Onboarding performs no network requests by itself. Only a provider connection
  test initiated by the user talks to an endpoint.
- Completion is idempotent: calling `complete_onboarding` again leaves the flag
  set.

## Alternatives considered

- **Blocking wizard before the workspace opens.** Rejected: an unreadable
  configuration or a user who just wants to look around must not be trapped.
- **Deriving first-run from "no providers and no agents".** Rejected: it is not a
  reliable signal, and deleting providers would re-open the guide.
- **Storing the flag in SQLite.** Rejected for the same reasons as ADR-0012: it
  is local runtime configuration, not workspace data.
- **A separate onboarding file.** Unnecessary state for one boolean.

## Consequences

- Fresh installs see the guide once; existing installs with a `config.json` skip
  it.
- Skipping marks setup complete; users finish in Settings.
- Onboarding reuses the Settings provider component, so provider behavior stays
  in one place.
- Installer builds remain (milestone 1.7), and re-running onboarding or naming
  the workspace are follow-up work.
