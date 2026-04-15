//! Atomic write helper for sensitive files
//!
//! Ensures data integrity by:
//! 1. Creating backup of existing file
//! 2. Writing to temp file
//! 3. Atomically renaming temp file to target
//! 4. Truncating backup (kept as empty file for crash-recovery detection)
//!
//! The .bak file is truncated (not deleted) after a successful write. This gives
//! a crash-recovery window: if the process dies between the backup copy and the
//! persist, the .bak contains the previous data and can be used for recovery. Once
//! the write succeeds, the .bak is truncated to zero bytes so it no longer holds
//! sensitive data, but its presence signals that a previous version existed.

use crate::error::{PyxError, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Write data to file atomically with backup
pub fn atomic_write_with_backup<P: AsRef<Path>>(
    path: P,
    data: &[u8],
    permissions: u32,
) -> Result<()> {
    let path = path.as_ref();
    let mut backup_path = path.as_os_str().to_owned();
    backup_path.push(".bak");

    let had_backup = if path.exists() {
        fs::copy(path, &backup_path)?;
        true
    } else {
        false
    };

    let mut temp_file = NamedTempFile::new_in(path.parent().unwrap_or_else(|| Path::new(".")))?;
    temp_file.write_all(data)?;

    #[cfg(not(unix))]
    let _ = permissions;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp_file
            .as_file()
            .set_permissions(fs::Permissions::from_mode(permissions))?;
    }

    temp_file
        .persist(path)
        .map_err(|e| PyxError::TempFilePersist(format!("Failed to persist temp file: {e}")))?;

    // Truncate backup after successful write — removes sensitive data but keeps
    // the file indicator. Only truncate if we actually created a backup.
    if had_backup {
        if let Err(e) = fs::File::create(&backup_path) {
            eprintln!(
                "Warning: failed to truncate backup {}: {e}",
                Path::new(&backup_path).display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        atomic_write_with_backup(&path, b"test data", 0o600).unwrap();

        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "test data");
    }

    #[test]
    fn test_atomic_write_truncates_backup_after_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let backup = dir.path().join("test.json.bak");

        // First write: no prior file, no backup
        atomic_write_with_backup(&path, b"first", 0o600).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");
        assert!(!backup.exists());

        // Second write: backup created then truncated
        atomic_write_with_backup(&path, b"second", 0o600).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        assert!(backup.exists(), "truncated .bak should exist after write");
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            "",
            ".bak should be truncated (empty) after successful write"
        );
    }

    #[test]
    fn test_atomic_write_backup_exists_during_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let backup = dir.path().join("test.json.bak");

        fs::write(&path, "original").unwrap();
        assert!(!backup.exists());

        // After atomic_write, .bak exists but is empty
        atomic_write_with_backup(&path, b"new data", 0o600).unwrap();
        assert!(backup.exists());
        assert_eq!(fs::read_to_string(&backup).unwrap(), "");
    }
}
