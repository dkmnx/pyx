//! Database storage for encrypted provider credentials

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use time::OffsetDateTime;

/// Database entry for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub provider: String,
    pub cipher: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Database containing all provider entries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Database {
    pub providers: Vec<ProviderEntry>,
}

impl Database {
    /// Load database from file
    pub fn load(path: &Path) -> Result<Self> {
        // TODO: Implement database loading
        let _ = path;
        Err(crate::error::PyxError::Config(
            "Database::load not yet implemented".to_string(),
        ))
    }

    /// Save database to file with atomic write and backup
    pub fn save(&self, path: &Path) -> Result<()> {
        // TODO: Implement database saving
        let _ = path;
        Err(crate::error::PyxError::Config(
            "Database::save not yet implemented".to_string(),
        ))
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
        if let Some(pos) = self.providers.iter().position(|p| p.provider == provider_name) {
            Some(self.providers.remove(pos))
        } else {
            None
        }
    }
}
