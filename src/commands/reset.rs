//! Reset command implementation

use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::storage::paths::{
    database_path, master_key_path, models_cache_path, providers_env_path, settings_path,
};
use std::fs;
use std::path::Path;

/// Execute the reset command
pub fn execute() -> Result<()> {
    println!("=== Pyx Reset ===");
    println!();
    println!("WARNING: This will permanently delete all pyx data:");
    println!("  - Master key");
    println!("  - All stored API keys");
    println!("  - Models cache");
    println!("  - Settings");
    println!();

    // Confirm deletion
    if !confirm_reset()? {
        println!("Reset cancelled.");
        return Ok(());
    }

    println!();
    println!("Deleting pyx data...");

    // Delete master key
    delete_file("Master key", &master_key_path()?)?;

    // Delete database
    delete_file("Provider database", &database_path()?)?;
    delete_file(
        "Database backup",
        &database_path()?.with_extension("json.bak"),
    )?;

    // Delete models cache
    delete_file("Models cache", &models_cache_path()?)?;

    // Delete settings
    delete_file("Settings", &settings_path()?)?;
    delete_file(
        "Settings backup",
        &settings_path()?.with_extension("json.bak"),
    )?;

    // Delete providers config
    delete_file("Providers config", &providers_env_path()?)?;

    // Clear keyring
    if KeyManager::master_key_exists() || keyring_has_entry() {
        println!("Clearing keyring entry...");
        KeyManager::clear_passphrase().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to clear keyring: {e}");
        });
    }

    println!();
    println!("✓ Reset complete!");
    println!();
    println!("Pyx has been reset to initial state.");
    println!("Run 'pyx setup' to initialize again.");

    Ok(())
}

/// Confirm reset with user
fn confirm_reset() -> Result<bool> {
    use inquire::Confirm;

    let confirmed = Confirm::new("Are you sure you want to reset?")
        .with_default(false)
        .prompt()
        .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {e}")))?;

    Ok(confirmed)
}

/// Delete a file if it exists
fn delete_file(description: &str, path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| PyxError::Config(format!("Failed to delete {description}: {e}")))?;
        println!("  ✓ Deleted: {description}");
    } else {
        println!("  - Not found: {description}");
    }
    Ok(())
}

/// Check if keyring has an entry
fn keyring_has_entry() -> bool {
    use keyring::Entry;

    // Check pyx entry
    if let Ok(entry) = Entry::new("pyx", "master-key") {
        if entry.get_password().is_ok() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_file() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        // Create file
        fs::write(&path, "test").unwrap();
        assert!(path.exists());

        // Delete it
        delete_file("Test file", &path).unwrap();
        assert!(!path.exists());

        // Delete non-existent (should not error)
        assert!(delete_file("Non-existent", &path).is_ok());
    }
}
