# CowBot user guide

This guide covers the 0.1.x local desktop app: installing it, connecting a
model, creating a workspace, running a task with an agent crew, handling
approvals, and moving workspaces in and out of the app.

CowBot runs entirely on your machine. Messages, tasks, approvals, and run
history live in local SQLite databases; the only network traffic is the model
request sent to the endpoint you configure.

## Install

Download the build for your platform from the
[releases page](https://github.com/BerryUIKI/CowBot/releases):

| Platform | Files |
| --- | --- |
| Windows 10/11 (x64) | `CowBot_<version>_x64-setup.exe` (NSIS) or `.msi` |
| macOS (Apple Silicon + Intel) | `CowBot_<version>_universal.dmg` |
| Linux (x64) | `CowBot_<version>_amd64.deb` or `.AppImage` |

- **Windows:** run the installer. SmartScreen may warn because builds are
  unsigned; choose **More info → Run anyway**.
- **macOS:** open the DMG and drag CowBot to Applications. On first launch,
  right-click the app and choose **Open**, or allow it in **System Settings →
  Privacy & Security**.
- **Linux:** `sudo apt install ./CowBot_<version>_amd64.deb`, or
  `chmod +x CowBot_<version>_amd64.AppImage && ./CowBot_<version>_amd64.AppImage`.

Releases are unsigned and have no auto-updater; install a newer release the same
way.

## First run

The three-step setup guide covers:

1. **Welcome** — CowBot is local-first; data stays on this machine.
2. **Model** — connect a model provider, or skip and use the offline mock model.
3. **Team** — add a starter team from a built-in template.

You can skip setup at any point. Everything is available later in **Settings**
(the gear in the sidebar footer).

## Connect a model

Open **Settings → Model**. Providers are model endpoints; CowBot ships with an
offline mock model and uses it whenever no provider is active.

1. Choose a protocol:
   - **OpenAI Chat** — the Chat Completions API, used by OpenAI, DeepSeek,
     Ollama, OpenRouter, and most relays.
   - **Anthropic** — the Messages API, used by Claude and Claude
     Code-compatible relays.
   - **Responses** — the OpenAI Responses API, used by Codex-style endpoints.
2. Pick a preset (**OpenAI**, **DeepSeek**, **Ollama**, **Claude**, **OpenAI
   Responses**) or **Custom**. Presets fill the base URL and model; you can edit
   both.
3. Enter the API key if the endpoint needs one. Local endpoints usually do not.
4. **Test connection** sends one small request. The result appears in the
   editor.
5. **Save provider**. The first provider becomes active; use **Use** on another
   card to switch, or **Use offline mock** to run without a model.

The base URL may be a bare host (`https://api.anthropic.com`), include `/v1`, or
be the full endpoint (`https://api.anthropic.com/v1/messages`). CowBot appends
the protocol path when it is missing.

API keys are stored in plain text in the local configuration file. Protect that
file as sensitive data.

### Connection errors

- **401 / unauthorized** — the endpoint wants a key or the key is wrong.
- **Connection refused / error sending request** — nothing is listening at that
  URL, or a proxy is blocking it.
- **404** — the endpoint is reachable but does not serve the selected protocol.
  Check that the protocol matches the provider (for example, a Claude relay
  needs **Anthropic**, not **OpenAI Chat**).

## Workspaces

A workspace contains its channels, agents, tasks, and history. CowBot keeps
**one local database per workspace**. The first workspace is created
automatically; **Settings → Workspace** manages the rest.

- **Create workspace** — enter a name and choose **Create workspace**. CowBot
  restarts into the new, empty workspace.
- **Switch** — selects another registered workspace and restarts CowBot.
- **Export workspace** — writes the active workspace to a `.cowpkg` package.
  Exported packages contain channels, messages, agents, templates, tasks, runs,
  and audit events with SHA-256 checksums; credentials are never included.
- **Import package** — verifies a `.cowpkg` package, creates a new workspace
  database named `<name> (imported)`, registers it, and restarts into it. Import
  never merges with or overwrites existing data.

Creating, importing, and switching restart the app; finish running tasks first,
or CowBot refuses the change. Workspace databases and `config.json` live in the
app data directory:

- Windows: `%APPDATA%\com.cowbot.desktop\`
- macOS: `~/Library/Application Support/com.cowbot.desktop/`
- Linux: `~/.local/share/com.cowbot.desktop/`

The default database is `cowbot.db`; imported or created workspaces live under
`workspaces/`. Deleting a database file removes that workspace from the switcher
on the next start.

## Team

**Settings → Team** lists the built-in templates and the agents in the
workspace. **Add agents** instantiates every agent in a template with its role,
goal, and tool access. The 0.1.x app creates agents from templates; a standalone
agent editor is not part of this release.

## Channels and tasks

- **Channels** — create a channel, post messages, and follow the timeline.
- **Tasks** — create a task with a description, expected output, an agent, an
  optional channel, and an approval policy. Select the task and choose **Run**
  to execute the Planner → Coder → Reviewer crew. The task detail shows
  assignments, assignment events, runs, and tool calls.
- **Cancel** stops a running task.

Progress refreshes from live events; all history persists across restarts.

### Approval policies

- **Ask** — prompt for risky tool calls (default for interactive use).
- **Safe auto** — automatically approve low-risk reads while asking for
  higher-risk actions.
- **Plan only** — deny actions that would modify the machine.
- **Trusted** — approve everything except critical actions.

Approvals appear in **Approvals**. Each decision is recorded in the audit log.
Approval is a guardrail, not OS isolation: the subprocess sandbox runs commands
with your user permissions, so treat model-generated commands as untrusted.

## Data and backups

- Workspace data: the workspace databases under the app data directory.
- Model providers, API keys, the onboarding flag, and the workspace registry:
  `config.json` in the same directory (plain text).
- Backups: use **Export workspace** for a portable package, or copy the database
  files while CowBot is closed.
- Uninstalling the app does not delete your data; remove the app data directory
  manually if you want a clean install.

## Command line

The `cowbot` CLI covers headless and scripted use. It does not read the desktop
`config.json`; it takes flags and environment variables.

```bash
cowbot init --db cowbot.db
cowbot run --task "Analyze and write code" --dry-run
cowbot run --task "Fix the bug" --model gpt-4o --approval ask
cowbot tools
cowbot export --output my-workspace.cowpkg --db cowbot.db
cowbot import --input my-workspace.cowpkg --db other.db
```

- CLI `--db` defaults to `COWBOT_DB`, then `cowbot.db` in the working directory.
- `--dry-run` forces the offline mock model.
- `--model`, `--base-url` (`OPENAI_BASE_URL`), and `--api-key`
  (`OPENAI_API_KEY`) configure the OpenAI Chat Completions endpoint.
- `--approval` accepts `ask`, `safe-auto`, `plan-only`, and `trusted`.

## Troubleshooting

- **Setup guide does not appear** — it runs once. Use **Settings** to configure
  models, agents, and workspaces.
- **A task fails immediately** — check the active provider and connection test;
  with no provider, tasks run against the offline mock model.
- **A workspace cannot be switched** — finish or cancel running tasks first.
- **A workspace shows Missing** — its database file was moved or deleted; remove
  it from the registry by restarting, or restore the file.
- **Package import fails** — the package may be corrupt, from an unsupported
  schema version, or larger than 512 MB.

Report bugs at https://github.com/BerryUIKI/CowBot/issues. For security
reporting, see [SECURITY.md](../SECURITY.md).

## Known limitations in 0.1.x

- Local/offline single-user mode only: no server, remote sessions, or
  multi-user workspaces.
- Sequential Planner → Coder → Reviewer crews; no hierarchical orchestration or
  DAG workflows.
- No background wake/re-entry scheduler for delegated assignments (deferred by
  ADR-0006).
- No streaming responses, retries, rate limiting, or token accounting.
- The subprocess sandbox is not OS-level isolation.
- Workspace deletion and renaming are not available in the UI.
- Builds are unsigned and do not auto-update.
