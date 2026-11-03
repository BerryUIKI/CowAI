# ADR 0005: Authentication and Identity Model

- **Status**: Accepted
- **Date**: 2026-11-03
- **Related**: `docs/PRD.md` §5.1, `docs/ARCHITECTURE.md` §3.1 / §7

## Context

v1 is multi-user from the first release. The desktop local mode must work with zero configuration, while the self-hosted server needs secure multi-user authentication. The v1 authentication methods, token model, and OIDC/SSO priority must be defined.

## Decision

1. **v1 local accounts**: email/username + password hashed with **Argon2id**; the first registered user becomes the workspace `Owner`.
2. **Session tokens**: the server issues opaque random tokens, stores only hashes, and supports revocation; the desktop stores them in the OS keychain (Tauri keychain plugin); API clients use the `Bearer` header.
3. **API tokens**: prefixed with `cow_` for identification, bound to a user and workspace, scoped and revocable; only hashes are stored.
4. **Workspace roles**: `Owner` / `Admin` / `Member`. Local mode auto-creates a local Owner and a default workspace, with no password by default and an optional app lock.
5. **Invitations**: single-use invite tokens/links, distributed manually by an administrator; v1 does not depend on an email service.
6. **OIDC/SSO is P1**: the auth layer is designed around a pluggable `AuthProvider` for enterprise directory integration.
7. **Transport security**: the server requires HTTPS by default (TLS terminated by a reverse proxy); LAN/development environments may opt in explicitly with `--allow-insecure`.
8. **v1 baseline protection**: login failure rate limiting and temporary lockout; no email verification or self-service password reset, but administrators can reset passwords.
9. **Data-at-rest encryption is not in v1**: it relies on OS accounts and disk encryption, and is recorded as a known limitation.

## Rationale

- Opaque session tokens can be revoked immediately, avoiding JWT key management and revocation problems;
- Argon2id is the current recommended password hashing algorithm;
- Passwordless local mode and authenticated team mode combine zero configuration with security;
- OIDC/SSO comes after the server MVP to avoid scope creep in v1.

## Consequences

- Session storage, token hashing, rate limiting, and revocation logic must be implemented;
- Password reset, email notifications, and OIDC integration are later milestones;
- Local-mode data security depends on OS accounts, and the UI must make this clear;
- A future browser entry point will require Cookie/CSRF policies.

## Alternatives

- **JWT**: stateless and scalable, but revocation and key rotation are complex; not ideal for a self-hosted start.
- **OAuth/OIDC first**: enterprise-friendly but high configuration cost for personal local mode.
- **Basic Auth**: simplest to implement but insufficiently secure for a long-term solution.
