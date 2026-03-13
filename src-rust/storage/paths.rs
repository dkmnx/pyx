//! Data directory path resolution

use crate::error::{PyxError, Result};
use std::fs;
use std::path::PathBuf;

/// Get the pyx data directory
/// Uses XDG_DATA_HOME or defaults to ~/.local/share/pyx
pub fn get_data_dir() -> Result<PathBuf> {
    // Check XDG_DATA_HOME environment variable
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg_data).join("pyx"));
    }

    // Fall back to ~/.local/share/pyx
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".local").join("share").join("pyx"));
    }

    Err(PyxError::Config(
        "Could not determine data directory: HOME not set".to_string(),
    ))
}

/// Get the legacy ply data directory (for migration)
pub fn get_legacy_data_dir() -> Option<PathBuf> {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        let path = PathBuf::from(xdg_data).join("ply");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".local").join("share").join("ply");
        if path.exists() {
            return Some(path);
        }
    }

    None
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
        // Try to migrate from legacy ply directory
        if let Some(legacy_dir) = get_legacy_data_dir() {
            migrate_from_ply(&legacy_dir, &dir)?;
        } else {
            std::fs::create_dir_all(&dir)?;

            // Set directory permissions (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
            }
        }
    }

    Ok(dir)
}

/// Migrate data from legacy ply directory to pyx
fn migrate_from_ply(legacy_dir: &PathBuf, new_dir: &PathBuf) -> Result<()> {
    println!(
        "Migrating data from {} to {}...",
        legacy_dir.display(),
        new_dir.display()
    );

    // Create new directory
    std::fs::create_dir_all(new_dir)?;

    // Set directory permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(new_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    // Files to migrate
    let files = [
        "database.json",
        "master.key",
        "settings.json",
        "providers.json",
        "models.json",
    ];

    let mut migrated = 0;
    for file in files {
        let src = legacy_dir.join(file);
        let dst = new_dir.join(file);
        if src.exists() && !dst.exists() {
            if let Err(e) = fs::copy(&src, &dst) {
                eprintln!("Warning: Failed to copy {}: {}", file, e);
            } else {
                // Set file permissions (Unix only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&dst, fs::Permissions::from_mode(0o600));
                }
                migrated += 1;
            }
        }
    }

    if migrated > 0 {
        println!("✓ Migrated {} file(s)", migrated);
    }

    Ok(())
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
        assert_eq!(path, PathBuf::from("/test/home/.local/share/pyx"));

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
        assert_eq!(path, PathBuf::from("/test/xdg/pyx"));

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
