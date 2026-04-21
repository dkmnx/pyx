//! Models command implementation

use crate::error::{PyxError, Result};
use crate::models::fetch::fetch_models_from_remote;
use crate::storage::models_cache::ModelsCache;
use std::collections::BTreeMap;

/// Arguments for the models command.
pub struct ModelsCommandArgs<'a> {
    pub json: bool,
    pub refresh: bool,
    pub provider: Option<&'a str>,
}

/// Execute the models command
pub fn execute(args: ModelsCommandArgs) -> Result<()> {
    let cache = load_cache_or_error(&args)?;

    if args.refresh || cache.is_stale_default() {
        return handle_refresh(&cache, &args);
    }

    print_models(&cache, &args, false)
}

fn load_cache_or_error(args: &ModelsCommandArgs) -> Result<ModelsCache> {
    match ModelsCache::load() {
        Ok(cache) => Ok(cache),
        Err(PyxError::Config(_)) => {
            if args.refresh {
                Err(PyxError::Config(
                    "No models cache found. Run 'pyx models update' first.".to_string(),
                ))
            } else {
                if !args.json {
                    eprintln!("No models cache found. Run 'pyx models update' to fetch models.");
                }
                Ok(ModelsCache::new("empty"))
            }
        }
        Err(e) => Err(e),
    }
}

fn handle_refresh(stale_cache: &ModelsCache, args: &ModelsCommandArgs) -> Result<()> {
    if args.refresh {
        eprintln!("Refreshing models cache...");
    } else if !args.json {
        eprintln!("Cache is stale, fetching updated models...");
    }

    match fetch_models_from_remote() {
        Ok(new_cache) => {
            new_cache.save()?;
            if !args.json {
                eprintln!("Models cache updated.");
            }
            print_models(&new_cache, args, false)
        }
        Err(e) => {
            if args.refresh {
                return Err(e);
            }
            if !args.json {
                eprintln!("Warning: Failed to fetch updated models: {e}");
                eprintln!(
                    "Using cached models (last updated: {})",
                    stale_cache.updated_at
                );
            } else {
                eprintln!("Warning: Failed to fetch updated models: {e}");
                eprintln!(
                    "Using cached models (last updated: {}). Output contains stale=true.",
                    stale_cache.updated_at
                );
            }
            print_models(stale_cache, args, true)
        }
    }
}

fn print_models(cache: &ModelsCache, args: &ModelsCommandArgs, stale: bool) -> Result<()> {
    if let Some(provider) = args.provider {
        if !cache.models.contains_key(provider) {
            return Err(PyxError::Config(format!("Provider '{provider}' not found")));
        }
    }

    if args.json {
        return print_models_json(cache, args.provider, stale);
    }

    print_models_text(cache, args.provider)
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

fn print_models_json(
    cache: &ModelsCache,
    provider_filter: Option<&str>,
    stale: bool,
) -> Result<()> {
    let mut output = serde_json::json!({
        "version": cache.version,
        "updated_at": cache.updated_at,
        "models": filtered_models(cache, provider_filter),
    });
    if stale {
        output["stale"] = serde_json::json!(true);
    }
    println!("{output}");
    Ok(())
}

fn print_models_text(cache: &ModelsCache, provider_filter: Option<&str>) -> Result<()> {
    println!("Supported models:");
    println!();

    let provider_models = filtered_models(cache, provider_filter);

    for (provider, models) in &provider_models {
        if models.is_empty() {
            continue;
        }

        println!("  {} ({} models)", provider, models.len());
        for model in *models {
            println!("    - {model}");
        }
        println!();
    }

    let total_models: usize = provider_models.values().map(|models| models.len()).sum();

    println!(
        "Total: {} providers, {} models",
        provider_models.len(),
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
