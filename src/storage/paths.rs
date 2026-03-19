//! Data directory path resolution

use crate::error::{PyxError, Result};
use std::path::PathBuf;

/// Get the pyx data directory.
///
/// Resolution order:
/// 1. `XDG_DATA_HOME` (Linux/macOS explicit override)
/// 2. `dirs::data_local_dir()` (cross-platform: AppData\Local on Windows, etc.)
/// 3. `HOME` + `.local/share/` (Linux/macOS fallback)
pub fn get_data_dir() -> Result<PathBuf> {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg_data).join("pyx"));
    }

    if let Some(local_data) = dirs::data_local_dir() {
        return Ok(local_data.join("pyx"));
    }

    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".local").join("share").join("pyx"));
    }

    Err(PyxError::Config(
        "Could not determine data directory".to_string(),
    ))
}

/// Get path to database.json
pub fn database_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("database.json"))
}

/// Get path to models.json
pub fn models_cache_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("models.json"))
}

/// Get path to settings.json
pub fn settings_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("settings.json"))
}

/// Get path to master.key
pub fn master_key_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("master.key"))
}

/// Get path to providers.json
pub fn providers_env_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("providers.json"))
}

/// Get path to passphrase file (fallback when OS keyring unavailable)
pub fn passphrase_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join(".passphrase"))
}

/// Ensure data directory exists with proper permissions.
///
/// Unix: restricts to owner-only (0o700).
/// Windows: inherits from parent (typically already user-restricted).
pub fn ensure_data_dir() -> Result<PathBuf> {
    let dir = get_data_dir()?;

    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        }
        // On Windows, create_dir_all inherits ACLs from the parent directory,
        // which by default restricts access to the current user.
    }

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ENV_MUTEX;

    #[test]
    fn test_get_data_dir_with_xdg() {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("XDG_DATA_HOME", "/test/xdg");
        }

        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/xdg/pyx"));

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    /// HOME fallback is only reached on Unix when dirs::data_local_dir() is unavailable.
    #[cfg(unix)]
    #[test]
    fn test_get_data_dir_with_home() {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("HOME", "/test/home");
            std::env::remove_var("XDG_DATA_HOME");
        }

        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/home/.local/share/pyx"));

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_get_data_dir_resolves_something() {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }

        // With default environment, should resolve via dirs::data_local_dir() or HOME
        let path = get_data_dir();
        assert!(path.is_ok());
        assert!(path.unwrap().ends_with("pyx"));
    }
}
