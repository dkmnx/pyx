//! Delete command implementation

use crate::error::{PyxError, Result};
use crate::storage::database::Database;

/// Execute the delete command
pub fn execute(provider_name: &str, skip_confirm: bool) -> Result<()> {
    let mut db = Database::load_or_error()?;

    if !db.has_provider(provider_name) {
        return Err(PyxError::ProviderNotFound(format!(
            "Provider '{provider_name}' not found. Run 'pyx list' to see configured providers."
        )));
    }

    if !skip_confirm && !confirm_delete(provider_name)? {
        println!("Delete cancelled.");
        return Ok(());
    }

    let removed = db.remove(provider_name).ok_or_else(|| {
        PyxError::ProviderNotFound(format!(
            "Provider '{provider_name}' not found. Run 'pyx list' to see configured providers."
        ))
    })?;

    db.save()?;

    println!("✓ Removed provider: {}", removed.provider);

    Ok(())
}

fn confirm_delete(provider_name: &str) -> Result<bool> {
    use inquire::Confirm;
    use std::io::IsTerminal;

    if !stdin().is_terminal() {
        return Err(PyxError::Validation(
            "Confirmation requires an interactive terminal. Use --yes to skip.".into(),
        ));
    }

    let confirmed = Confirm::new(&format!(
        "Are you sure you want to delete the '{provider_name}' provider?"
    ))
    .with_default(false)
    .prompt()
    .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {e}")))?;

    Ok(confirmed)
}

fn stdin() -> std::io::Stdin {
    std::io::stdin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::ProviderEntry;
    use tempfile::tempdir;

    #[test]
    fn test_delete_provider() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");

        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "test-provider".to_string(),
            "cipher".to_string(),
        ));
        db.save_to_path(&db_path).unwrap();

        assert!(db.has_provider("test-provider"));

        db.remove("test-provider");
        assert!(!db.has_provider("test-provider"));
    }

    #[test]
    fn test_remove_returns_entry() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "my-provider".to_string(),
            "cipher-key".to_string(),
        ));

        let removed = db.remove("my-provider");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().provider, "my-provider");
    }

    #[test]
    fn test_remove_nonexistent_returns_none() {
        let mut db = Database::default();
        assert!(db.remove("nonexistent").is_none());
    }
}
