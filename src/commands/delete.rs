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

    #[test]
    fn test_database_remove_is_idempotent() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("test".to_string(), "cipher".to_string()));

        let first = db.remove("test");
        let second = db.remove("test");

        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn test_database_remove_after_save() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");

        let mut db = Database::default();
        db.upsert(ProviderEntry::new("test".to_string(), "cipher".to_string()));
        db.save_to_path(&db_path).unwrap();

        db.remove("test");
        db.save_to_path(&db_path).unwrap();

        let loaded = Database::load_from_path(&db_path).unwrap();
        assert!(!loaded.has_provider("test"));
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_database_remove_multiple_providers() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("a".to_string(), "1".to_string()));
        db.upsert(ProviderEntry::new("b".to_string(), "2".to_string()));
        db.upsert(ProviderEntry::new("c".to_string(), "3".to_string()));

        db.remove("b");
        assert_eq!(db.len(), 2);
        assert!(db.has_provider("a"));
        assert!(!db.has_provider("b"));
        assert!(db.has_provider("c"));

        db.remove("a");
        assert_eq!(db.len(), 1);
        assert!(db.has_provider("c"));
    }

    #[test]
    fn test_execute_error_for_nonexistent_provider() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");

        let db = Database::default();
        db.save_to_path(&db_path).unwrap();

        let result = execute("nonexistent", true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PyxError::ProviderNotFound(_)));
    }
}
