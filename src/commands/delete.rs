//! Delete command implementation

use crate::error::{PyxError, Result};
use crate::storage::database::Database;

/// Execute the delete command
pub fn execute(provider_name: &str) -> Result<()> {
    // Load database
    let mut db = Database::load().map_err(|e| match e {
        PyxError::Config(_) => {
            PyxError::Config("No providers configured. Run 'pyx setup' first.".to_string())
        }
        _ => e,
    })?;

    // Check if provider exists
    if !db.has_provider(provider_name) {
        return Err(PyxError::ProviderNotFound(format!(
            "Provider '{}' not found. Run 'pyx list' to see configured providers.",
            provider_name
        )));
    }

    // Remove provider
    let removed = db.remove(provider_name)
        .expect("provider should exist after has_provider check");

    // Save database
    db.save()?;

    println!("✓ Removed provider: {}", removed.provider);
    println!("  Created: {}", removed.created_at);
    println!("  Updated: {}", removed.updated_at);

    Ok(())
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

        // Would need to override database_path() for full testing
        assert!(db.has_provider("test-provider"));

        db.remove("test-provider");
        assert!(!db.has_provider("test-provider"));
    }
}
