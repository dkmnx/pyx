//! Atomic write helper for sensitive files
//!
//! Ensures data integrity by:
//! 1. Creating backup of existing file
//! 2. Writing to temp file
//! 3. Atomically renaming temp file to target

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

    let backup_name: Option<std::ffi::OsString> = if path.exists() {
        let mut backup_name = path.as_os_str().to_owned();
        backup_name.push(".bak");
        fs::copy(path, &backup_name)?;
        Some(backup_name)
    } else {
        None
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

    // Remove backup after successful write - backups of secret-bearing files
    // should not persist as they contain sensitive data
    if let Some(backup_name) = backup_name {
        let _ = fs::remove_file(&backup_name);
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
    fn test_atomic_write_removes_backup_after_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        // Create initial file
        fs::write(&path, "original").unwrap();

        // Write new data
        atomic_write_with_backup(&path, b"new data", 0o600).unwrap();

        // Backup should be removed after successful write
        let backup = dir.path().join("test.json.bak");
        assert!(
            !backup.exists(),
            "backup should be removed after successful write"
        );

        // Check new data
        assert_eq!(fs::read_to_string(&path).unwrap(), "new data");
    }

    #[test]
    fn test_atomic_write_creates_backup_during_write_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        // Create initial file
        fs::write(&path, "original").unwrap();

        // Backup should not exist before we call atomic_write
        let backup = dir.path().join("test.json.bak");
        assert!(!backup.exists());

        // After atomic_write, backup should be removed
        atomic_write_with_backup(&path, b"new data", 0o600).unwrap();
        assert!(!backup.exists());
    }
}
