# CowAI Functional and System Architecture (Draft)

> **Status**: Draft v0.1 (to be confirmed section by section)
> **Date**: 2026-11-03
> **Related**: `docs/PRD.md`, `docs/ADR/0001-runtime-and-deployment.md`

---

## 1. Architecture Principles

1. **One core, multiple runtime forms**: desktop and server share the same Rust domain model and business logic.
2. **Local-first, multi-user-ready**: the platform works without a server by default, yet accounts, workspaces, and permissions exist from v1.
3. **API-first**: the desktop app, CLI, and future integrations all access capabilities through `cow-client` or HTTP + WebSocket.
4. **Event-driven**: messages, task state, tool calls, and approvals are broadcast over the event bus for real-time UI and auditing.
5. **Configuration over hard-coding**: roles, tools, templates, and approval policies are configurable; software engineering is not baked in.
6. **Secure by default**: risky operations require approval; the sandbox restricts working directories, timeouts, and output size.
7. **Incremental delivery**: every phase can be released, tested, and rolled back independently.

---

## 2. Runtime and Deployment

```
+---------------------------- Desktop (Tauri) -----------------------------+
|  Web frontend (channels / tasks / agents / approvals / settings)        |
|        | Tauri IPC / HTTP                                               |
|  cow-client --+-- Local mode: in-process cow-* core + SQLite            |
|               +-- Team mode: HTTP/WebSocket -> cow-server               |
+-------------------------------------------------------------------------+

+---------------------------- Self-hosted server --------------------------+
|  cow-server (HTTP + WebSocket API, auth, workspaces, permissions)       |
|        |                                                                |
|  cow-agent / cow-tools / cow-llm / cow-bus / cow-store                  |
|        |                                                                |
|  SQLite (default) or PostgreSQL (P1)                                    |
+-------------------------------------------------------------------------+

CLI (cowai) -- cow-client -- local mode or remote server
```

- The desktop app runs in local mode by default and needs no server; switching to team mode only changes the `cow-client` implementation and connection settings.
- The server supports multiple users from v1; local mode auto-creates a single-user workspace, but the data model matches the server.

### 2.1 Implementation Order

1. **Local/offline first**: desktop app + core + SQLite + templates + approvals + tool sandbox.
2. **Self-hosted team mode**: `cow-server`, multi-user accounts, invitations, API tokens, remote client.
3. **Extensions**: Docker sandbox, IM/Webhooks, long-term memory, PostgreSQL, template marketplace.

---

## 3. Functional Domains

### 3.1 Identity and Workspace

- Entities: `User`, `Workspace`, `Membership`, `Role` (Owner / Admin / Member).
- Local mode: first launch auto-creates a local Owner and a default workspace.
- Team mode: sign-up/sign-in, workspace invitations, member role management, API tokens.
- v1 authentication: local accounts (Argon2id), opaque server session tokens, API tokens; OIDC/SSO is P1.

### 3.2 Collaboration

- Entities: `Channel`, `Thread`, `Message`, `Mention`, `TaskCard`, `ApprovalCard`.
- Humans and agents are both channel members; messages are persisted and shown on a timeline.
- `@Agent` mentions or task cards trigger tasks; agent reasoning, tool calls, approvals, and deliverables are written back to the channel.
- Channels are the primary collaboration surface but do not restrict usage: users can create channels and threads freely.
- `cow-service` provides the local bootstrap (idempotent owner and default workspace) and channel/message services that persist writes before publishing `ChannelCreated` and `MessagePosted` events.

### 3.3 Agents and Templates

- `AgentProfile`: name, role, goal, backstory, model, tool set, iteration limit, delegation flag.
- Built-in template library: role templates and team templates (e.g., development, research, writing teams) selectable at creation time.
- Templates can be customized, imported, and exported; they are a starting point, not a fixed scenario.
- Template format is JSON (`schema_version` / `kind` / `agents` / `tasks`); built-in templates are compiled into the binary, user templates are stored in SQLite, and exports use `.cowtemplate`.
- Templates only reference tool names and model configuration; they never embed executable code. v1 has no online template marketplace.
- `cow-service::TemplateService` loads the compiled-in templates, validates documents (schema version, references, cycles), persists user templates, and imports/exports `.cowtemplate` files. Template format details live in `docs/TEMPLATE_FORMAT.md`.

### 3.4 Tasks and Orchestration

- Task lifecycle: `Pending -> Assigned -> Running -> WaitingApproval -> Completed / Failed / Cancelled`.
- v1 scope (confirmed): single-agent tasks plus sequential crew (`ProcessType::Sequential`).
- Later: hierarchical orchestration (manager agent delegation) and DAG workflows.
- Tasks support prerequisite dependencies (`context_tasks`); prior deliverables are injected into downstream context.

### 3.5 Tools and Sandbox

- `Tool` trait and `ToolRegistry`: name, description, parameter schema, async execution.
- Built-in tools: code execution, file read, file write; later: network access, retrieval, browser, and more.
- `SandboxBackend`: local subprocess in v1; Docker in P1; WASM evaluated in P2.
- Resource limits: timeout, output truncation, working-directory restriction, environment-variable filtering.

### 3.6 Model Gateway

- `LlmProvider` abstraction; v1 provides an OpenAI-compatible client (cloud APIs, DeepSeek, Ollama, LocalAI) and a Mock provider.
- Model configuration is workspace- or agent-scoped, with a connection test.
- P1 adds streaming, retries/backoff, rate limiting, and token/cost accounting.

### 3.7 Memory

- v1 (confirmed): session context plus channel/task history as the short-term memory source.
- P1: project-level long-term memory and knowledge base (retrieval-augmented), with explicit storage and privacy boundaries.
- Agent memory must never leak across workspaces.

### 3.8 Persistence and Audit

- `CowStore` abstraction; v1 implements SQLite (WAL); PostgreSQL is deferred to the server phase (P1).
- Stored entities: users, workspaces, members, channels, messages, agents, templates, tasks, runs, tool calls, approvals, and audit events.
- Audit events record who approved or rejected what, and which tools an agent executed.

### 3.9 Access Layer

- **Desktop UI**: Tauri web frontend for channels, tasks, agents, approvals, and settings.
- **HTTP/WebSocket API**: `cow-server` serves desktop team mode, the CLI, and a future browser entry point.
- **CLI**: `cowai` for initialization, diagnostics, headless runs, and debugging.
- **Webhooks / IM** (P1): Mattermost, Slack, Feishu, and others.

---

## 4. Core Flows

### 4.1 From Channel to Deliverable

1. A user `@Agent`s, creates a task card, or starts a task from a team template in a channel.
2. The orchestrator creates the task and assigns agents (sequential in v1).
3. An agent calls the LLM and produces text or tool-call requests.
4. The approval policy decides: auto-approve or create an approval card.
5. Tools execute in the sandbox; results are written back to the channel and task context.
6. The task completes; deliverables, run records, and audit events are persisted.

### 4.2 Approval Policy (Multiple Modes)

Inspired by Claude Code's permission modes:

| Mode | Behavior |
| --- | --- |
| **Ask (default)** | Every sensitive tool call requires approval; approve once, for this session, or always |
| **SafeAuto** | Read-only/safe operations auto-approve; writes/executions still require approval |
| **PlanOnly** | Agents may plan and investigate read-only, but cannot write |
| **Trusted** | Auto-approve within a trusted workspace (admin-enabled), with auditing retained |

- Rule priority: workspace policy > agent policy > global default.
- Fine-grained rules: per tool, command prefix, path allowlist/denylist.
- Risk levels: `ReadOnly` / `Low` / `Medium` / `High` / `Critical`.
- Decision order: static rules first (tool, command pattern, path, network, destructive operations); an optional LLM classifier may handle unmatched operations and can only raise risk, never lower the static floor; undecidable operations default to `High`.
- Decision record: rule ID / classifier / human, together with the approver, written to the audit log.

### 4.3 Local Mode vs. Team Mode

- **Local mode**: single user, in-process core, SQLite; no network or server required.
- **Team mode**: multi-user, remote `cow-server`, shared workspaces; the desktop app only renders and interacts.
- Both modes share the domain model and UI; differences are encapsulated in `cow-client` and the auth layer.
- **Data exchange (v1)**: the two modes keep independent data and migrate via import/export (workspace packages, template packages, task archives); no bidirectional sync.

---

## 5. Logical Data Model (Draft)

| Entity | Key Fields |
| --- | --- |
| `User` | id, name, email, password_hash, created_at |
| `Workspace` | id, name, created_at |
| `Membership` | user_id, workspace_id, role |
| `Session` | id, user_id, token_hash, expires_at, created_at |
| `ApiToken` | id, user_id, workspace_id, token_hash, scopes, revoked_at |
| `Invite` | id, workspace_id, token_hash, role, expires_at |
| `Channel` | id, workspace_id, name, topic, created_at |
| `Message` | id, channel_id, sender, content, thread_id, created_at |
| `AgentProfile` | id, workspace_id, name, role, goal, backstory, model, tools, limits |
| `Template` | id, kind (agent/team), name, payload |
| `Task` | id, workspace_id, channel_id, description, expected_output, assignee, status, context_tasks |
| `Run` | id, task_id, agent_id, status, started_at, finished_at |
| `ToolCall` | id, run_id, tool_name, arguments, result, approved_by |
| `Approval` | id, task_id, tool_call_id, status, decided_by, decided_at |
| `AuditEvent` | id, workspace_id, actor, action, target, created_at |

> Final field definitions will follow the `cow-store` migrations.

---

## 6. Crate Map and Plan

| Crate | Responsibility | Status |
| --- | --- | --- |
| `cow-core` | Domain model, errors, events, `Tool` trait | Existing |
| `cow-bus` | Event bus and subscriptions | Existing |
| `cow-tools` | Tool registry and sandbox backends | Existing (subprocess) |
| `cow-llm` | Model providers and types | Existing (OpenAI-compatible + Mock) |
| `cow-agent` | Agent loop and crew orchestration | Existing (sequential) |
| `cow-cli` | `cowai` CLI | Existing |
| `cow-store` | Persistence abstraction and SQLite/Postgres implementations | Existing (SQLite) |
| `cow-service` | Local bootstrap, channel/message services, template management, and event wiring | Existing |
| `cow-server` | HTTP/WebSocket API, auth, multi-user | Planned |
| `cow-client` | Unified local/remote client abstraction | Planned |
| `cow-package` | `.cowpkg` / `.cowtemplate` import/export | Planned |
| `cow-desktop` | Tauri shell and frontend integration | Planned |
| Frontend app | React + TypeScript (channels, tasks, approvals, agent management UI) | Planned |

---

## 7. Permissions and Approval Model

- **Workspace roles**: Owner (full control), Admin (members/agents/policy management), Member (channels and tasks).
- **Agent permissions**: allowed tool set, allowed models, delegation flag, maximum iterations.
- **Approval decisions**: record approver, time, mode, and rule basis in the audit log.
- **Sensitive operations**: file writes/deletes, command execution, network access, cross-workspace access.

---

## 8. Non-Functional Requirements

- **Security**: deny risky operations by default; encrypt credentials at rest; least privilege; auditable actions.
- **Performance**: local-mode cold start < 2s; real-time channel messages; low-latency task events.
- **Reliability**: recoverable task crashes; transactional SQLite writes; horizontally scalable server (P1).
- **Extensibility**: tools, models, templates, and access methods all extend through interfaces.
- **Observability**: structured logs, task/run tracing, cost accounting (P1).
- **Internationalization**: UI strings localizable (at least English and Chinese).

---

## 9. Confirmed Decisions and Open Questions

### 9.1 Confirmed (2026-11-03)

1. **Orchestration scope**: v1 = single agent + sequential crew; hierarchical manager and DAG are P1.
2. **Memory scope**: v1 = session + channel/task history; project-level long-term memory is P1.
3. **Frontend stack**: React + TypeScript (Tauri WebView).
4. **Local/team modes**: independent data, import/export only, no bidirectional sync.
5. **Templates**: JSON, built-in templates compiled into the binary, user templates import/export as `.cowtemplate`; no online marketplace in v1.
6. **Risk determination**: static-rule-first (ReadOnly–Critical), optional LLM classifier can only raise risk; unknown operations default to High.
7. **Import/export**: `.cowpkg` (ZIP + manifest + JSON/JSONL); no secrets, password hashes, or tokens.
8. **Authentication**: local accounts + Argon2id + opaque session tokens + API tokens; OIDC/SSO is P1.
9. **PostgreSQL**: deferred to the server phase; local/offline functionality comes first.

### 9.2 Open Questions

1. Data-at-rest encryption design.
2. Online template marketplace and template signing (P2).
3. TLS termination and reverse-proxy deployment details.
4. Invitation and notification channels (email / IM).

---

## 10. Design References

External projects and the patterns CowAI adopts, defers, or rejects are recorded in `docs/REFERENCES.md` (CrewAI, Mattermost, and OpenMausBot). The most immediate candidate is a simplified assignment runtime for Phase 1.4, described there.
