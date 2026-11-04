# Security policy

## Current status and trust boundaries

CowBot is an early-development local runtime with a CLI and an incremental desktop shell. No production-hardening guarantee, supported-version matrix, or dedicated security response SLA is established. Reports against the current source are welcome; older versions may require an upgrade rather than a backport.

- The subprocess backend executes commands as the current host user. It is not a container, virtual machine, or OS isolation boundary.
- Timeouts and returned-output truncation do not bound total memory use, prevent network access, or guarantee termination of every descendant process.
- CLI file tools are unrestricted. The optional library base path joins paths but does not enforce traversal or symlink containment.
- CLI crew execution gates tool calls through persisted approval policy (default `ask`). Policy is a decision layer, not OS isolation. Direct library use can omit the optional tool gate; `trusted` deliberately permits broader execution. Review policy configuration before using a live model.
- Commands inherit the environment. Prompts, file content, and tool outputs can be sent to the configured model endpoint. Console/debug logs may contain sensitive task and tool data.
- Provider credentials, endpoint trust, and hosted-service data retention must be managed by the operator.

Run live tasks only in a disposable environment with restricted OS permissions and no valuable files or credentials. Treat model output as untrusted input. SQLite stores local tasks, messages, runs, tool calls, and audit data; protect database files and backups. Container/WASM isolation and remote authentication remain future work.

## Reporting a vulnerability

Do not post exploit details, credentials, or private data in a public issue. Use GitHub's private vulnerability reporting **if enabled** on the repository's [Security page](https://github.com/BerryUIKI/CowAI/security). If it is unavailable, open a public issue asking maintainers to establish a private reporting channel, without including vulnerability details. No dedicated security email is currently published.

Once a private channel is available, include the affected revision, platform, minimal reproduction, expected and observed behavior, impact, and any suggested mitigation. Use synthetic data and redact secrets. Maintainers should acknowledge the report, coordinate a fix and disclosure with the reporter, and avoid publishing details before a mitigation is ready. Response timing depends on maintainer availability.
