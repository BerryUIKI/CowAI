# ADR 0023: Scoped Assignment Scheduling for Background Turns

- **Status**: Accepted
- **Date**: 2026-11-07
- **Related**: `docs/ADR/0006-assignment-runtime.md`, `docs/ADR/0019-assignment-scheduler.md`, `docs/ADR/0022-channel-mention-turns.md`

## Context

`AssignmentScheduler::run_until_idle` scans the assignments table for `queued` and `resume` rows and dispatches everything it finds. ADR-0019 chose that on purpose: one visible run at a time, started explicitly, so draining the workspace is the right behaviour.

ADR-0022 introduces background channel turns. Each turn builds its own scheduler and runs it while the user may start a task run from the Tasks view or send a second message. Two unscoped schedulers would both scan the whole table and race for the same rows: one could claim another turn's assignment (and fail it, since it registered only its own agents), and the visible crew run would compete with a background turn for the same queue.

The desktop already guards workspace switching and updates while tasks run, but task runs and turns can still be started concurrently, so the guard cannot be the only answer.

## Decision

`AssignmentScheduler::with_scope()` puts a scheduler in scoped mode. A scoped scheduler records the `root_id` of every assignment it enqueues and claims only roots in that set; an unscoped scheduler keeps the previous behaviour and claims any queued or resumed work.

`ChannelAgentService` always builds a scoped scheduler, so a channel turn claims exactly the roots it created. The CLI crew run and the desktop task run remain unscoped for now: they are user-initiated and, in practice, run one at a time.

The scope set is a `std::sync::Mutex<HashSet<AssignmentId>>`, held only for a set lookup or insert and never across an `await`. A poisoned lock is treated as "not tracked", which skips the root rather than dispatching it from a scheduler that cannot account for it.

## Alternatives considered

- **One process-wide scheduler with a work queue**: the cleaner long-term shape and what Phase 2 will need, but it requires the runtime to own LLM and tool handles for the workspace. Today those are supplied per call by the CLI and the desktop, so a resident scheduler cannot execute anything on its own. Deferred to the server phase.
- **A database lock or `claimed_by` column**: solves the same race across processes, which is a Phase 2 concern. Within one process the in-memory scope set is simpler and needs no migration.
- **Serialize turns and task runs behind one mutex**: correct but pessimistic; two channel turns answering different messages have no reason to block each other.

## Consequences

- Independent channel turns run concurrently without stealing each other's work, and a background turn no longer competes with a visible task run.
- An unscoped scheduler still drains the workspace. A future non-interactive caller must use `with_scope()` before running concurrently with anything else.
- The scope set lives as long as the scheduler clone, so a caller that enqueues on one clone and runs another would see no work. `CrewRunner` and `ChannelAgentService` enqueue and run on the same instance.
- Coverage: `crates/cowbot-service/tests/scheduler.rs::a_scoped_scheduler_claims_only_its_own_roots`.
