# ADR 0015: pnpm for Desktop Dependencies

- **Status**: Accepted
- **Date**: 2026-11-05
- **Related**: `docs/ADR/0009-desktop-ipc-and-state.md`, `docs/ADR/0014-desktop-release-builds.md`

## Context

The desktop frontend (`desktop/`) was installed with npm and locked with
`package-lock.json`. npm hoists transitive dependencies into a flat
`node_modules`, so frontend code can import packages that `package.json` never
declares ("phantom dependencies") and only fail in CI or after a dependency
change. Every checkout also keeps its own copy of each package.

The desktop app is a single package with a small dependency set, so a strict,
content-addressed installer costs little and removes a class of resolution
mistakes. Node.js does not bundle pnpm, so contributors obtain it through
Corepack or a global install, and CI installs it explicitly.

## Decision

`desktop/` uses pnpm 11:

- `package.json` pins `"packageManager": "pnpm@11.18.0"` and requires Node.js
  22.13 or newer (pnpm 11's minimum). `package-lock.json` is replaced by
  `pnpm-lock.yaml`, which is committed.
- pnpm's isolated `node_modules` layout keeps only declared dependencies
  reachable from the project root; transitive packages stay under
  `node_modules/.pnpm`. Code must import only what `package.json` declares.
- pnpm 11 reads settings from `pnpm-workspace.yaml`; the `pnpm` field in
  `package.json` is ignored. `allowBuilds` explicitly permits esbuild's install
  script, which Vite needs to place its native binary. Other dependencies run no
  install scripts, and new ones require an explicit decision.
- `tauri.conf.json` runs `pnpm run dev` and `pnpm run build` for
  `beforeDevCommand` and `beforeBuildCommand`.
- CI (`ci.yml`) and the release workflow (`release.yml`) install pnpm with
  `pnpm/action-setup` using the pinned version, cache with
  `actions/setup-node`'s pnpm support keyed on `desktop/pnpm-lock.yaml`, and
  install with `pnpm install --frozen-lockfile`. Node.js moves from 20 to 22.

## Alternatives considered

- **Keep npm.** Zero installation cost, but keeps phantom dependencies and
  per-checkout duplication; rejected because the migration is small and the
  desktop package is self-contained.
- **Yarn.** Comparable to pnpm but adds a second competing tool without a
  concrete benefit here; rejected.
- **Bun.** Faster installs, but a different runtime and lockfile with less
  mature Windows/Tauri support; rejected for a release-critical build path.
- **Hoisted pnpm layout** (`node-linker=hoisted`). Restores phantom-dependency
  tolerance for compatibility; unnecessary because the frontend builds cleanly
  under the isolated layout, and rejected unless a dependency requires it.

## Consequences

- Imports of undeclared packages now fail locally and in CI, which surfaces
  latent dependency mistakes early.
- Installs share a global content-addressed store, so repeated checkouts and CI
  runs use less disk and time.
- Contributors and CI must have pnpm; `corepack enable pnpm` provides it from
  the pinned `packageManager` field. The Node.js minimum rises to 22.13.
- `pnpm-workspace.yaml` must be updated when the desktop package needs a
  dependency's install script; unapproved scripts are skipped, not executed.
- The release workflow's platform matrix and bundle formats are unchanged.
