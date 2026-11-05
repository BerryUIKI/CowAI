# ADR 0012: Desktop Model Provider Configuration

- **Status**: Accepted
- **Date**: 2026-11-05
- **Related**: `docs/ADR/0007-client-api.md`, `docs/ADR/0009-desktop-ipc-and-state.md`, `docs/ARCHITECTURE.md` §3.9
- **Supersedes**: The provider-selection paragraph of ADR-0009 §6 (desktop execution)
- **Extended by**: ADR-0016 (provider protocols)

## Context

Until this change the desktop backend built one LLM provider at startup from the
`OPENAI_API_KEY` and `OPENAI_BASE_URL` environment variables, hardcoded the model
to `gpt-4o`, and silently fell back to the mock provider when neither variable was
set. An installed desktop app therefore could not be pointed at a real model
without editing the environment, and could not switch between endpoints at all.

Users commonly keep several OpenAI-compatible endpoints — an official API, a
reasoning-model service, a local Ollama server, or a relay — and expect to switch
between them without restarting the app or editing files. The provider-list and one-click
switch model used by [CC Switch](https://github.com/farion1231/cc-switch) is a
useful external UX reference; no code is shared with it.

## Decision

### 1. Storage

Model providers live in a single `config.json` in the app data directory
(`app_data_dir()/config.json`), next to `cowbot.db`:

```json
{
  "version": 1,
  "active_provider_id": "provider-1762300000000",
  "providers": [
    {
      "id": "provider-1762300000000",
      "name": "OpenAI",
      "base_url": "https://api.openai.com/v1",
      "model": "gpt-4o",
      "api_key": "sk-..."
    }
  ]
}
```

- A missing file is an empty configuration. A malformed file does not block
  launch — the runtime falls back to the mock model — and the settings view
  reports the parse error with the file path so it can be corrected or removed.
- Unknown fields are ignored so older builds can read newer files.
- Writes are atomic (temporary file, then rename). The file mode is `0600` on
  Unix; on Windows the app-data directory is per-user, and the file inherits its
  access control.
- This file is local runtime configuration, not workspace data. It is never
  included in `.cowpkg` exports or in the SQLite database.

### 2. Selection and runtime

Exactly one provider is active. When no provider is active — including a fresh
install — the desktop uses the offline mock provider, preserving the previous
no-credentials behavior. Saving the first provider activates it automatically.
Switching providers rebuilds the LLM client inside `AppState` immediately; a
restart is not required and in-flight runs keep their snapshot.

Desktop provider selection is configuration-only: `OPENAI_API_KEY` and
`OPENAI_BASE_URL` no longer affect the desktop. The CLI keeps its flags and
environment variables unchanged.

### 3. Command boundary

Five Tauri commands own the provider list:

- `get_model_settings` — providers (without keys), active id, config path.
- `save_model_provider` — create or update; the first provider becomes active.
- `delete_model_provider` — remove; deleting the active provider selects the mock.
- `activate_model_provider` — select a provider, or `null` for the mock.
- `test_model_provider` — bounded connectivity check against drafted values.

API keys are write-only across IPC: the frontend receives `has_api_key`, never
the secret. A blank key on update keeps the stored one; `clear_api_key` removes
it. Keys are trimmed, never logged, and never placed in error messages.

### 4. Connection test

`test_model_provider` issues one minimal chat completion against the drafted base
URL, model, and key, bounded by a 30-second timeout. A blank key falls back to
the key stored for the provider being edited, so a saved provider can be tested
without retyping its secret.

## Alternatives considered

- **Single provider form.** Simpler, but switching between an official endpoint,
  a local Ollama server, and a relay would require retyping keys and URLs.
- **Store providers in SQLite.** Would make provider configuration part of the
  workspace and of backup/export paths, and would couple a desktop-only runtime
  concern to the domain schema. A small file keeps it local and out of packages.
- **OS keychain for secrets.** Stronger protection, but adds platform-specific
  dependencies, migration, and packaging work. Deferred; `config.json` is the
  documented single place to migrate from later.
- **Keep environment-only configuration.** Cannot be configured from the
  installed app and gives no way to keep more than one endpoint.

## Consequences

- Provider switching works offline and without a restart; the connection test
  talks only to the configured endpoint.
- API keys are stored in plaintext in a local file. This is documented in the UI
  and the development guide; a keychain migration is follow-up work.
- The settings view grows a small provider management section; the first-run
  onboarding in milestone 1.7 can reuse it.
- Phase 2 remote mode will need server-side provider configuration; this record
  covers local mode only.
