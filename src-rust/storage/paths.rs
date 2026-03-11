//! Data directory path resolution

use crate::error::{PyxError, Result};
use std::path::PathBuf;

/// Get the pyx data directory
/// Uses XDG_DATA_HOME or defaults to ~/.local/share/ply
pub fn get_data_dir() -> Result<PathBuf> {
    // Check XDG_DATA_HOME environment variable
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg_data).join("ply"));
    }

    // Fall back to ~/.local/share/ply
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".local").join("share").join("ply"));
    }

    Err(PyxError::Config(
        "Could not determine data directory: HOME not set".to_string(),
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

/// Ensure data directory exists with proper permissions
pub fn ensure_data_dir() -> Result<PathBuf> {
    let dir = get_data_dir()?;

    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;

        // Set directory permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Prevent tests from running in parallel since they modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_data_dir_with_home() {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("HOME", "/test/home");
            std::env::remove_var("XDG_DATA_HOME");
        }

        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/home/.local/share/ply"));

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_get_data_dir_with_xdg() {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("XDG_DATA_HOME", "/test/xdg");
        }

        let path = get_data_dir().unwrap();
        assert_eq!(path, PathBuf::from("/test/xdg/ply"));

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
