//! Models command implementation

use crate::error::{PyxError, Result};
use crate::models::fetch::fetch_models_from_remote;
use crate::storage::models_cache::ModelsCache;

/// Default cache TTL in seconds (24 hours)
const DEFAULT_TTL_SECONDS: i64 = 24 * 60 * 60;

/// Execute the models command
pub fn execute(json: bool, refresh: bool, provider: Option<&str>) -> Result<()> {
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
                return print_models(&new_cache, json, provider);
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

    print_models(&cache, json, provider)
}

/// Print models in text or JSON format
fn print_models(cache: &ModelsCache, json: bool, provider_filter: Option<&str>) -> Result<()> {
    // Validate provider filter if specified
    if let Some(provider) = provider_filter {
        if !cache.models.contains_key(provider) {
            return Err(PyxError::Config(format!(
                "Provider '{}' not found",
                provider
            )));
        }
    }

    if json {
        let filtered_models: std::collections::HashMap<_, _> =
            if let Some(provider) = provider_filter {
                cache
                    .models
                    .iter()
                    .filter(|(k, _)| *k == provider)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                cache.models.clone()
            };

        let output = serde_json::json!({
            "version": cache.version,
            "updated_at": cache.updated_at,
            "models": filtered_models,
        });
        println!("{}", output);
    } else {
        println!("Supported models:");
        println!();

        let mut providers: Vec<_> = cache.models.keys().collect();
        providers.sort();

        // Filter providers if specified
        let providers: Vec<_> = if let Some(provider) = provider_filter {
            providers.into_iter().filter(|p| *p == provider).collect()
        } else {
            providers
        };

        for provider in &providers {
            let models = cache.models.get(*provider)
                .expect("provider should exist after validation check");
            if models.is_empty() {
                continue;
            }
            println!("  {} ({} models)", provider, models.len());
            for model in models {
                println!("    - {}", model);
            }
            println!();
        }

        let total_models: usize = providers
            .iter()
            .map(|p| cache.models.get(*p).map(|m| m.len()).unwrap_or(0))
            .sum();
        println!(
            "Total: {} providers, {} models",
            providers.len(),
            total_models
        );
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
