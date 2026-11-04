# ADR 0007: Client API Boundary

- **Status**: Accepted
- **Date**: 2026-11-04
- **Related**: `docs/ADR/0001-runtime-and-deployment.md`, `docs/ARCHITECTURE.md` §3.9, `docs/ROADMAP.md` Phase 1.6 / 2.3

## Context

ADR-0001 requires a unified client abstraction: local mode calls the core in-process, team mode connects to `cow-server`, and the difference must stay inside `cow-client`. The desktop UI and CLI must not depend on `cow-store` or `cow-service` types directly. The crate did not exist, so the UI could not be built without binding itself to storage internals, and the CLI bypassed persistence, approvals, and startup reconciliation entirely.

## Decision

### 1. Transport-neutral trait

`cow-client` exposes a `Client` trait. The desktop UI and CLI hold a `Client` implementation, not a concrete backend:

```rust
pub trait Client: Send + Sync {
    async fn session(&self) -> CowResult<ClientSession>;
    fn subscribe(&self) -> EventStream;
    async fn list_channels(&self) -> CowResult<Vec<Channel>>;
    async fn create_channel(&self, name: &str, topic: &str) -> CowResult<Channel>;
    async fn post_message(&self, channel_id: &ChannelId, content: &str) -> CowResult<ChannelMessage>;
    async fn post_thread_message(&self, channel_id: &ChannelId, content: &str, thread_id: &str) -> CowResult<ChannelMessage>;
    async fn list_messages(&self, channel_id: &ChannelId, limit: u32) -> CowResult<Vec<ChannelMessage>>;
}
```

The trait grows as later slices add commands. Adding methods is additive for in-repo implementations.

### 2. Local mode

`LocalClient` wraps `cow-store` and `cow-service` in-process. Opening a client migrates the database, bootstraps the local user/workspace, and runs assignment and approval startup reconciliation before returning.

### 3. Identity and scoping

`ClientSession` carries a sanitized `ClientUser` (`id`, `name`, `email`) and the active `Workspace`. Credentials never cross the boundary. Commands are implicitly scoped to the session's workspace and user; agent and system writes remain runtime concerns, not client commands.

### 4. Events

`EventStream` is a transport-neutral `mpsc` receiver of domain events. Local mode forwards the in-process broadcast bus into it and skips lagged events with a warning. A remote implementation can feed the same type from a WebSocket subscription.

### 5. Errors and implementation hiding

Client methods return `CowResult`, reusing `CowError`; a remote implementation maps transport failures into it. `Store`, `cow-service` services, and bus handles stay private to the client crate. Callers see domain types from `cow-core` only.

## Consequences

- The UI and CLI compile against one trait and can switch from local to remote mode without changing view code.
- The client surface must stay in sync with UI needs; each slice is tracked in its own issue/PR.
- Local mode depends on `cow-service` (and transitively SQLite). A future remote-only build may feature-gate the local implementation.
- Startup reconciliation is no longer something callers must remember; it runs whenever a local client opens.
- Authentication, reconnection, and request pagination are transport concerns deferred to roadmap 2.3.
