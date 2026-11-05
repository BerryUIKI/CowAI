# ADR 0006: Assignment Runtime for Task Orchestration

- **Status**: Accepted (scope delegated to the implementation by the user)
- **Date**: 2026-11-03
- **Related**: `docs/PRD.md` §5.1, `docs/ARCHITECTURE.md` §3.4 / §5, `docs/ROADMAP.md` Phase 1.4, `docs/REFERENCES.md` §3.1

## Context

The current `Task` and `Crew` model is a flat, sequential pipeline: tasks run one after another, context is passed explicitly, and there is no persistent record of who asked for work, which parent task spawned it, how many attempts it took, or what should happen after a restart.

OpenMausBot's runtime contract demonstrates the failure mode of a flat model: delegation, approvals, routines, restarts, and multiple wake paths eventually require a single job record, a single lifecycle, and an event log. CowBot does not need the full OpenMausBot rule set in v1, but it should adopt the core shape before Phase 1.4 is implemented.

## Decision

Adopt a **simplified assignment runtime** for Phase 1.4.

### 1. Assignment record

Each unit of agent work is an `Assignment`:

| Field | Meaning |
| --- | --- |
| `id` | Assignment identifier |
| `root_id`, `parent_id`, `depth` | Assignment tree; every root is a user/system request |
| `task_id` | The user-facing task this assignment executes |
| `origin` | `user`, `agent`, `system`, `routine`, `external` |
| `requester` | User ID, agent ID, or `system` |
| `assignee` | Agent ID |
| `mode` | `work`, `consult`, `notify` |
| `reply` | `resume`, `poll`, `none` |
| `status` | `queued`, `running`, `waiting`, `resume`, `needs_input`, `completed`, `failed`, `cancelled` |
| `executions`, `restarts` | Attempt and restart counters |
| `approval_level` | Effective approval level for this assignment (enforced in Phase 1.5) |
| `created_at`, `updated_at` | Timestamps |

An append-only `AssignmentEvent` record captures every transition (`enqueued`, `started`, `waiting`, `resumed`, `needs_input`, `settled`, `restarted`, `cancelled`). The assignment row and its event are written in one transaction.

### 2. Lifecycle

```
queued -> running -> waiting -> resume -> running -> completed / failed / cancelled
              \-> needs_input -> running
```

- A root assignment is enqueued from a user message, system action, or routine.
- A child assignment is enqueued by an agent that delegates work.
- `work` assignments execute; `consult` assignments answer without acting; `notify` assignments are one-way and never awaited.
- `reply` controls whether the requester is resumed, polls an inbox, or is not notified.
- A cancelled parent cancels descendants; a running child is stopped and reported.

### 3. Budgets and guards

- Per-root budgets: `requests`, `executions`, `wakes`; they renew only on durable progress.
- Depth limit and cycle prevention: no assignment may be delegated back to an ancestor.
- Stall guard: two consecutive continuations with no successful tool call, or three identical errors, settle the assignment as `failed(stalled)`.
- Undecidable or missing budget data fails safe rather than allowing unbounded work.

### 4. Restart reconciliation

On startup, before any assignment is dispatched:

- `queued` — dispatch normally.
- `running` with no children started — rerun once with a note to check what was already done; a second interruption settles as `failed(interrupted)`.
- `running` with children started — move to `waiting`; its resume notes the interruption.
- `needs_input` — stay parked.
- `waiting` / `resume` — remain unchanged.

Phase 1.4 does not track outward side effects (sends, posts, git pushes). The conservative rule is: rerun at most once, and never claim a side effect was undone. Phase 1.5 adds explicit outward-action tracking before it enables automatic retries.

### 5. Scope boundaries

Adopted in Phase 1.4:

- Assignment record, tree, lifecycle, event log, budgets, stall guard, restart reconciliation.
- Sequential Crew implemented on top of assignments.
- One wake path; no second orchestration mechanism.
- Approval level stored on the assignment; enforcement and inheritance are delivered in Phase 1.5.

Deferred:

- Hierarchical manager and DAG process types.
- Background wake scheduler and cross-root concurrency.
- Full OpenMausBot rule set (consult/notify edge cases, outward-action reconciliation, provider capability matrix).

## Invariants for Phase 1.4

1. No non-terminal assignment without an owner (running, queued, waiting, parked, or resuming).
2. Every assignment has at most one terminal outcome.
3. No assignment is dispatched twice except through an explicit restart rule.
4. Every status change has an event; the row can be reconstructed from its events.
5. A restart reruns a cut assignment at most once.
6. Refusals are typed results, not exceptions thrown into an agent turn.

## Consequences

- More schema and service work than a flat queue, but it prevents the rewrite that delegation, approvals, and restarts would otherwise force.
- The event log becomes the audit source for task execution and approval decisions.
- The desktop app can reconcile interrupted work on startup.
- The runtime stays local and synchronous in Phase 1.4; a background scheduler is not required yet.
- UI work in Phase 1.6 must display the assignment tree and its events, not only the task list.

## Alternatives

- **Flat sequential tasks only**: simplest, but cannot represent delegation, restarts, or per-child approvals.
- **Full OpenMausBot runtime contract**: validated by a mature product but far larger than CowBot's v1 scope.
- **External workflow engine (Temporal, etc.)**: heavy for a local-first desktop app and adds a service dependency.
