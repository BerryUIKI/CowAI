# CowBot Design References

> **Status**: Draft v0.1
> **Date**: 2026-11-03
> **Purpose**: Record the external projects whose patterns shape CowBot, and which ideas we adopt, defer, or deliberately reject.

---

## 1. CrewAI

Repository: <https://github.com/crewAIInc/crewAI>

CrewAI is the primary reference for role-based multi-agent orchestration:

- **Agents** carry a role, goal, and backstory plus tools and delegation rules.
- **Tasks** declare an expected output and can depend on earlier tasks; context flows downstream.
- **Crews** group agents and tasks under a process (sequential first, hierarchical later).

CowBot already mirrors this in `AgentProfile`, `Task`, `Crew`, and the built-in software/research team templates. The roadmap keeps sequential crews for v1 and defers hierarchical/DAG orchestration to P1.

---

## 2. Mattermost

Repository: <https://github.com/mattermost/mattermost>

Mattermost is the reference for channel-native collaboration:

- Channels and threads are the primary workspace, not a chat box bolted onto an agent.
- Humans and agents share the same timeline; messages, tasks, and decisions are traceable.
- Multi-user workspaces and permissions are first-class.

CowBot adopts the channel/thread model (`Channel`, `ChannelMessage`, `Sender`) and builds the local runtime on top of it. The desktop app is the first client; team mode adds the self-hosted server later.

---

## 3. OpenMausBot

Repository: <https://github.com/milind-soni/OpenMausBot>

OpenMausBot is an open-source, local-first desktop app that presents a roster of bots as chat contacts. Each bot runs a local Claude/Codex/Grok-class CLI or API model and has its own personality, model, computer, connected apps, memory, and approval level. Key documents: `docs/runtime-contract.md`, `docs/approval-levels.md`, `docs/memory.md`, `docs/bot-box.md`, `docs/team-sharing.md`, `docs/tool-selection.md`, `docs/decision-model.md`.

It validates several CowBot choices: messaging-app UX, channel/department structure, per-agent models, inline approval cards, local-first data, and portable team packages. The patterns below are the most valuable; they are recommendations, not requirements.

### 3.1 Assignment runtime contract

OpenMausBot uses one record per delegated job, whatever started it (person, routine, webhook, another bot). The record carries:

- `rootId`, `parentId`, `depth` — a tree, not a flat queue;
- `origin` — person, routine, or outside;
- `requester` and `assignee` — who asked and which bot/thread answers;
- `mode` — `work`, `consult`, or `notify`;
- `reply` — `resume`, `poll`, or `none`;
- `status`, `outcome`, `executions`, `restarts`;
- approval level and its source;
- append-only events and a single wake lane.

Lifecycle: `queued -> running -> waiting -> resume -> needs_input -> completed / failed / cancelled`. Every transition writes the row and its event in one transaction. Budgets (`requests`, `executions`, `wakes`) renew only on durable progress; two continuations without a successful tool call, or three identical errors, fail the node as stalled. Restart reconciliation reruns a cut turn at most once and never reruns a turn that already acted outward. Invariants include: no result delivered twice, one wake per result, and no "approved" without an allow.

**CowBot implementation (Phase 1.4)**: the simplified assignment tree is implemented:

- Extend tasks with `root_id`, `parent_id`, `depth`, `origin`, `requester`, `mode`, `reply`, `status`, `executions`, `restarts`.
- Persist an append-only `assignment_events` table; every state change is transactional.
- Implement `queued -> running -> waiting -> resume -> needs_input -> completed/failed/cancelled`.
- Add per-root budgets and a stall guard; add restart reconciliation for the desktop app.
- Keep one wake path. Do not add a second orchestration mechanism beside it.

This is more work than a flat sequential crew, but it prevents the rewrite that happens when delegation, approvals, and restarts arrive.

### 3.2 Approval levels

OpenMausBot offers per-bot and per-thread levels: **Ask for approval**, **Auto-accept edits**, **Approve for me**, and **Full access**, plus a Codex-specific custom mode. Changes apply mid-turn where the engine allows it. Delegated work inherits the Chief's level under `level = max(own, inherited)`, capped by what the engine actually supports. A bot reading its own files never asks; everything else needs an explicit grant. Full access still does not bypass OS privacy, authentication, MFA, or workspace ownership. The decision log records who approved what.

**CowBot implementation (Phase 1.5)**: approval levels are stored per assignment and inherited for delegated work (`inherit_approval_level`). The static risk classifier, approval service, and `ToolGate` enforcement are implemented. Saved exact-command grants and per-thread overrides remain future work.

| CowBot mode | OpenMausBot analogue | Notes |
| --- | --- | --- |
| Ask | Ask for approval | Default for unknown/risky operations |
| SafeAuto | Auto-accept edits / Approve for me | Split later if the policy engine needs it |
| PlanOnly | (CowBot-specific) | Read-only planning and investigation |
| Trusted | Full access | Admin-enabled, audited, still blocks Critical |

Add per-task/per-thread overrides, `max(own, inherited)` for delegated work, saved exact-command grants, and auto-allow for reading the agent's own workspace. Keep the static risk floor from ADR-0003; OpenMausBot's provider-level modes inform UX, not our safety boundary.

### 3.3 Bot box sandbox

OpenMausBot cloud homes run every bot turn in a bubblewrap sandbox with mount, PID, and network namespaces, an egress proxy that only reaches public addresses, a harness gate that exposes only `/api/internal/*`, an environment allowlist, per-process capability tokens, a credential broker, and a seccomp filter. A self-check probes what a hostile bot can and cannot do.

**CowBot recommendation (Phase 3)**: keep the v1 subprocess sandbox, but design `SandboxBackend` for these layers:

- workspace-only mounts, separate temp/PID namespace;
- egress policy (public-only proxy or no network);
- credential broker (never place provider keys in the sandbox);
- environment allowlist and output/timeout limits;
- a self-check that reports which protections are active.

Do not claim container-grade isolation until these are implemented and tested.

### 3.4 Memory

OpenMausBot stores memory as plain Markdown: `MEMORY.md` (first 200 lines or 24 KB loaded each turn), topic files read on demand, daily logs, deterministic recall before a turn, quiet-time capture/upkeep, secret redaction, and atomic writes.

**CowBot recommendation (P1)**: keep v1 memory as session plus channel/task history, then adopt a transparent Markdown-based project memory with an explicit load budget, on-demand topic files, recall, and redaction. Avoid opaque vector-only memory until users can inspect and edit it.

### 3.5 Team sharing

OpenMausBot shares a whole department as one file: bots, skills, playbooks, group chats, shared instructions, routines, connection slots, and optional starter notes. The format has no field for secrets, chat history, models, computers, or visibility. Import is additive; routines arrive paused, skills off, connections disabled; a redaction scan runs before export; package ids and versions make re-sharing a new release.

**CowBot recommendation (already partly in ADR-0004)**: adopt the safety properties for `.cowtemplate` and `.cowpkg`:

- additive import only; never mutate existing records;
- safe defaults on import (approval Ask, tools disabled, no credentials);
- redaction scan for secrets before export;
- stable package/template ids and semantic versions;
- atomic import with full rollback.

### 3.6 Tool selection and large tool catalogs

OpenMausBot lets a bot keep all tools, allow only listed tools, or exclude listed tools; selectors are exact names or namespaced wildcards, and an empty allow list selects no tools. For large catalogs it offers a tool bridge: `find_tool(query)` and `use_tool(name, arguments)` reduce the prompt from 43–57 KB to about 15 KB without changing what the turn may call.

**CowBot recommendation (P1/P2)**: add allow/exclude tool selection per agent, with no fallback to "all tools". If the registry grows large (MCP/plugins), add a tool search bridge that routes through the same permission checks as a direct call.

### 3.7 Messaging UX and routing

OpenMausBot treats bots as contacts, groups them into departments, gives each channel its own roster, shared instructions, working folder, and responder rules, and can route unmentioned messages with a fast decision model that fails open to the channel lead.

**CowBot recommendation**: keep channels and threads as the primary surface; add per-channel agent rosters and shared instructions in Phase 1.6/2. Defer automatic routing/decision models until channels have enough traffic to justify them.

### 3.8 What not to copy

- **Electron** — CowBot uses Tauri.
- **Wrapping provider CLIs as the core engine** — CowBot keeps the `LlmProvider` abstraction, so any supported model protocol (OpenAI Chat Completions, Anthropic Messages, or OpenAI Responses) or local model works.
- **Cloud VMs, phone clients, voice calls, biometric flows** — outside CowBot's local-first scope.
- **The full provider permission-mode matrix** — CowBot has one policy model (ADR-0003); provider quirks should not leak into the domain.
- **Unbounded runtime complexity** — adopt the assignment record and invariants, not every edge-case rule from a mature multi-provider product.

---

## 4. CC Switch

Repository: <https://github.com/farion1231/cc-switch>

CC Switch is a desktop manager that keeps several API provider configurations for coding agents and switches the active one in a click, instead of hand-editing JSON/TOML/YAML files.

- Providers are stored as a named list with endpoint, model, and credentials; one is active at a time.
- Switching applies immediately to the running app.
- Provider management is separate from agent/tool settings and from workspace data.

**CowBot implementation (Phase 1.6)**: the desktop settings page stores providers with an explicit protocol — OpenAI Chat Completions, Anthropic Messages, or OpenAI Responses — in `app_data_dir()/config.json`, marks one as active, and rebuilds the runtime LLM client without a restart (ADR-0012, ADR-0016). Keys stay on the local machine and never enter `.cowpkg` exports. Provider presets for common endpoints exist in the editor; per-agent model selection and import/export of provider lists are follow-up work.

---

## 5. Adoption Summary

| Idea | Source | CowBot phase | Decision |
| --- | --- | --- | --- |
| Role/goal/backstory agents, task dependencies | CrewAI | Done | Adopted |
| Channels, threads, multi-user workspaces | Mattermost | In progress | Adopted |
| Assignment tree, lifecycle, event log, budgets, restart | OpenMausBot | 1.4 | Adopted (simplified) |
| Provider list, one-click switch, local credentials | CC Switch | 1.6 | Adopted (local `config.json`, ADR-0012) |
| Per-task approval overrides and inheritance | OpenMausBot | 1.5 | Adopted (assignment-level + inheritance) |
| Static allowlists and own-workspace reads | OpenMausBot | 1.5 | Partially: static risk + policy; saved grants later |
| Markdown project memory with budget and recall | OpenMausBot | P1 | Adopt |
| Additive team sharing with safe defaults and redaction | OpenMausBot | 1.7 / P1 | Adopt |
| Tool allow/exclude selectors and tool bridge | OpenMausBot | P1/P2 | Adopt later |
| Namespaces, egress gate, credential broker, seccomp | OpenMausBot | Phase 3 | Adopt later |
| Decision-model routing for channels | OpenMausBot | P2 | Defer |
| Electron, provider CLIs, cloud VMs, voice | OpenMausBot | — | Reject |
