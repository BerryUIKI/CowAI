# ADR 0001: Runtime and Deployment Model

- **Status**: Accepted (deployment form delegated to the implementation by the user)
- **Date**: 2026-11-03
- **Related**: `docs/PRD.md` §2.7
- **Decision makers**: CowAI project

## Context

CowAI must satisfy three requirements at once:

1. **Offline personal use**: no external server, with data staying on the local machine.
2. **Team/enterprise self-hosting**: multiple users, shared workspaces, private-network deployment.
3. **Future expansion**: browser entry points, webhooks, and additional runtimes.

The previous CLI-only baseline cannot carry channel-native human-AI collaboration, multi-user support, or a desktop console, so the runtime and deployment model must be decided explicitly.

## Decision

1. **Single Rust core**: business logic lives in `cow-core` and its downstream crates, shared by desktop and server to avoid duplicate implementations.
2. **Desktop app uses Tauri**: Rust backend + web frontend, packaged for Windows / macOS / Linux; embeds the core and SQLite and needs no server by default.
3. **Server is a headless Rust service**: a new `cow-server` exposes HTTP + WebSocket APIs and supports single-binary and Docker deployment.
4. **Unified client abstraction**: the desktop app accesses the core through `cow-client`:
   - **Local mode**: in-process calls into the core, low latency and zero configuration;
   - **Team mode**: HTTP/WebSocket connection to `cow-server`.
5. **Storage strategy**: SQLite by default (desktop and small servers); PostgreSQL optional for enterprise scale (P1), switchable through a storage abstraction.
6. **Multi-user enters the data model and APIs from v1**: `User`, `Workspace`, `Membership`, and `Role` exist from the start; local mode auto-creates a single-user workspace.

## Rationale

- Tauri matches the Rust stack and has a significantly smaller footprint than Electron;
- A headless service plus a single binary fits self-hosted and enterprise intranet deployments;
- In-process calls avoid requiring a local server process and port management on the desktop;
- SQLite lowers the first-deployment barrier while the storage abstraction leaves room for PostgreSQL;
- A shared core boundary and API contract keep local and team modes behaviorally consistent.

## Consequences

- A stable core API boundary and a `cow-client` abstraction must be designed;
- Authentication and sessions differ between desktop (local session) and server (account-based);
- Data synchronization or import/export between local and team modes must be designed separately;
- The frontend stack remains open but is isolated by the Tauri WebView and can evolve independently;
- New crates (`cow-server`, `cow-client`, `cow-store`) are added and delivered in phases.

## Alternatives

- **Electron**: mature ecosystem but larger footprint and lower reuse of the Rust core.
- **Browser-only SPA + server**: cannot satisfy the optional serverless desktop/offline requirement.
- **CLI-only**: cannot carry channel-native collaboration or a console experience.
- **Desktop launches a local server process**: feasible but adds process management and port-conflict complexity; not the default.
