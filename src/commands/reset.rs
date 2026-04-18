//! Reset command implementation

use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::storage::paths::{
    database_path, master_key_path, models_cache_path, passphrase_path, providers_env_path,
    settings_path,
};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Append `.bak` to a path (backup naming convention)
fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".bak");
    PathBuf::from(name)
}

/// Execute the reset command
pub fn execute(skip_confirm: bool) -> Result<()> {
    eprintln!("=== Pyx Reset ===");
    eprintln!();
    eprintln!("WARNING: This will permanently delete all pyx data:");
    eprintln!("  - Master key");
    eprintln!("  - All stored API keys");
    eprintln!("  - Models cache");
    eprintln!("  - Settings");
    eprintln!();

    // Confirm deletion
    if !skip_confirm && !confirm_reset()? {
        eprintln!("Reset cancelled.");
        return Err(PyxError::Cancelled);
    }

    eprintln!();
    eprintln!("Deleting pyx data...");

    // Delete master key
    delete_file("Master key", &master_key_path()?)?;

    // Delete database
    let db_path = database_path()?;
    delete_file("Provider database", &db_path)?;
    delete_file("Database backup", &backup_path(&db_path))?;

    // Delete models cache
    delete_file("Models cache", &models_cache_path()?)?;

    // Delete settings
    let settings_p = settings_path()?;
    delete_file("Settings", &settings_p)?;
    delete_file("Settings backup", &backup_path(&settings_p))?;

    // Delete providers config
    delete_file("Providers config", &providers_env_path()?)?;

    // Delete passphrase fallback file (if exists)
    delete_file("Passphrase file", &passphrase_path()?)?;

    // Clear keyring
    if KeyManager::master_key_exists() || keyring_has_entry() {
        eprintln!("Clearing keyring entry...");
        KeyManager::clear_passphrase().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to clear keyring: {e}");
        });
    }

    eprintln!();
    eprintln!("✓ Reset complete!");
    eprintln!();
    eprintln!("Pyx has been reset to initial state.");
    eprintln!("Run 'pyx init' to initialize again.");

    Ok(())
}

fn confirm_reset() -> Result<bool> {
    use std::io::stdin;

    if !stdin().is_terminal() {
        return Err(PyxError::Validation(
            "Confirmation requires an interactive terminal. Use --yes to skip.".into(),
        ));
    }

    use inquire::Confirm;

    let confirmed = Confirm::new("Are you sure you want to reset?")
        .with_default(false)
        .prompt()
        .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {e}")))?;

    Ok(confirmed)
}

fn delete_file(description: &str, path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| PyxError::Config(format!("Failed to delete {description}: {e}")))?;
        eprintln!("  ✓ Deleted: {description}");
    } else {
        eprintln!("  - Not found: {description}");
    }
    Ok(())
}

fn keyring_has_entry() -> bool {
    crate::keys::keyring::has_entry()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_file() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        fs::write(&path, "test").unwrap();
        assert!(path.exists());

        delete_file("Test file", &path).unwrap();
        assert!(!path.exists());

        assert!(delete_file("Non-existent", &path).is_ok());
    }

    #[test]
    fn test_backup_path() {
        let path = PathBuf::from("/data/database.json");
        let bak = backup_path(&path);
        assert_eq!(bak, PathBuf::from("/data/database.json.bak"));
    }

    #[test]
    fn test_backup_path_no_extension() {
        let path = PathBuf::from("/data/db");
        let bak = backup_path(&path);
        assert_eq!(bak, PathBuf::from("/data/db.bak"));
    }
}
