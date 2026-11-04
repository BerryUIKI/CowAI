# CowAI Roadmap

> **Status**: Draft v0.1
> **Date**: 2026-11-03
> **Related**: `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/ADR/`

This roadmap sequences the work from the current foundation to a self-hosted, channel-native multi-agent platform. It follows the agreed order: **local/offline first, self-hosted team mode second, ecosystem extensions third.**

---

## 1. Principles

1. **Local/offline first**: the desktop app must be fully usable without a server before multi-user server work begins.
2. **Multi-user-ready from day one**: the data model and APIs include users, workspaces, memberships, and roles even in local mode.
3. **Small, shippable increments**: each milestone produces a testable, demonstrable outcome.
4. **Documentation travels with code**: every milestone updates the relevant docs and records decisions as ADRs.
5. **Quality gates are non-negotiable**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and workspace tests must pass.
6. **English-only artifacts**: code, comments, docs, commit messages, and branch names are English; conversation may be Chinese.

---

## 2. Phase Overview

| Phase | Theme | Primary Outcome |
| --- | --- | --- |
| **Phase 0** | Foundation | Workspace, core crates, product/architecture docs, initial ADRs |
| **Phase 1** | Local/Offline MVP | Installable desktop app with channels, agents, tasks, approvals, sandbox, and SQLite |
| **Phase 2** | Self-Hosted Team Mode | Multi-user server, shared workspaces, remote desktop client, Docker deployment |
| **Phase 3** | Extensibility and Ecosystem | Docker sandbox, memory, integrations, plugins/MCP, marketplace, enterprise features |

No calendar dates are committed; a phase ships when its exit criteria are met.

---

## 3. Phase 0 — Foundation (Completed)

**Goal**: establish the repository, crate boundaries, and shared product/architecture understanding.

Delivered:

- Cargo workspace with `cow-core`, `cow-bus`, `cow-tools`, `cow-llm`, `cow-agent`, and `cow-cli`.
- Sequential agent crew with a tool-calling loop and a working subprocess code sandbox.
- OpenAI-compatible and mock LLM providers.
- `cowai` CLI with `run` and `tools` commands.
- PRD, functional/system architecture, and ADRs 0001–0005.
- Passing tests, clippy, and formatting checks.

---

## 4. Phase 1 — Local/Offline MVP (Desktop)

**Goal**: a single user can install CowAI Desktop, create a workspace, assemble an agent team from templates, run a sequential task in a channel, approve risky tool calls, inspect deliverables, and export/import the workspace — all without a server.

### 4.1 Milestones

| ID | Milestone | Key Deliverables | Depends On |
| --- | --- | --- | --- |
| 1.1 | Persistence foundation (`cow-store`) | SQLite schema and migrations; repositories for users, workspaces, channels, messages, agents, templates, tasks, runs, tool calls, approvals, audit; WAL and transactional writes; store tests | Phase 0 |
| 1.2 | Local identity and workspace | Local owner bootstrap; default workspace; channel/thread/message persistence; event bus wired to storage | 1.1 |
| 1.3 | Agent templates and configuration | Template JSON schema; built-in templates compiled in; template CRUD and validation; `.cowtemplate` import/export | 1.1 |
| 1.4 | Task orchestration | Assignment runtime (root/parent/depth tree, lifecycle, event log, budgets, stall guard, restart reconciliation); sequential crew on assignments; context chaining; cancellation and failure handling | 1.2, 1.3 |
| 1.5 | Tools, sandbox, and approvals | Static risk classifier; Ask/SafeAuto/PlanOnly/Trusted modes; approval cards and audit records; code/file tools with limits | 1.2, 1.4 |
| 1.6 | Desktop shell and UI | Tauri shell; React + TypeScript UI for channels, tasks, agents, approvals, templates, and settings; local `cow-client` | 1.2–1.5 |
| 1.7 | Packaging and data exchange | Windows/macOS/Linux builds; `.cowpkg` workspace export/import; first-run onboarding; user documentation | 1.6 |

> **Progress**: milestones 1.1–1.5 are implemented — `cow-store` provides embedded SQLite migrations and repositories, `cow-service` provides idempotent local bootstrap, channel/message persistence wired to the event bus, the assignment runtime with event-sourced lifecycle/restart reconciliation, a sequential crew runner, the static risk classifier with approval gates and audit records, and validated built-in templates with `.cowtemplate` import/export.
>
> **Remaining within 1.1–1.5**: `AssignmentService` enforces the request, execution, and wake budgets, but the sequential crew runner only exercises the execution budget and there is no wake/re-entry scheduler for delegated work; `cow-client` local mode exists and powers the CLI, while the desktop UI and remote mode remain. These are tracked as the remaining tasks of 1.6.

### 4.2 Phase 1 Exit Criteria

- A new user can install the app and complete first-run onboarding offline.
- The user can create a workspace and channel, and add agents from built-in templates.
- A task can be submitted in a channel and executed by a sequential crew with visible progress.
- Risky operations trigger approvals according to the selected mode, and decisions appear in the audit log.
- Code execution and file operations respect sandbox limits (timeout, output truncation, working directory).
- Task deliverables and run history persist across restarts.
- Assignments expose an append-only event log, enforce per-root budgets, and reconcile interrupted runs on startup.
- A workspace can be exported to `.cowpkg` and re-imported into a fresh install.
- All quality gates pass and the main flow is covered by integration tests.

### 4.3 Explicitly Out of Scope for Phase 1

- Multi-user server, invitations, and remote sessions.
- OIDC/SSO, PostgreSQL, Docker sandbox, IM/Webhook integrations.
- Project-level long-term memory, streaming responses, cost accounting.
- Template marketplace and plugin/MCP ecosystem.

---

## 5. Phase 2 — Self-Hosted Team Mode

**Goal**: a small team can deploy CowAI Server, invite members, share workspaces and channels, and collaborate with agents under role-based permissions and audit.

### 5.1 Milestones

| ID | Milestone | Key Deliverables | Depends On |
| --- | --- | --- | --- |
| 2.1 | Server and API foundation (`cow-server`) | HTTP + WebSocket API; Argon2id accounts; opaque sessions; API tokens; error model; login rate limiting | Phase 1 |
| 2.2 | Multi-user workspaces | Workspace creation; invitations; Owner/Admin/Member roles; membership enforcement; audit integration | 2.1 |
| 2.3 | Remote client mode | `cow-client` remote implementation; desktop sign-in and server connection; reconnection; local/team mode switch | 2.1, 2.2 |
| 2.4 | Deployment and operations | Single binary and Docker image; Compose example; TLS via reverse proxy; health checks; backup/restore; configuration reference | 2.1 |
| 2.5 | Migration and compatibility | `.cowpkg` import into the server; server-side schema migrations; version compatibility policy | 2.2, 2.4 |

### 5.2 Phase 2 Exit Criteria

- A team can deploy the server with Docker or a single binary and connect desktop clients over HTTPS.
- Multiple users can sign in, join a workspace, and collaborate in shared channels.
- Role checks prevent unauthorized workspace, agent, and policy changes.
- Approvals and tool executions record the acting user and appear in the audit log.
- Server restart preserves workspaces, messages, tasks, and approvals.
- Backup and restore procedures are documented and tested.
- Workspace export from local mode imports cleanly into the server.

### 5.3 Explicitly Out of Scope for Phase 2

- PostgreSQL, OIDC/SSO, browser access, horizontal scaling.
- Docker sandbox, long-term memory, marketplace, and plugin ecosystem.

---

## 6. Phase 3 — Extensibility and Ecosystem

**Goal**: make CowAI adaptable to more environments and workflows without compromising the secure, local-first core.

### 6.1 Workstreams

| ID | Workstream | Key Deliverables |
| --- | --- | --- |
| 3.1 | Sandbox and execution | Docker sandbox backend; resource quotas; network policy; additional built-in tools |
| 3.2 | Memory and knowledge | Project-level long-term memory; retrieval-augmented context; privacy and retention controls |
| 3.3 | Integrations | Webhooks; Mattermost, Slack, and Feishu adapters; optional browser access |
| 3.4 | Ecosystem | Plugin mechanism; MCP compatibility; template marketplace with signing; SDK for custom tools |
| 3.5 | Enterprise readiness | OIDC/SSO; PostgreSQL backend; data-at-rest encryption; audit export; multi-workspace/organization management |
| 3.6 | Scale and polish | Streaming responses; retries/backoff; rate limiting; token and cost accounting; performance and UX improvements |

Each workstream can ship independently; prioritization will be decided with users after Phase 2.

---

## 7. Cross-Cutting Workstreams

- **Quality**: unit and integration tests per milestone; `fmt`, `clippy -D warnings`, and workspace tests in CI.
- **Security**: threat modeling for tool execution and authentication; dependency auditing; sandbox escape review.
- **Documentation**: PRD, architecture, ADRs, and roadmap updated with every milestone; docs remain English-only.
- **Observability**: structured logging and task/run tracing from Phase 1; metrics and cost accounting later.
- **Internationalization**: UI strings externalized for at least English and Chinese; repository artifacts remain English.
- **Git workflow**: GitFlow with `main` as the release branch and `dev` as the integration branch.

---

## 8. GitFlow Workflow

- `main`: stable, release-ready history only.
- `dev`: integration branch for completed features; all feature work merges here first.
- `feature/<name>`: branched from `dev`, merged back into `dev` with `--no-ff`.
- `release/<version>`: branched from `dev` when preparing a release, merged into both `main` and `dev`.
- `hotfix/<name>`: branched from `main` for urgent fixes, merged into both `main` and `dev`.
- Commits are small, atomic, and English-written; pull requests target `dev` unless they are release or hotfix PRs.

---

## 9. Dependencies and Sequencing

```
Phase 0 (done)
   |
   v
1.1 Store --> 1.2 Identity/Workspace --> 1.4 Orchestration --> 1.5 Tools/Approvals --> 1.6 Desktop UI --> 1.7 Packaging
                     |                        ^
                     +--> 1.3 Templates ------+
   |
   v
Phase 2: 2.1 Server/API --> 2.2 Multi-user --> 2.3 Remote client
                 |                |
                 +--> 2.4 Deployment
                 +--> 2.5 Migration
   |
   v
Phase 3 workstreams (parallel, prioritized with users)
```

Critical path: `cow-store` → identity/workspace → orchestration → approvals → desktop UI → packaging. Delaying persistence or approvals blocks most later work.

---

## 10. Risks and Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Scope creep in Phase 1 | Delays offline MVP | Strict exit criteria; explicitly deferred list; small atomic PRs |
| Sandbox escape or destructive commands | Security incident | Static risk rules first; approval gates; limited working directory; container backend in Phase 3 |
| Model/provider API changes | Broken agent runs | Provider abstraction; OpenAI-compatible baseline; mock provider for tests |
| Tauri or React ecosystem churn | Packaging and UI delays | Pin versions; isolate the UI behind `cow-client`; keep the core independent |
| Cross-platform packaging issues | Poor first-run experience | Early CI builds for Windows/macOS/Linux; smoke tests per release |
| Multi-user auth complexity | Phase 2 delays | Opaque sessions and simple RBAC first; OIDC deferred |
| Local/team data divergence | User confusion | Explicit import/export; no bidirectional sync in v1; documented migration path |
| Template supply-chain risk | Malicious templates | No embedded code; tool-reference validation; signing deferred but planned |

---

## 11. Definition of Done (Per Milestone)

A milestone is done when:

1. Acceptance criteria are written and met.
2. Unit and integration tests cover the new behavior.
3. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass.
4. Relevant docs and ADRs are updated in English.
5. A demo or reproducible verification path is documented.
6. Changes are merged into `dev` through small, atomic commits and reviewed.

---

## 12. Success Metrics by Phase

### Phase 1

- Install-to-first-task completion under 30 minutes.
- Task completion rate and approval response time.
- Crash-free desktop sessions.
- Successful `.cowpkg` export/import round trips.

### Phase 2

- Self-hosted deployment success rate.
- Number of active workspaces and members per deployment.
- Authentication and authorization failures.
- Audit completeness for approvals and tool calls.

### Phase 3

- Adoption of Docker sandbox and integrations.
- Memory retrieval usefulness and privacy incidents (target: zero).
- Plugin/MCP ecosystem growth.
- Cost and latency efficiency per task.
