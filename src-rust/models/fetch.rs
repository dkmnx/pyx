//! Fetch models from remote source

use crate::error::Result;
use crate::storage::models_cache::ModelsCache;

/// Fetch models from pi-mono or other remote source
pub fn fetch_models_from_remote() -> Result<ModelsCache> {
    // TODO: Implement actual model fetching from pi-mono
    // For now, return a placeholder with common providers
    
    let mut cache = ModelsCache::new("v1.0.0-placeholder");
    
    // Add common providers as placeholder
    cache.upsert_models("openai".to_string(), vec![
        "openai/gpt-4".to_string(),
        "openai/gpt-4-turbo".to_string(),
        "openai/gpt-3.5-turbo".to_string(),
    ]);
    
    cache.upsert_models("anthropic".to_string(), vec![
        "anthropic/claude-3-opus".to_string(),
        "anthropic/claude-3-sonnet".to_string(),
        "anthropic/claude-3-haiku".to_string(),
    ]);
    
    cache.upsert_models("google".to_string(), vec![
        "google/gemini-pro".to_string(),
        "google/gemini-ultra".to_string(),
    ]);
    
    Ok(cache)
}

/// Get the latest pyx/pi version from GitHub releases
pub fn get_latest_version() -> Result<String> {
    // TODO: Implement version check from GitHub API
    // For now, return placeholder
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_models() {
        let cache = fetch_models_from_remote().unwrap();
        assert_eq!(cache.version, "v1.0.0-placeholder");
        assert!(cache.models.contains_key("openai"));
        assert!(cache.models.contains_key("anthropic"));
    }
}
