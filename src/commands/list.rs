//! List command implementation

use crate::error::Result;
use crate::storage::database::Database;

/// Execute the list command
pub fn execute() -> Result<()> {
    let db = Database::load_or_error()?;

    if db.is_empty() {
        eprintln!("No providers configured. Run 'pyx add' to add providers.");
        return Ok(());
    }

    eprintln!("Configured providers:");
    for provider in db.get_provider_names() {
        eprintln!("  - {provider}");
    }
    eprintln!();
    eprintln!("Total: {} provider(s)", db.len());

    Ok(())
}

/// Execute list command with JSON output
pub fn execute_json() -> Result<()> {
    let db = Database::load_or_error()?;

    let providers = db.get_provider_names();
    let output = serde_json::json!({
        "providers": providers,
        "count": db.len(),
    });
    println!("{output}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::ProviderEntry;
    use tempfile::tempdir;

    #[test]
    fn test_list_with_providers() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");

        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "cipher".to_string(),
        ));
        db.upsert(ProviderEntry::new(
            "anthropic".to_string(),
            "cipher".to_string(),
        ));
        db.save_to_path(&db_path).unwrap();

        // Test database operations work correctly
        assert_eq!(db.len(), 2);
        assert!(db.has_provider("openai"));
        assert!(db.has_provider("anthropic"));
        assert!(!db.has_provider("nonexistent"));

        // Test provider names are retrievable
        let names = db.get_provider_names();
        assert!(names.contains(&"openai"));
        assert!(names.contains(&"anthropic"));
    }

    #[test]
    fn test_list_empty_database() {
        let db = Database::default();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert!(!db.has_provider("anything"));
    }

    #[test]
    fn test_database_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");

        let mut db = Database::default();
        db.upsert(ProviderEntry::new("test".to_string(), "cipher".to_string()));
        db.save_to_path(&db_path).unwrap();

        let loaded = Database::load_from_path(&db_path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.has_provider("test"));
    }

    #[test]
    fn test_database_upsert_updates_existing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");

        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "test".to_string(),
            "cipher1".to_string(),
        ));
        db.save_to_path(&db_path).unwrap();

        db.upsert(ProviderEntry::new(
            "test".to_string(),
            "cipher2".to_string(),
        ));
        db.save_to_path(&db_path).unwrap();

        let loaded = Database::load_from_path(&db_path).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_database_get_provider_names_sorted() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("zebra".to_string(), "c".to_string()));
        db.upsert(ProviderEntry::new("apple".to_string(), "a".to_string()));
        db.upsert(ProviderEntry::new("mango".to_string(), "b".to_string()));

        let names = db.get_provider_names();
        assert_eq!(names, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn test_database_len_after_multiple_upserts() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("a".to_string(), "1".to_string()));
        db.upsert(ProviderEntry::new("b".to_string(), "2".to_string()));

        assert_eq!(db.len(), 2);
        assert_eq!(db.get_provider_names().len(), 2);
    }
}
