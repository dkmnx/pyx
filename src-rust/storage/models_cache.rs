//! Models cache storage

use crate::error::Result;
use crate::storage::atomic_write::atomic_write_with_backup;
use crate::storage::paths::models_cache_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;

/// Default TTL for models cache (24 hours)
const DEFAULT_TTL_SECONDS: i64 = 24 * 60 * 60;

/// Models cache structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsCache {
    pub version: String,
    pub updated_at: OffsetDateTime,
    pub models: HashMap<String, Vec<String>>,
}

impl ModelsCache {
    /// Create a new empty models cache
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            updated_at: OffsetDateTime::now_utc(),
            models: HashMap::new(),
        }
    }

    /// Load models cache from file
    pub fn load() -> Result<Self> {
        let path = models_cache_path()?;
        Self::load_from_path(&path)
    }

    /// Load models cache from specific path (for testing)
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(crate::error::PyxError::Config(
                "Models cache file does not exist".to_string(),
            ));
        }

        let content = std::fs::read_to_string(path)?;
        let cache: Self = serde_json::from_str(&content)?;
        Ok(cache)
    }

    /// Save models cache to file
    pub fn save(&self) -> Result<()> {
        let path = models_cache_path()?;
        self.save_to_path(&path)
    }

    /// Save models cache to specific path (for testing)
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        atomic_write_with_backup(path, content.as_bytes(), 0o600)?;
        Ok(())
    }

    /// Check if cache is stale (older than TTL)
    pub fn is_stale(&self, ttl_seconds: i64) -> bool {
        let now = OffsetDateTime::now_utc();
        let age = now - self.updated_at;
        age.whole_seconds() > ttl_seconds
    }

    /// Check if cache is stale using default TTL
    pub fn is_stale_default(&self) -> bool {
        self.is_stale(DEFAULT_TTL_SECONDS)
    }

    /// Add or update models for a provider
    pub fn upsert_models(&mut self, provider: String, models: Vec<String>) {
        self.models.insert(provider, models);
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Get models for a provider
    pub fn get_models(&self, provider: &str) -> Option<&Vec<String>> {
        self.models.get(provider)
    }

    /// Get all providers in cache
    pub fn get_providers(&self) -> impl Iterator<Item = &String> {
        self.models.keys()
    }
}

impl Default for ModelsCache {
    fn default() -> Self {
        Self::new("unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_cache() {
        let cache = ModelsCache::new("v1.0.0");
        assert_eq!(cache.version, "v1.0.0");
        assert!(cache.models.is_empty());
    }

    #[test]
    fn test_save_and_load_cache() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        
        let mut cache = ModelsCache::new("v1.0.0");
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string(), "gpt-3.5".to_string()]);
        cache.upsert_models("anthropic".to_string(), vec!["claude-3".to_string()]);
        
        cache.save_to_path(&path).unwrap();
        
        let loaded = ModelsCache::load_from_path(&path).unwrap();
        assert_eq!(loaded.version, "v1.0.0");
        assert_eq!(loaded.models.len(), 2);
        assert!(loaded.get_models("openai").is_some());
    }

    #[test]
    fn test_cache_staleness() {
        let mut cache = ModelsCache::new("v1.0.0");
        
        // Fresh cache should not be stale with 1 hour TTL
        assert!(!cache.is_stale(3600));
        
        // Manually set old timestamp
        cache.updated_at = OffsetDateTime::now_utc() - time::Duration::days(2);
        
        // Now it should be stale with 1 day TTL
        assert!(cache.is_stale(86400));
    }

    #[test]
    fn test_upsert_models() {
        let mut cache = ModelsCache::new("v1.0.0");
        
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string()]);
        assert_eq!(cache.get_models("openai").unwrap().len(), 1);
        
        // Update should replace
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string(), "gpt-5".to_string()]);
        assert_eq!(cache.get_models("openai").unwrap().len(), 2);
    }
}
