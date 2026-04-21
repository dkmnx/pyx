//! Delete command implementation

use crate::error::{PyxError, Result};
use crate::storage::database::Database;

/// Execute the delete command
pub fn execute(provider_name: Option<&str>, skip_confirm: bool) -> Result<()> {
    let mut db = Database::load_or_error()?;

    if db.is_empty() {
        return Err(PyxError::Config(
            "No providers configured. Run 'pyx add' to add a provider.".to_string(),
        ));
    }

    let provider_name = match provider_name {
        Some(name) => name.to_owned(),
        None => prompt_provider_selection(&db)?,
    };

    if !db.has_provider(&provider_name) {
        return Err(PyxError::ProviderNotFound(format!(
            "Provider '{provider_name}' not found. Run 'pyx list' to see configured providers."
        )));
    }

    if !skip_confirm && !confirm_delete(&provider_name)? {
        eprintln!("Delete cancelled.");
        return Ok(());
    }

    let removed = db.remove(&provider_name).ok_or_else(|| {
        PyxError::ProviderNotFound(format!(
            "Provider '{provider_name}' not found. Run 'pyx list' to see configured providers."
        ))
    })?;

    db.save()?;

    eprintln!("✓ Removed provider: {}", removed.provider);

    Ok(())
}

fn prompt_provider_selection(db: &Database) -> Result<String> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(PyxError::Validation(
            "No provider specified. Pass a provider name or run in an interactive terminal.".into(),
        ));
    }

    let providers = db
        .get_provider_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    crate::prompt::prompt_provider(&providers)
}

fn confirm_delete(provider_name: &str) -> Result<bool> {
    use inquire::Confirm;

    crate::commands::helpers::require_interactive_terminal()?;

    let confirmed = Confirm::new(&format!(
        "Are you sure you want to delete the '{provider_name}' provider?"
    ))
    .with_default(false)
    .prompt()
    .map_err(|e| PyxError::Validation(format!("Failed to read confirmation: {e}")))?;

    Ok(confirmed)
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
    fn test_execute_error_for_empty_database() {
        let dir = tempdir().unwrap();
        let _env = crate::test_helpers::EnvGuard::set_var(
            "XDG_DATA_HOME",
            dir.path().to_string_lossy().to_string(),
        );

        let result = execute(Some("nonexistent"), true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PyxError::Config(_)));
    }

    #[test]
    fn test_execute_error_for_nonexistent_provider() {
        let dir = tempdir().unwrap();
        let _env = crate::test_helpers::EnvGuard::set_var(
            "XDG_DATA_HOME",
            dir.path().to_string_lossy().to_string(),
        );

        let db_path = dir.path().join("pyx").join("database.json");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "other".to_string(),
            "cipher".to_string(),
        ));
        db.save_to_path(&db_path).unwrap();

        let result = execute(Some("nonexistent"), true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PyxError::ProviderNotFound(_)));
    }
}
