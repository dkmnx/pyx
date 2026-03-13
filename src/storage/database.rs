//! Database storage for encrypted provider credentials

use crate::error::{PyxError, Result};
use crate::storage::atomic_write::atomic_write_with_backup;
use crate::storage::paths::database_path;
use serde::{Deserialize, Serialize};
use std::path::Path;
use time::OffsetDateTime;

/// Database entry for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub provider: String,
    pub cipher: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl ProviderEntry {
    /// Create a new provider entry
    pub fn new(provider: String, cipher: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            provider,
            cipher,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the cipher and timestamp
    pub fn update_cipher(&mut self, cipher: String) {
        self.cipher = cipher;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

/// Database containing all provider entries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Database {
    pub providers: Vec<ProviderEntry>,
}

impl Database {
    /// Load database from file
    pub fn load() -> Result<Self> {
        let path = database_path()?;
        Self::load_from_path(&path)
    }

    /// Load database from specific path (for testing)
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(PyxError::Config("Database file does not exist".to_string()));
        }

        let content = std::fs::read_to_string(path)?;

        // Preferred format: { "providers": [...] }
        if let Ok(database) = serde_json::from_str::<Self>(&content) {
            return Ok(database);
        }

        // Compatibility format (Go): top-level provider array
        let providers: Vec<ProviderEntry> = serde_json::from_str(&content)?;
        Ok(Self { providers })
    }

    /// Save database to file with atomic write and backup
    pub fn save(&self) -> Result<()> {
        let path = database_path()?;
        self.save_to_path(&path)
    }

    /// Save database to specific path (for testing)
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write in Go-compatible top-level array format.
        let content = serde_json::to_string_pretty(&self.providers)?;
        atomic_write_with_backup(path, content.as_bytes(), 0o600)?;
        Ok(())
    }

    /// Add or update a provider entry
    pub fn upsert(&mut self, entry: ProviderEntry) {
        // Remove existing entry for this provider
        self.providers.retain(|p| p.provider != entry.provider);
        // Add new entry
        self.providers.push(entry);
    }

    /// Remove a provider by name
    pub fn remove(&mut self, provider_name: &str) -> Option<ProviderEntry> {
        if let Some(pos) = self
            .providers
            .iter()
            .position(|p| p.provider == provider_name)
        {
            Some(self.providers.remove(pos))
        } else {
            None
        }
    }

    /// Get a provider entry by name
    pub fn get(&self, provider_name: &str) -> Option<&ProviderEntry> {
        self.providers.iter().find(|p| p.provider == provider_name)
    }

    /// Get all provider names
    pub fn get_provider_names(&self) -> Vec<&String> {
        self.providers.iter().map(|p| &p.provider).collect()
    }

    /// Check if database has a provider
    pub fn has_provider(&self, provider_name: &str) -> bool {
        self.providers.iter().any(|p| p.provider == provider_name)
    }

    /// Get number of providers
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_database() {
        let db = Database::default();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_upsert_provider() {
        let mut db = Database::default();

        let entry = ProviderEntry::new("openai".to_string(), "cipher1".to_string());
        db.upsert(entry);

        assert_eq!(db.len(), 1);
        assert!(db.has_provider("openai"));

        // Update should not create duplicate
        let entry2 = ProviderEntry::new("openai".to_string(), "cipher2".to_string());
        db.upsert(entry2);

        assert_eq!(db.len(), 1);
        assert_eq!(db.get("openai").unwrap().cipher, "cipher2");
    }

    #[test]
    fn test_remove_provider() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "cipher1".to_string(),
        ));
        db.upsert(ProviderEntry::new(
            "anthropic".to_string(),
            "cipher2".to_string(),
        ));

        let removed = db.remove("openai");
        assert!(removed.is_some());
        assert_eq!(db.len(), 1);
        assert!(!db.has_provider("openai"));

        // Remove non-existent should return None
        let removed2 = db.remove("nonexistent");
        assert!(removed2.is_none());
    }

    #[test]
    fn test_save_and_load_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("database.json");

        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "test-cipher".to_string(),
        ));

        db.save_to_path(&path).unwrap();

        let loaded = Database::load_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.has_provider("openai"));
    }

    #[test]
    fn test_load_nonexistent_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");

        let result = Database::load_from_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_go_compat_array_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("database.json");

        let content = r#"[
  {
    "provider": "openai",
    "cipher": "cipher",
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]"#;

        std::fs::write(&path, content).unwrap();

        let db = Database::load_from_path(&path).unwrap();
        assert_eq!(db.len(), 1);
        assert!(db.has_provider("openai"));
    }

    #[test]
    fn test_load_object_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("database.json");

        let content = r#"{
  "providers": [
    {
      "provider": "anthropic",
      "cipher": "cipher",
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ]
}"#;

        std::fs::write(&path, content).unwrap();

        let db = Database::load_from_path(&path).unwrap();
        assert_eq!(db.len(), 1);
        assert!(db.has_provider("anthropic"));
    }
}
