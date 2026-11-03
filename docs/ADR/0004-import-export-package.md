# ADR 0004: Import/Export Package Format

- **Status**: Accepted
- **Date**: 2026-11-03
- **Related**: `docs/PRD.md` §7.1, `docs/ARCHITECTURE.md` §4.3

## Context

Local and team modes keep independent data, and v1 migrates between them through import/export. The format must be portable, reviewable, and compatible across versions, while never carrying credentials or password hashes out of the system.

## Decision

1. **File extension `.cowpkg`, ZIP container.**
2. **`manifest.json`** at the archive root, containing:
   - `format`: `"cowai-package"`;
   - `schema_version`: integer;
   - `kind`: `workspace` / `template` / `task-archive`;
   - `app_version`, `created_at`, `exported_by`;
   - `checksums`: a map from file path to SHA-256.
3. **Content layout**:
   ```
   manifest.json
   workspace.json
   agents.json
   templates.json
   channels.jsonl
   messages.jsonl
   tasks.json
   runs.jsonl
   audit.jsonl
   attachments/...
   ```
   Large data sets (messages, runs, audit) use JSONL for streaming reads and writes.
4. **Never export secrets**: API keys, model credentials, password hashes, and session/API tokens are excluded; they must be reconfigured after import.
5. **v1 import semantics**:
   - `template`: merge into the template library (deduplicate by `id` + version);
   - `workspace`: create a new workspace; no in-place merge or ID remapping;
   - `task-archive`: import into a specified channel/workspace as a read-only archive.
6. **Validation and transactions**: validate the manifest, `schema_version`, checksums, and package size before import; roll back the whole import on failure, leaving no partial state.
7. **Compatibility**: `schema_version` increases monotonically; the importer supports the current version, and breaking changes require a migrator; new fields remain backward compatible.

## Rationale

- JSON/JSONL is readable, diffable, and reviewable, avoiding the version and platform coupling of raw SQLite files;
- A ZIP container carries attachments and multi-file structures cleanly;
- Excluding secrets reduces leakage risk;
- "Create a new workspace, never merge in place" significantly reduces v1 complexity and conflict handling.

## Consequences

- Streaming ZIP reads/writes and checksum verification must be implemented;
- Model and tool credentials must be reconfigured after import;
- Future merge/sync support would require ID mapping and conflict resolution;
- The package format needs versioned documentation and compatibility tests.

## Alternatives

- **Export the SQLite file directly**: simple, but unreliable across versions/platforms and contains sensitive data.
- **A single JSON file**: awkward for attachments and large data sets.
- **An online sync protocol**: better UX but high v1 complexity and conflicts with the "independent data" decision.
