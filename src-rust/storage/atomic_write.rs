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
    
    // Create backup if file exists
    if path.exists() {
        let backup_path = path.with_extension("json.bak");
        fs::copy(path, &backup_path)?;
    }
    
    // Write to temp file
    let mut temp_file = NamedTempFile::new_in(
        path.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    temp_file.write_all(data)?;
    
    // Set permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp_file.as_file().set_permissions(fs::Permissions::from_mode(permissions))?;
    }
    
    // Atomically rename
    temp_file.persist(path).map_err(|e| {
        PyxError::TempFilePersist(format!("Failed to persist temp file: {}", e))
    })?;
    
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
    fn test_atomic_write_creates_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        
        // Create initial file
        fs::write(&path, "original").unwrap();
        
        // Write new data
        atomic_write_with_backup(&path, b"new data", 0o600).unwrap();
        
        // Check backup exists
        let backup = path.with_extension("json.bak");
        assert!(backup.exists());
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
        
        // Check new data
        assert_eq!(fs::read_to_string(&path).unwrap(), "new data");
    }
}
