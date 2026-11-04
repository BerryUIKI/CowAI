//! `.cowpkg` workspace packages: a documented, verifiable way to move a
//! workspace between CowBot installations without exporting credentials.
//!
//! See `docs/ADR/0004-import-export-package.md` and `docs/PACKAGE_FORMAT.md`.

mod export;
mod import;
mod manifest;

pub use export::{export_workspace, MAX_PACKAGE_BYTES};
pub use import::{import_workspace, inspect, ImportSummary};
pub use manifest::{
    ExportedBy, Manifest, PackageError, PackageKind, WorkspaceHeader, PACKAGE_FORMAT,
    PACKAGE_SCHEMA_VERSION,
};
