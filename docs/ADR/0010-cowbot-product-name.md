# 0010: CowBot product name

- Status: Implemented; see ADR 0011 for the software rename
- Date: 2026-11-04
- Decision: Adopt CowBot as the product name at the owner's request.

## Context

The product is being renamed from CowAI to CowBot. This change covers documentation and documentation-only visual assets. The software implementation is assigned separately to the development team.

## Decision

Use CowBot in product prose, documentation diagrams, and documentation wordmarks. Keep executable commands and technical references accurate for the existing software: the current binary is `cowai`, and the application may still display CowAI.

Do not rename Rust crates, package identifiers, CLI commands, database files, environment variables, internal events, desktop identifiers, or application assets in this documentation change. Do not alter applied migrations. Keep the existing GitHub URL until the repository is renamed separately.

## Consequences

Documentation distinguishes the new product name from current executable and storage identifiers. Developers will implement the software rename separately, with any compatibility and migration behavior defined as part of that work. Update examples and availability notes when those changes land.
