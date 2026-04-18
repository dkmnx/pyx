//! Data directory path resolution

use crate::error::{PyxError, Result};
use std::path::PathBuf;

/// Get the pyx data directory.
///
/// Resolution order:
/// 1. `XDG_DATA_HOME` (Linux/macOS explicit override)
/// 2. `dirs::data_local_dir()` (cross-platform: AppData\Local on Windows, etc.)
/// 3. Platform-specific env vars: LOCALAPPDATA, USERPROFILE, HOME
/// 4. Error if no directory can be determined
pub fn get_data_dir() -> Result<PathBuf> {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        if !xdg_data.is_empty() {
            return Ok(PathBuf::from(xdg_data).join("pyx"));
        }
    }

    if let Some(local_data) = dirs::data_local_dir() {
        return Ok(local_data.join("pyx"));
    }

    // Try LOCALAPPDATA directly (Windows, used by dirs::data_local_dir)
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        if !local_app_data.is_empty() {
            return Ok(PathBuf::from(local_app_data).join("pyx"));
        }
    }

    // Try USERPROFILE\AppData\Local (Windows fallback)
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        if !userprofile.is_empty() {
            return Ok(PathBuf::from(userprofile)
                .join("AppData")
                .join("Local")
                .join("pyx"));
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Ok(PathBuf::from(home).join(".local").join("share").join("pyx"));
        }
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

/// Get path to pi binary path cache
pub fn pi_path_cache() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("pi.path"))
}

/// Ensure data directory exists with proper permissions.
///
/// Unix: restricts to owner-only (0o700).
/// Windows: relies on inherited ACLs (same as dirs::data_local_dir behavior).
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
    use crate::test_helpers::EnvGuard;
    use crate::ENV_MUTEX;

    #[test]
    fn test_get_data_dir_with_xdg() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _guard2 = EnvGuard::set_var("XDG_DATA_HOME", "/test/xdg");
        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/xdg/pyx"));
    }

    /// HOME fallback is only reached on Unix when dirs::data_local_dir() is unavailable.
    #[cfg(unix)]
    #[test]
    fn test_get_data_dir_with_home() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let _xdg_guard = EnvGuard::remove_var("XDG_DATA_HOME");
        let _home_guard = EnvGuard::set_var("HOME", "/test/home");
        let path = get_data_dir().unwrap();
        // macOS resolves dirs::data_local_dir() to ~/Library/Application Support before HOME fallback
        let expected_macos = PathBuf::from("/test/home/Library/Application Support/pyx");
        let expected_generic = PathBuf::from("/test/home/.local/share/pyx");
        assert!(path == expected_macos || path == expected_generic);
    }

    #[test]
    fn test_get_data_dir_resolves_something() {
        // With default environment, should resolve via dirs::data_local_dir() or HOME
        let path = get_data_dir();
        assert!(path.is_ok());
        assert!(path.unwrap().ends_with("pyx"));
    }
}
