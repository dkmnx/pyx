//! Data directory path resolution

use crate::error::{PyxError, Result};
use std::path::PathBuf;

/// System path resolution traits for testability.
pub trait DataDirPaths {
    fn data_local_dir(&self) -> Option<PathBuf>;
    fn home_dir(&self) -> Option<PathBuf>;
}

/// Production implementation using real system calls.
pub struct RealDataDirPaths;

impl DataDirPaths for RealDataDirPaths {
    fn data_local_dir(&self) -> Option<PathBuf> {
        dirs::data_local_dir()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }
}

thread_local! {
    static DATA_DIR_PATHS: std::cell::RefCell<Box<dyn DataDirPaths + Send + Sync>> =
        std::cell::RefCell::new(Box::new(RealDataDirPaths));
}

#[cfg(test)]
pub fn set_data_dir_paths<P: DataDirPaths + Send + Sync + 'static>(paths: P) {
    DATA_DIR_PATHS.with(|s| {
        *s.borrow_mut() = Box::new(paths);
    });
}

#[cfg(test)]
pub fn reset_data_dir_paths() {
    DATA_DIR_PATHS.with(|s| {
        *s.borrow_mut() = Box::new(RealDataDirPaths);
    });
}

pub(crate) fn with_data_dir_paths<T>(f: impl FnOnce(&dyn DataDirPaths) -> T) -> T {
    DATA_DIR_PATHS.with(|s| f(s.borrow().as_ref()))
}

/// Get the pyx data directory.
///
/// Resolution order:
/// 1. `XDG_DATA_HOME` (Linux/macOS explicit override)
/// 2. `dirs::data_local_dir()` (cross-platform: AppData\Local on Windows, etc.)
/// 3. Platform-specific env vars: LOCALAPPDATA, USERPROFILE, HOME
/// 4. Error if no directory can be determined
pub fn get_data_dir() -> Result<PathBuf> {
    if let Ok(xdg_data) = crate::env_vars::var("XDG_DATA_HOME") {
        if !xdg_data.is_empty() {
            return Ok(PathBuf::from(xdg_data).join("pyx"));
        }
    }

    if let Some(local_data) = with_data_dir_paths(|p| p.data_local_dir()) {
        return Ok(local_data.join("pyx"));
    }

    // Try LOCALAPPDATA directly (Windows, used by dirs::data_local_dir)
    if let Ok(local_app_data) = crate::env_vars::var("LOCALAPPDATA") {
        if !local_app_data.is_empty() {
            return Ok(PathBuf::from(local_app_data).join("pyx"));
        }
    }

    // Try USERPROFILE\AppData\Local (Windows fallback)
    if let Ok(userprofile) = crate::env_vars::var("USERPROFILE") {
        if !userprofile.is_empty() {
            return Ok(PathBuf::from(userprofile)
                .join("AppData")
                .join("Local")
                .join("pyx"));
        }
    }

    if let Ok(home) = crate::env_vars::var("HOME") {
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

    // create_dir_all is idempotent — succeeds whether the directory exists
    // or not. No TOCTOU race window here.
    std::fs::create_dir_all(&dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // set_permissions is also idempotent on Unix.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    // On Windows, create_dir_all inherits ACLs from the parent directory,
    // which by default restricts access to the current user.

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::EnvGuard;

    struct MockDataDirPaths {
        data_local_dir: Option<PathBuf>,
        home_dir: Option<PathBuf>,
    }

    impl DataDirPaths for MockDataDirPaths {
        fn data_local_dir(&self) -> Option<PathBuf> {
            self.data_local_dir.clone()
        }

        fn home_dir(&self) -> Option<PathBuf> {
            self.home_dir.clone()
        }
    }

    #[test]
    fn test_get_data_dir_with_xdg() {
        let _guard2 = EnvGuard::set_var("XDG_DATA_HOME", "/test/xdg");
        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/xdg/pyx"));
    }

    /// HOME fallback is only reached on Unix when dirs::data_local_dir() is unavailable.
    #[cfg(unix)]
    #[test]
    fn test_get_data_dir_with_home() {
        set_data_dir_paths(MockDataDirPaths {
            data_local_dir: None,
            home_dir: Some(PathBuf::from("/test/home")),
        });
        let _xdg_guard = EnvGuard::remove_var("XDG_DATA_HOME");
        let _home_guard = EnvGuard::set_var("HOME", "/test/home");
        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/home/.local/share/pyx"));
        reset_data_dir_paths();
    }

    #[test]
    fn test_get_data_dir_resolves_something() {
        // With default environment, should resolve via dirs::data_local_dir() or HOME
        let path = get_data_dir();
        assert!(path.is_ok());
        assert!(path.unwrap().ends_with("pyx"));
    }
}
