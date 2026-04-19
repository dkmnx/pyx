//! Atomic write helper for sensitive files
//!
//! Ensures data integrity by:
//! 1. Creating backup of existing file
//! 2. Writing to temp file
//! 3. Atomically renaming temp file to target
//!
//! The backup file (.bak) contains the previous version of the data and is
//! kept after a successful write for crash recovery. If the primary file is
//! later corrupted, the backup can be used for recovery. The backup file has
//! the same restrictive permissions as the primary file.

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

    let _had_backup = if path.exists() {
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

    // Keep the backup intact after a successful write so it's available for
    // crash recovery. If the primary file is corrupted later, the backup
    // contains the last-known-good data. The backup has the same restrictive
    // permissions as the primary file, so the sensitive data exposure is
    // equivalent to the primary file itself.

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
    fn test_atomic_write_keeps_backup_for_recovery() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let backup = dir.path().join("test.json.bak");

        // First write: no prior file, no backup
        atomic_write_with_backup(&path, b"first", 0o600).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");
        assert!(!backup.exists());

        // Second write: backup contains the previous version for crash recovery
        atomic_write_with_backup(&path, b"second", 0o600).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        assert!(
            backup.exists(),
            ".bak should exist after write for crash recovery"
        );
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            "first",
            ".bak should contain the previous version for recovery"
        );
    }

    #[test]
    fn test_atomic_write_backup_contains_previous_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let backup = dir.path().join("test.json.bak");

        fs::write(&path, "original").unwrap();
        assert!(!backup.exists());

        // After atomic_write, .bak contains previous version for recovery
        atomic_write_with_backup(&path, b"new data", 0o600).unwrap();
        assert!(backup.exists());
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
    }
}
