//! Models command implementation

use crate::error::{PyxError, Result};
use crate::storage::models_cache::ModelsCache;
use crate::models::fetch::fetch_models_from_remote;

/// Default cache TTL in seconds (24 hours)
const DEFAULT_TTL_SECONDS: i64 = 24 * 60 * 60;

/// Execute the models command
pub fn execute(json: bool, refresh: bool) -> Result<()> {
    // Try to load cached models
    let cache = match ModelsCache::load() {
        Ok(c) => c,
        Err(PyxError::Config(_)) => {
            if refresh {
                return Err(PyxError::Config(
                    "No models cache found. Run 'pyx models update' first.".to_string(),
                ));
            }
            println!("No models cache found. Run 'pyx models update' to fetch models.");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Check if refresh is needed
    if refresh || cache.is_stale(DEFAULT_TTL_SECONDS) {
        if refresh {
            println!("Refreshing models cache...");
        } else {
            println!("Cache is stale, fetching updated models...");
        }
        
        match fetch_models_from_remote() {
            Ok(new_cache) => {
                new_cache.save()?;
                println!("Models cache updated.");
                return print_models(&new_cache, json);
            }
            Err(e) => {
                if refresh {
                    return Err(e);
                }
                // If not explicit refresh, fall back to stale cache
                eprintln!("Warning: Failed to fetch updated models: {}", e);
                eprintln!("Using cached models (last updated: {})", cache.updated_at);
            }
        }
    }

    print_models(&cache, json)
}

/// Print models in text or JSON format
fn print_models(cache: &ModelsCache, json: bool) -> Result<()> {
    if json {
        let output = serde_json::json!({
            "version": cache.version,
            "updated_at": cache.updated_at,
            "models": cache.models,
        });
        println!("{}", output);
    } else {
        println!("Models cache (version: {})", cache.version);
        println!("Last updated: {}", cache.updated_at);
        println!();
        
        let mut providers: Vec<_> = cache.models.keys().collect();
        providers.sort();
        
        for provider in providers {
            let models = cache.models.get(provider).unwrap();
            println!("{}:", provider);
            for model in models {
                println!("  - {}", model);
            }
        }
        println!();
        println!("Total: {} provider(s)", cache.models.len());
    }
    
    Ok(())
}

/// Execute models update command
pub fn execute_update() -> Result<()> {
    println!("Fetching models from remote...");
    
    let cache = fetch_models_from_remote()
        .map_err(|e| PyxError::Network(format!("Failed to fetch models: {}", e)))?;
    
    cache.save()?;
    println!("Models cache updated successfully.");
    println!("Version: {}", cache.version);
    println!("Providers: {}", cache.models.len());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_models_cache_creation() {
        let mut cache = ModelsCache::new("v1.0.0");
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string()]);
        
        assert_eq!(cache.version, "v1.0.0");
        assert_eq!(cache.models.len(), 1);
    }
}
