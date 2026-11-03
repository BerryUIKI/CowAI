# ADR 0002: Template Format and Distribution

- **Status**: Accepted
- **Date**: 2026-11-03
- **Related**: `docs/PRD.md` §5.1, `docs/ARCHITECTURE.md` §3.3

## Context

CowAI needs built-in agent and team templates while allowing users to customize and share them. Templates must work offline and must not become an entry point for code execution or supply-chain attacks.

## Decision

1. **Use JSON** with fields including `schema_version`, `kind` (`agent` / `team`), `id`, `name`, `description`, `version`, `tags`, `agents`, and `tasks`.
2. **Compile built-in templates into the binary**: template files live in `templates/builtin/*.json` and are bundled via `include_str!` or `rust-embed`, guaranteeing offline availability and version consistency.
3. **Store user templates in SQLite**, viewable, copyable, and editable through the template library UI.
4. **Import/export uses `.cowtemplate`** (a single JSON file); workspace packages also contain template definitions.
5. **No online template marketplace in v1**; only built-in templates and local file import/export. An online registry and signing are deferred to P2.
6. **Templates only reference tool names and model configuration; they never embed executable code or scripts.** Loading validates against a JSON Schema plus semantic checks; unknown fields are ignored for forward compatibility.

## Rationale

- JSON is readable, diffable, and uses the same serialization as the HTTP API;
- Compiling templates into the binary avoids network or filesystem dependencies at first launch;
- Prohibiting embedded code reduces supply-chain and privilege-escalation risk;
- File import/export supports sharing without introducing a server dependency.

## Consequences

- A template `schema_version` and migration strategy must be maintained;
- An online marketplace, signature verification, and template ratings require later design;
- Tool references in templates must be validated at import time.

## Alternatives

- **TOML / YAML**: readable, but requires an extra API conversion layer and weaker cross-platform consistency.
- **Database-only storage**: easy to manage but awkward to share and version-control.
- **Online registry first**: good UX but unusable offline and introduces trust problems.
