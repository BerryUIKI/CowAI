# Development guide

## Setup

Use current stable Rust, Cargo, rustfmt, Clippy, and native compiler/linker tools. The workspace uses edition 2021 and does not declare an MSRV. Clone the `dev` branch; see [Contributing](../CONTRIBUTING.md) and [Collaboration](COLLABORATION.md) for the PR workflow.

```sh
rustup component add rustfmt clippy
cargo build --workspace --locked
cargo run -p cow-cli -- --help
cargo run -p cow-cli -- init --db cowai.db
cargo run -p cow-cli -- run --task "Plan a calculator" --dry-run --db cowai.db
```

The mock provider needs no model service or API credentials. It produces mock deliverables, but initialization and run history still write to SQLite. Initial builds need registry access unless dependencies are cached.

## Code map

- `cow-core`: domain models, assignments, approvals, risk types, tools, and events.
- `cow-bus`: broadcast event delivery and bounded in-memory history.
- `cow-store`: SQLite migrations and repositories.
- `cow-service`: bootstrap, channels, templates, assignments, approvals, and persisted crew execution.
- `cow-client`: transport-neutral data/command interface and `LocalClient`; opening it initializes storage and reconciles interrupted state.
- `cow-agent`: sequential crews, runtime loop, and an optional tool gate.
- `cow-tools`: tool registry, subprocess/file tools, and static risk classification.
- `cow-llm`: OpenAI-compatible transport and mock provider.
- `cow-cli`: `init`, `run`, and `tools`; `run` executes through `LocalClient::run_crew`.
- `desktop`: React/TypeScript frontend and standalone Tauri backend, outside the root Cargo workspace. The frontend calls Tauri commands wrapping the client boundary, never SQLite directly.

Review [architecture](ARCHITECTURE.md), [ADRs](ADR/README.md), and [template format](TEMPLATE_FORMAT.md) before changing boundaries. Designs are agreed for Phase 1; discuss changes and new scope before implementation. Desktop execution, remote mode, packaging, and other roadmap items remain unfinished; consult [CLAUDE.md](../CLAUDE.md) for current gaps.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
```

Use focused tests such as `cargo test -p cow-service` while iterating; run workspace gates for code PRs. Add regression coverage for changed behavior, including persistence, approval rejection/timeouts, and reconciliation where relevant. Keep tests deterministic and free of live API dependencies and secrets. Documentation-only changes should check links, examples, status claims, and SVG rendering.

Desktop changes require separate checks; root workspace commands do not cover Tauri:

```sh
cd desktop
npm ci
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

Use `npm run tauri dev` for the native shell. See [desktop prerequisites](../desktop/README.md); a browser-only Vite preview does not supply the native client backend.

## Runtime configuration

`init` and `run` accept `--db`, falling back to `COWAI_DB`, then `cowai.db` in the current directory. Desktop uses `app_data_dir()/cowai.db`; its state is separate by default. Protect database files and backups as potentially sensitive local data.

`run` accepts `--model` (default `gpt-4o`), `--base-url` / `OPENAI_BASE_URL`, `--api-key` / `OPENAI_API_KEY`, `--dry-run`, and `--approval` (default `ask`). With neither key nor URL it uses the mock; `--dry-run` forces the mock. A key alone selects the OpenAI endpoint. The HTTP client appends `/chat/completions`. Config files and automatic `.env` loading are not implemented. Streaming, retries, and rate limiting remain gaps.

Approval policies are `ask`, `safe-auto`, `plan-only`, and `trusted`. Headless callers should select an appropriate policy; `ask` prompts on stdin and EOF rejects. Review [ADR 0003](ADR/0003-risk-classification-and-approval.md) and [ADR 0008](ADR/0008-cli-and-local-execution.md). Approval is not OS isolation; see [Security](../SECURITY.md).

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Unexpected mock output | Remove `--dry-run` and supply a key or endpoint |
| HTTP/authentication errors | Verify endpoint prefix, credential, available model, and tool-call compatibility |
| Waiting for approval | Inspect stdin or pending approval state and the chosen policy |
| Desktop and CLI show different data | Check their database locations; they are separate by default |
| Desktop task does not start | The shell supports creation and inspection; execution launch is follow-up work |
| Tool command syntax fails | Agent commands use `cmd /C` on Windows and `sh -c` elsewhere |
| Dependencies/linker unavailable | Check cached registry dependencies and native toolchain prerequisites |

`--verbose` enables debug logs. Redact credentials, prompts, file content, paths, and tool arguments before sharing output. `tools` prints schemas and executes a local `echo` smoke test.
