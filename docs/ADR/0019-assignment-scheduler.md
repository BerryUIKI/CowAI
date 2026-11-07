# ADR 0019: Assignment Scheduler and Cross-Root Concurrency

- **Status**: Accepted
- **Date**: 2026-11-07
- **Related**: `docs/ADR/0006-assignment-runtime.md`, `docs/ADR/0007-client-api.md`, `docs/ADR/0008-cli-and-local-execution.md`, `docs/ROADMAP.md` milestone 1.4

## Context

ADR-0006 adopted the assignment runtime and deferred two pieces: the background wake scheduler and cross-root concurrency. `cowbot-service::AssignmentService` already recorded the `Resume` status when a child settled, and it enforced the request, execution, and wake budgets, but nothing consumed a resumed assignment. `CrewRunner` executed one task after another inline, enqueued each task only when its turn arrived, and had no path that dispatched a queued child or re-entered a waiting parent.

That left the wake budget and the `ReplyMode::Resume` contract untested in practice, and it kept independent tasks in one crew from making progress at the same time. Phase 1 completion needs a single dispatch path that consumes queued and resumed assignments.

## Decision

### 1. Scheduler inside `cowbot-service`

`crates/cowbot-service/src/scheduler.rs` adds `AssignmentScheduler`. It owns an `AssignmentService`, an `ApprovalService`, a map of registered agent execution handles, and a concurrency limit. It is created for one crew run and dropped when the run is idle; a process-wide resident scheduler is not needed while all work starts from an explicit run request.

`AgentExecution` holds the `AgentProfile`, the `Arc<dyn LlmProvider>`, the `ToolRegistry`, and the model name. Every assignment execution builds a fresh `AgentInstance`, so independent assignments never share mutable agent state.

### 2. Dispatch rules

`AssignmentScheduler::run_until_idle` is a background loop over a `JoinSet`:

- It scans assignments in `queued` and `resume` status.
- A queued assignment runs when every `context_tasks` dependency has a completed assignment. A dependency that is terminal but not completed fails the dependent assignment with a typed reason instead of silently dropping its context.
- A resumed assignment runs when all of its children are terminal; completed child outputs are appended to the task context, so a waiting parent re-enters with the child's deliverable.
- Assignments without a registered executor fail instead of spinning in the queue.
- One assignment per root executes at a time; a semaphore bounds the number of roots that execute concurrently (`CrewRunOptions::max_concurrency`, default 4).
- Each execution calls `AssignmentService::start`, creates a `Run`, installs a `ScopedApprovalGate` and a tool-call recorder scoped to the assignment, executes the agent turn, then calls `complete` or `fail`. Budgets, wake handling, the event log, and task-status projection are unchanged.

`ReplyMode::Resume` is the only reply mode that wakes a parent. `Poll` and `None` leave the parent waiting, matching the simplified rules ADR-0006 adopted.

### 3. Crew API

`CrewRunner::run_with_options` registers every agent in the crew, enqueues every task up front, and delegates to the scheduler. `CrewRunner::run` remains as a wrapper using default options. `CrewRunOptions` carries the approval level, an optional channel id for the root tasks, and the concurrency limit.

`cowbot-client::LocalClient` gains `run_crew_with_options`; `run_crew` keeps its existing signature and passes default options, so the desktop shell does not change.

## Alternatives considered

- **Keep the inline sequential runner**: simplest, but it cannot consume `resume` assignments and makes the wake budget dead code. It also cannot run independent roots concurrently.
- **Process-wide resident scheduler**: closer to a server runtime, but the current local client does not own LLM or tool handles; those are supplied per run by the CLI and desktop. A resident scheduler becomes useful with the server phase.
- **Hold one semaphore permit per root for its whole tree**: would deadlock when a waiting parent holds a permit while its child needs one. The scheduler instead permits concurrent executions and serializes one root through an active-root set.
- **Add an agent-facing delegation tool now**: would make wake/re-entry visible in ordinary conversations, but hierarchical orchestration is still deferred by ADR-0006. This ADR verifies delegation and wake behavior through the service API and integration tests.

## Consequences

- Independent root tasks in one crew run concurrently, bounded by `max_concurrency`.
- A parent that delegates work and goes `waiting` is resumed and re-executed with its children's outputs; the wake budget is exercised.
- A failed or cancelled dependency now fails the dependent assignment with a clear reason. This is stricter than the previous runner, which omitted missing context.
- Run history records one `Run` per attempt, including resumed attempts.
- Hierarchical manager/DAG process types, `consult`/`notify` edge cases, and agent-facing delegation remain deferred.
- Integration coverage lives in `crates/cowbot-service/tests/scheduler.rs`: concurrent roots, resume with child output, poll replies, dependency failure, and missing executors.
