//! Models cache storage

use crate::error::Result;
use crate::storage::atomic_write::atomic_write_with_backup;
use crate::storage::paths::models_cache_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;

/// Current cache format version. Increment when the schema changes to ensure
/// compatibility with older cached data.
const CACHE_FORMAT_VERSION: &str = "1";

/// Default TTL for models cache (24 hours)
const DEFAULT_TTL_SECONDS: i64 = 24 * 60 * 60;

/// Models cache structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsCache {
    pub version: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub models: HashMap<String, Vec<String>>,
    /// Cache format version for schema compatibility.
    /// Defaults to "1" for backward compatibility with existing caches.
    #[serde(default)]
    pub cache_format_version: String,
}

impl ModelsCache {
    /// Create a new empty models cache
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            updated_at: OffsetDateTime::now_utc(),
            models: HashMap::new(),
            cache_format_version: CACHE_FORMAT_VERSION.to_string(),
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

        // Check format version compatibility.
        // - Missing field (empty string) → allowed for backward compat with v0 caches
        // - "1" → current format
        // - anything else → outdated, needs refresh
        if !cache.cache_format_version.is_empty()
            && cache.cache_format_version != CACHE_FORMAT_VERSION
        {
            return Err(crate::error::PyxError::Config(
                format!(
                    "Models cache format version mismatch (found {}, expected {}). Run 'pyx models update' to refresh.",
                    cache.cache_format_version, CACHE_FORMAT_VERSION
                )
            ));
        }

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

    /// Check if cache is stale (older than or exactly at TTL)
    ///
    /// A cache is considered stale when `age >= TTL`. This differs from the
    /// previous `age.whole_seconds() > ttl_seconds` in two ways:
    ///
    /// 1. **Boundary flip:** A cache whose age equals the TTL is now stale.
    ///    Previously it was fresh (the `>` operator excluded the boundary).
    ///
    /// 2. **Sub-second precision:** The old `whole_seconds()` truncated
    ///    fractional seconds, so a cache aged 86400.9 s was *not* stale at
    ///    TTL=86400 (since 86400 > 86400 is false). Now it *is* stale.
    ///    This is the more impactful change: caches that previously stayed
    ///    fresh for nearly one extra second (up to 0.999 s) after crossing
    ///    the TTL boundary are now immediately stale. This affects cache
    ///    hit rates and may surprise callers that relied on the old window.
    ///
    /// Both changes are intentional corrections — the old behavior was
    /// arguably a bug — but they constitute a semantic change.
    pub fn is_stale(&self, ttl_seconds: i64) -> bool {
        let now = OffsetDateTime::now_utc();
        let age = now - self.updated_at;
        age >= time::Duration::seconds(ttl_seconds)
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
    pub fn get_models(&self, provider: &str) -> Option<&[String]> {
        self.models.get(provider).map(Vec::as_slice)
    }

    /// Get all providers in cache
    pub fn get_providers(&self) -> impl Iterator<Item = &str> {
        self.models.keys().map(String::as_str)
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
        cache.upsert_models(
            "openai".to_string(),
            vec!["gpt-4".to_string(), "gpt-3.5".to_string()],
        );
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
        cache.upsert_models(
            "openai".to_string(),
            vec!["gpt-4".to_string(), "gpt-5".to_string()],
        );
        assert_eq!(cache.get_models("openai").unwrap().len(), 2);
    }

    #[test]
    fn test_stale_exactly_at_ttl() {
        let mut cache = ModelsCache::new("v1.0.0");
        cache.updated_at = OffsetDateTime::now_utc() - time::Duration::seconds(86400);
        assert!(
            cache.is_stale(86400),
            "cache should be stale at exactly TTL"
        );
    }

    #[test]
    fn test_stale_sub_second_precision() {
        // A cache aged TTL + 0.5s should be stale (the old whole_seconds()
        // truncation would have made it fresh: 86400 > 86400 == false).
        let mut cache = ModelsCache::new("v1.0.0");
        cache.updated_at = OffsetDateTime::now_utc()
            - time::Duration::seconds(86400)
            - time::Duration::milliseconds(500);
        assert!(
            cache.is_stale(86400),
            "cache aged TTL + 0.5s should be stale (sub-second precision)"
        );

        // A cache aged TTL - 0.5s should still be fresh.
        let mut cache2 = ModelsCache::new("v1.0.0");
        cache2.updated_at = OffsetDateTime::now_utc() - time::Duration::seconds(86400)
            + time::Duration::milliseconds(500);
        assert!(
            !cache2.is_stale(86400),
            "cache aged TTL - 0.5s should still be fresh"
        );
    }

    #[test]
    fn test_new_cache_has_version_field() {
        let cache = ModelsCache::new("v1.0.0");
        assert_eq!(cache.cache_format_version, "1");
    }

    #[test]
    fn test_save_and_load_preserves_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");

        let cache = ModelsCache::new("v1.0.0");
        cache.save_to_path(&path).unwrap();

        let loaded = ModelsCache::load_from_path(&path).unwrap();
        assert_eq!(loaded.cache_format_version, "1");
    }

    #[test]
    fn test_load_old_cache_without_version_still_works() {
        // Old caches without the cache_format_version field deserialize with
        // the serde default (empty string), which is allowed for backward compat.
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");

        let old_content = r#"{
          "version": "v0.5.0",
          "updated_at": "2026-01-01T00:00:00Z",
          "models": {}
        }"#;
        std::fs::write(&path, old_content).unwrap();

        // Empty version string is allowed (backward compat with v0)
        let cache = ModelsCache::load_from_path(&path).unwrap();
        assert_eq!(cache.version, "v0.5.0");
        assert_eq!(cache.cache_format_version, "");
    }

    #[test]
    fn test_load_future_version_cache_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");

        // Simulate a cache from a future version with version "99"
        let future_content = r#"{
          "version": "v99.0.0",
          "updated_at": "2026-01-01T00:00:00Z",
          "models": {},
          "cache_format_version": "99"
        }"#;
        std::fs::write(&path, future_content).unwrap();

        let err = ModelsCache::load_from_path(&path).unwrap_err();
        assert!(
            matches!(err, crate::error::PyxError::Config(_)),
            "mismatched version should return Config error"
        );
        let msg = format!("{err}");
        assert!(
            msg.contains("99") && msg.contains("1"),
            "error message should mention both versions"
        );
    }
}
