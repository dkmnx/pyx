//! Models command implementation

use crate::error::{PyxError, Result};
use crate::models::fetch::fetch_models_from_remote;
use crate::storage::models_cache::ModelsCache;
use std::collections::BTreeMap;

/// Default cache TTL in seconds (24 hours)
const DEFAULT_TTL_SECONDS: i64 = 24 * 60 * 60;

/// Execute the models command
pub fn execute(json: bool, refresh: bool, provider: Option<&str>) -> Result<()> {
    let cache = load_cache_or_error(refresh, json)?;

    if refresh || cache.is_stale(DEFAULT_TTL_SECONDS) {
        return handle_refresh(&cache, json, refresh, provider);
    }

    print_models(&cache, json, provider)
}

fn load_cache_or_error(refresh: bool, json: bool) -> Result<ModelsCache> {
    match ModelsCache::load() {
        Ok(cache) => Ok(cache),
        Err(PyxError::Config(_)) => {
            if refresh {
                Err(PyxError::Config(
                    "No models cache found. Run 'pyx models update' first.".to_string(),
                ))
            } else {
                if !json {
                    eprintln!("No models cache found. Run 'pyx models update' to fetch models.");
                }
                Err(PyxError::Cancelled)
            }
        }
        Err(e) => Err(e),
    }
}

fn handle_refresh(
    stale_cache: &ModelsCache,
    json: bool,
    explicit_refresh: bool,
    provider: Option<&str>,
) -> Result<()> {
    if explicit_refresh {
        eprintln!("Refreshing models cache...");
    } else if !json {
        eprintln!("Cache is stale, fetching updated models...");
    }

    match fetch_models_from_remote() {
        Ok(new_cache) => {
            new_cache.save()?;
            if !json {
                eprintln!("Models cache updated.");
            }
            print_models(&new_cache, json, provider)
        }
        Err(e) => {
            if explicit_refresh {
                return Err(e);
            }
            eprintln!("Warning: Failed to fetch updated models: {e}");
            eprintln!(
                "Using cached models (last updated: {})",
                stale_cache.updated_at
            );
            print_models(stale_cache, json, provider)
        }
    }
}

fn print_models(cache: &ModelsCache, json: bool, provider_filter: Option<&str>) -> Result<()> {
    if let Some(provider) = provider_filter {
        if !cache.models.contains_key(provider) {
            return Err(PyxError::Config(format!("Provider '{provider}' not found")));
        }
    }

    if json {
        return print_models_json(cache, provider_filter);
    }

    print_models_text(cache, provider_filter)
}

fn filtered_models<'a>(
    cache: &'a ModelsCache,
    provider_filter: Option<&str>,
) -> BTreeMap<&'a str, &'a [String]> {
    cache
        .models
        .iter()
        .filter(|(provider, _)| match provider_filter {
            Some(filter) => provider.as_str() == filter,
            None => true,
        })
        .map(|(provider, models)| (provider.as_str(), models.as_slice()))
        .collect()
}

fn print_models_json(cache: &ModelsCache, provider_filter: Option<&str>) -> Result<()> {
    let output = serde_json::json!({
        "version": cache.version,
        "updated_at": cache.updated_at,
        "models": filtered_models(cache, provider_filter),
    });
    println!("{output}");
    Ok(())
}

fn print_models_text(cache: &ModelsCache, provider_filter: Option<&str>) -> Result<()> {
    println!("Supported models:");
    println!();

    let providers = filtered_models(cache, provider_filter);

    for (provider, models) in &providers {
        if models.is_empty() {
            continue;
        }

        println!("  {} ({} models)", provider, models.len());
        for model in *models {
            println!("    - {model}");
        }
        println!();
    }

    let total_models: usize = providers.values().map(|models| models.len()).sum();

    println!(
        "Total: {} providers, {} models",
        providers.len(),
        total_models
    );

    Ok(())
}

/// Execute models update command
pub fn execute_update() -> Result<()> {
    println!("Fetching models from remote...");

    let cache = fetch_models_from_remote()
        .map_err(|e| PyxError::Network(format!("Failed to fetch models: {e}")))?;

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
