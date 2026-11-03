<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/cowai-wordmark-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/brand/cowai-wordmark-light.svg">
    <img src="assets/brand/cowai-wordmark-light.svg" alt="CowAI — a cow icon beside serif lettering" width="440">
  </picture>
</p>

<p align="center">
  <strong>Agents that work together. A workspace you control.</strong><br>
  An open-source, local-first multi-agent collaboration platform built in Rust.
</p>

<p align="center">
  <a href="https://www.rust-lang.org/">Built with Rust</a> ·
  <a href="docs/ROADMAP.md">Phase 1 in progress</a> ·
  <a href="#license">MIT OR Apache-2.0</a>
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#what-works-today">Features</a> ·
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="docs/ROADMAP.md">Roadmap</a> ·
  <a href="assets/README.md">Brand assets</a>
</p>

---

CowAI brings role-based agent teams, tasks, and channel-based collaboration into one modular platform. It draws on [CrewAI](https://github.com/crewAIInc/crewAI)'s role-and-task orchestration and [Mattermost](https://github.com/mattermost/mattermost)'s channel-based collaboration model.

The Rust runtime and CLI work today. The local desktop experience is being built incrementally, with self-hosted team mode planned after the local MVP.

## What works today

| Capability | Available now |
| --- | --- |
| Agent crews | Roles, goals, backstories, tool-calling loops, and sequential workflows with context chaining |
| Assignment runtime | Task trees, lifecycle event logs, budgets, stall guards, and restart reconciliation |
| Local persistence | Embedded SQLite migrations and repositories for workspaces, channels, agents, tasks, and audit events |
| Collaboration services | Idempotent local owner/workspace bootstrap; channel and message persistence connected to the event bus |
| Agent and team templates | Built-in JSON templates, validation, template management, and `.cowtemplate` import/export |
| Tools | Subprocess execution and file operations with execution timeouts and output limits |
| Approvals | Static risk classification, Ask/SafeAuto/PlanOnly/Trusted modes, pending approvals, and audit records |
| Model providers | OpenAI-compatible endpoints, including DeepSeek, Ollama, and OpenRouter; mock providers for offline testing |

**In progress:** Phase 1 milestones 1.1–1.5 are implemented. The desktop UI, packaging, and `.cowpkg` import/export are next. See the [roadmap](docs/ROADMAP.md) for scope and acceptance criteria.

## Quick start

Install the [Rust toolchain](https://rustup.rs/), then build from the integration branch:

```bash
git clone --branch dev https://github.com/BerryUIKI/CowAI.git
cd CowAI
cargo build --workspace
```

### Try an agent crew offline

The mock provider runs without an API key:

```bash
cargo run -p cow-cli -- run \
  --task "Plan a REST API for a task tracker" \
  --dry-run
```

### Connect a model

With an OpenAI API key, run a task against a live model:

```bash
export OPENAI_API_KEY="your-api-key"
cargo run -p cow-cli -- run \
  --task "Implement a snake game in Python" \
  --model gpt-4o
```

For a local Ollama endpoint, use an installed model:

```bash
cargo run -p cow-cli -- run \
  --task "Analyze log files" \
  --base-url "http://localhost:11434/v1" \
  --model "llama3"
```

These examples use a POSIX shell. In PowerShell, set the key with `$env:OPENAI_API_KEY = "your-api-key"` and put each `cargo run` command on one line.

### Explore the CLI

```bash
cargo run -p cow-cli -- --help
cargo run -p cow-cli -- tools
```

## Architecture

Eight focused crates keep the domain model, runtime, and application services separate:

| Crate | Responsibility |
| --- | --- |
| [`cow-core`](crates/cow-core) | Domain models, assignments, tools, and events |
| [`cow-bus`](crates/cow-bus) | Tokio broadcast event bus and in-memory event history |
| [`cow-tools`](crates/cow-tools) | Tool registry, subprocess sandbox, and file tools |
| [`cow-llm`](crates/cow-llm) | OpenAI-compatible client and mock providers |
| [`cow-agent`](crates/cow-agent) | Agent execution loops, roles, and crew orchestration |
| [`cow-store`](crates/cow-store) | SQLite migrations, persistence, and repositories |
| [`cow-service`](crates/cow-service) | Local bootstrap, collaboration, templates, and assignment services |
| [`cow-cli`](crates/cow-cli) | The `cowai` command-line interface |

Read the [functional and system architecture](docs/ARCHITECTURE.md) for the planned desktop and server boundaries.

## Documentation

| Guide | Purpose |
| --- | --- |
| [Product requirements](docs/PRD.md) | Users, workflows, and product scope |
| [Architecture](docs/ARCHITECTURE.md) | Functional domains and system design |
| [Roadmap](docs/ROADMAP.md) | Local MVP → self-hosted teams → ecosystem extensions |
| [Architecture decisions](docs/ADR) | Recorded design decisions and tradeoffs |
| [Template format](docs/TEMPLATE_FORMAT.md) | Agent and team template schema |
| [Design references](docs/REFERENCES.md) | External references informing the platform |
| [Brand assets](assets/README.md) | Canonical icon, exports, and asset maintenance |

## Development

Run the workspace quality checks before submitting code changes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Keep contributions small and focused. Branch from `dev` using `feature/<name>` and open pull requests against `dev`. `main` is the release branch; releases are promoted separately. Repository documentation, code, and commit messages use English. See [CLAUDE.md](CLAUDE.md) for development conventions.

## License

The workspace declares `MIT OR Apache-2.0`: your choice of the [MIT License](https://opensource.org/licenses/MIT) or [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). See [Cargo.toml](Cargo.toml) for package metadata.
