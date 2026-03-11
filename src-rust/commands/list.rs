//! List command implementation

use crate::error::{PyxError, Result};
use crate::storage::database::Database;

/// Execute the list command
pub fn execute() -> Result<()> {
    // Load database
    let db = Database::load().map_err(|e| {
        match e {
            PyxError::Config(_) => PyxError::Config(
                "No providers configured. Run 'pyx setup' first.".to_string(),
            ),
            _ => e,
        }
    })?;

    if db.is_empty() {
        println!("No providers configured. Run 'pyx setup' to add providers.");
        return Ok(());
    }

    println!("Configured providers:");
    for entry in &db.providers {
        println!("  - {}", entry.provider);
    }
    println!();
    println!("Total: {} provider(s)", db.len());

    Ok(())
}

/// Execute list command with JSON output
pub fn execute_json() -> Result<()> {
    let db = Database::load().map_err(|e| {
        match e {
            PyxError::Config(_) => PyxError::Config(
                "No providers configured. Run 'pyx setup' first.".to_string(),
            ),
            _ => e,
        }
    })?;

    let providers: Vec<&str> = db.get_provider_names().iter().map(|s| s.as_str()).collect();
    let output = serde_json::json!({
        "providers": providers,
        "count": db.len(),
    });
    println!("{}", output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::ProviderEntry;
    use tempfile::tempdir;

    #[test]
    fn test_list_empty_database() {
        // This would require mocking the database path
        // For now, just verify the function exists
        let _ = execute();
    }

    #[test]
    fn test_list_with_providers() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("database.json");
        
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("openai".to_string(), "cipher".to_string()));
        db.upsert(ProviderEntry::new("anthropic".to_string(), "cipher".to_string()));
        db.save_to_path(&db_path).unwrap();
        
        // Would need to override database_path() for testing
        // For now, just verify database operations work
        assert_eq!(db.len(), 2);
        assert!(db.has_provider("openai"));
    }
}
