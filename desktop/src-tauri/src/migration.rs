//! One-time migration from the pre-CowBot desktop data directory.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const LEGACY_IDENTIFIER: &str = "com.cowai.desktop";
const LEGACY_DATABASE: &str = "cowai.db";
const DATABASE: &str = "cowbot.db";
const SQLITE_SIDECARS: [&str; 2] = ["-wal", "-shm"];

/// Copy an existing CowAI database into the new app data directory.
///
/// The Tauri identifier changed from `com.cowai.desktop` to
/// `com.cowbot.desktop`, so the resolved app data directory changes with it.
/// On the first launch after the rename, copy the legacy database and its
/// SQLite sidecars when the new database does not exist yet. Failures are
/// reported but never block startup.
pub fn migrate_legacy_database(data_dir: &Path) {
    let target = data_dir.join(DATABASE);
    if target.exists() {
        return;
    }

    let Some(parent) = data_dir.parent() else {
        return;
    };
    let source = parent.join(LEGACY_IDENTIFIER).join(LEGACY_DATABASE);
    if !source.exists() {
        return;
    }

    if let Err(error) = copy_database(&source, &target) {
        eprintln!(
            "warning: failed to migrate legacy database {} to {}: {error}",
            source.display(),
            target.display()
        );
    }
}

fn copy_database(source: &Path, target: &Path) -> io::Result<()> {
    fs::copy(source, target)?;
    for suffix in SQLITE_SIDECARS {
        let source_sidecar = sidecar_path(source, suffix);
        if source_sidecar.exists() {
            fs::copy(&source_sidecar, sidecar_path(target, suffix))?;
        }
    }
    Ok(())
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_legacy_database_and_sidecars() {
        let root = tempfile::tempdir().unwrap();
        let new_dir = root.path().join("com.cowbot.desktop");
        let legacy_dir = root.path().join(LEGACY_IDENTIFIER);
        fs::create_dir_all(&new_dir).unwrap();
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::write(legacy_dir.join(LEGACY_DATABASE), b"database").unwrap();
        fs::write(legacy_dir.join("cowai.db-wal"), b"wal").unwrap();

        migrate_legacy_database(&new_dir);

        assert_eq!(fs::read(new_dir.join(DATABASE)).unwrap(), b"database");
        assert_eq!(fs::read(new_dir.join("cowbot.db-wal")).unwrap(), b"wal");
    }

    #[test]
    fn keeps_existing_database() {
        let root = tempfile::tempdir().unwrap();
        let new_dir = root.path().join("com.cowbot.desktop");
        let legacy_dir = root.path().join(LEGACY_IDENTIFIER);
        fs::create_dir_all(&new_dir).unwrap();
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::write(new_dir.join(DATABASE), b"new").unwrap();
        fs::write(legacy_dir.join(LEGACY_DATABASE), b"old").unwrap();

        migrate_legacy_database(&new_dir);

        assert_eq!(fs::read(new_dir.join(DATABASE)).unwrap(), b"new");
    }
}
