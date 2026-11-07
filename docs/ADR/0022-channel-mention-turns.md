# ADR 0022: Channel Mention Turns

- **Status**: Accepted
- **Date**: 2026-11-07
- **Related**: `docs/ADR/0006-assignment-runtime.md`, `docs/ADR/0007-client-api.md`, `docs/ADR/0019-assignment-scheduler.md`, `docs/ADR/0023-channel-turn-scheduler-scope.md`, `docs/PRD.md` §2.5, `docs/ARCHITECTURE.md` §3.2

## Context

Phases 0–1 shipped channels as a persisted message timeline. `ChannelService::post_message` wrote the message and published `MessagePosted`, and nothing consumed that event: no component read a channel message, no code parsed an `@` mention, and the `Mention`, `Thread`, and `TaskCard` entities listed in `docs/ARCHITECTURE.md` §3.2 did not exist in `cowbot-core`. Agents could only be reached through the Tasks view, where `run_task` built a fixed Planner → Coder → Reviewer crew and never wrote its output back to a channel.

The product material promised the opposite. `docs/PRD.md` §2.5, `docs/ARCHITECTURE.md` §3.2 and §4.1, and the landing page demo all describe mentioning an agent in a channel and receiving an answer in that channel. Users typed into a channel, got no reply, and had no way to tell whether the model, the provider, or the product was at fault.

Phase 1 is complete and Phase 2 has not started, so the fix belongs in the local runtime. The open question was how much of the existing runtime a channel turn should reuse.

## Decision

### 1. A message with a mention is a task, not a side channel

`cowbot-service::ChannelAgentService` turns a message into one task per mentioned agent and runs those tasks through the existing `AssignmentScheduler`. It reuses `AssignmentService`, `ApprovalService`, `Run` and tool-call recording without a parallel execution path:

- The task description carries the channel name, the recent history, and the message, and asks for a concise reply suitable for the channel.
- The approval level comes from the caller (`ChannelTurnOptions::approval_level`, default `Ask`), so `ScopedApprovalGate` classifies and gates tool calls exactly as it does for a task run, and approvals appear in the Approvals view.
- Each turn therefore leaves the same audit trail as any other assignment: a root assignment, assignment events, a run, tool calls, and approvals.

The alternative — answering a mention with a direct `AgentInstance` call — was rejected. `ScopedApprovalGate` resolves the policy from an assignment id, so a direct call would need a new gate implementation and would either bypass approvals or duplicate them, and it would leave channel answers outside run history.

### 2. Mentions are resolved in `cowbot-core`

`crates/cowbot-core/src/mention.rs` parses `@Name` tokens that start the message or follow a non-word character, so `user@example.com` is not a mention, and deduplicates case-insensitively. `resolve_mentions` matches names and ids case-insensitively against the workspace agents, skips later agents that share a display name (a workspace that instantiated one template twice must not answer twice), and treats `@team`, `@all`, `@everyone`, and `@here` as addressing every distinct agent.

Parsing lives in the core crate because both the client and the desktop need the same rules; the desktop uses a small mirror of the parser to decide whether to show its "agents are working" note.

### 3. Feedback is always explicit

A turn never fails silently:

- An unknown mention produces a system notice listing the available agents.
- A workspace without agents produces a notice pointing at Settings.
- A failing agent's task produces a system notice with the error, and the other agents in the same turn still answer.
- An empty answer produces a notice instead of a blank message.

A message without a mention posts nothing beyond itself. The agents are not a default responder for the channel.

### 4. Output is written back as the agent

A completed task posts its output to the channel with `Sender::Agent(assignee)`, so the timeline attributes the answer and the desktop can render the agent's display name from `agent_profiles`. Answers stay as tasks in the Tasks view as well; that duplication is intentional, because the task detail is where a user reviews runs and tool calls.

### 5. The desktop runs turns in the background

`post_message` in the Tauri shell stores the message, then spawns the turn so the composer is not blocked, and emits `cowbot-channel-reply` when it settles. The frontend refreshes the timeline from `MessagePosted` events as the answers arrive and clears its "agents are working" note on the turn-finished event. A turn in flight blocks workspace switching, the same guard that applies to running crews.

The CLI exposes the same path as `cowbot message post --wait`, which lets scripts and tests drive a turn against a real endpoint.

## Alternatives considered

- **Reply through a direct agent call (no task, no assignment)**: smallest change, but it cannot reuse the approval gate, leaves no run or tool-call history, and would need its own budget and error handling. Rejected as a second execution path.
- **Run a whole Planner → Coder → Reviewer crew for each mention**: closer to the Task view, but a mention is a conversational request for one agent, and a three-agent pipeline would answer questions nobody asked. Rejected; the mentioned agent owns the turn.
- **Store a `Mention` entity and a responder configuration per channel**: matches the architecture sketch more closely but adds a schema migration and a settings surface before the behaviour itself exists. The mention is derived from the message text for now; a persisted mention table and per-channel responder rules are follow-up work.
- **Auto-detect the intended agent from message content**: rejected for Phase 1. An implicit responder makes it unclear who answered and when; the explicit `@Name` keeps the trigger visible in the transcript.

## Consequences

- A channel message that mentions an agent now produces an answer in the channel, backed by a normal assignment with approvals and history.
- Turns create tasks. The Tasks view will list them; users can cancel one like any other task.
- Two background turns can run at once, so the scheduler needs to claim only the work it enqueued. That narrow change is recorded separately in ADR-0023.
- Channel answers cost model calls and count against the same provider configuration as task runs.
- Deferred: threads as conversation scope (a reply currently uses the whole channel's recent history), persisted `Mention`/`TaskCard` entities, per-channel responder rules, provider-side streaming, and mention-aware notifications.
- Coverage: `crates/cowbot-core/src/mention.rs` unit tests, and `crates/cowbot-service/tests/channel_agents.rs` (answer, unknown mention, silent message, `@team` once per agent).
