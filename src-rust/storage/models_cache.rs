//! Models cache storage

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use time::OffsetDateTime;

/// Models cache structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsCache {
    pub version: String,
    pub updated_at: OffsetDateTime,
    pub models: std::collections::HashMap<String, Vec<String>>,
}

impl ModelsCache {
    /// Load models cache from file
    pub fn load(path: &Path) -> Result<Self> {
        // TODO: Implement models cache loading
        let _ = path;
        Err(crate::error::PyxError::Config(
            "ModelsCache::load not yet implemented".to_string(),
        ))
    }

    /// Save models cache to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // TODO: Implement models cache saving
        let _ = path;
        Err(crate::error::PyxError::Config(
            "ModelsCache::save not yet implemented".to_string(),
        ))
    }

    /// Check if cache is stale (older than TTL)
    pub fn is_stale(&self, ttl_seconds: i64) -> bool {
        let now = OffsetDateTime::now_utc();
        let age = now - self.updated_at;
        age.whole_seconds() > ttl_seconds
    }
}
