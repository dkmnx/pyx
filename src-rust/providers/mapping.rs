//! Provider environment variable mapping
//!
//! Implements the unified provider registry with deterministic precedence:
//! 1. providers.json (highest priority)
//! 2. settings.json.customProviderEnvVars (legacy)
//! 3. Built-in hardcoded mappings (compatibility)
//! 4. Derived env-var naming (fallback)

use crate::error::Result;
use crate::storage::providers_env::ProvidersEnvConfig;
use crate::storage::settings::Settings;
use std::collections::HashMap;

/// Built-in hardcoded provider mappings for compatibility
fn get_builtin_mappings() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    
    // Core providers
    map.insert("openai", "OPENAI_API_KEY");
    map.insert("anthropic", "ANTHROPIC_API_KEY");
    map.insert("google", "GOOGLE_API_KEY");
    map.insert("google-vertex", "GOOGLE_VERTEX_API_KEY");
    map.insert("azure", "AZURE_OPENAI_API_KEY");
    map.insert("azure-openai", "AZURE_OPENAI_API_KEY");
    
    // Chinese providers
    map.insert("minimax", "MINIMAX_API_KEY");
    map.insert("minimax-cn", "MINIMAX_API_KEY");
    map.insert("zhipu", "ZHIPU_API_KEY");
    map.insert("baichuan", "BAICHUAN_API_KEY");
    map.insert("moonshot", "MOONSHOT_API_KEY");
    
    // Other providers
    map.insert("vercel-ai", "VERCEL_AI_API_KEY");
    map.insert("vercel-openai", "VERCEL_OPENAI_API_KEY");
    map.insert("vercel-anthropic", "VERCEL_ANTHROPIC_API_KEY");
    map.insert("groq", "GROQ_API_KEY");
    map.insert("mistral", "MISTRAL_API_KEY");
    map.insert("cohere", "COHERE_API_KEY");
    map.insert("together", "TOGETHER_API_KEY");
    map.insert("anyscale", "ANYSCALE_API_KEY");
    map.insert("replicate", "REPLICATE_API_KEY");
    map.insert("perplexity", "PERPLEXITY_API_KEY");
    map.insert("friendli", "FRIENDLI_API_KEY");
    
    map
}

/// Derive environment variable name from provider name
/// e.g., "my-provider" -> "MY_PROVIDER_API_KEY"
fn derive_env_var(provider_name: &str) -> String {
    let upper = provider_name.to_uppercase();
    let normalized = upper.replace("-", "_");
    format!("{}_API_KEY", normalized)
}

/// Unified provider environment variable resolver
pub struct ProviderEnvResolver {
    providers_config: Option<ProvidersEnvConfig>,
    settings: Settings,
    builtin_mappings: HashMap<&'static str, &'static str>,
}

impl ProviderEnvResolver {
    /// Create a new provider resolver
    pub fn new() -> Result<Self> {
        Ok(Self {
            providers_config: ProvidersEnvConfig::load()?,
            settings: Settings::load().unwrap_or_default(),
            builtin_mappings: get_builtin_mappings(),
        })
    }

    /// Get environment variable for a provider
    /// Returns the first match according to precedence rules
    pub fn get_env_var(&self, provider_name: &str) -> Result<String> {
        // 1. Check providers.json (highest priority)
        if let Some(ref config) = self.providers_config {
            if let Some(env_var) = config.get_env_var(provider_name) {
                return Ok(env_var.to_string());
            }
        }

        // 2. Check settings.json customProviderEnvVars (legacy)
        if let Some(env_var) = self.settings.get_custom_env_var(provider_name) {
            return Ok(env_var.clone());
        }

        // 3. Check built-in hardcoded mappings
        if let Some(env_var) = self.builtin_mappings.get(provider_name) {
            return Ok(env_var.to_string());
        }

        // 4. Derive from provider name (fallback)
        Ok(derive_env_var(provider_name))
    }

    /// Check if a provider has an explicit mapping (not derived)
    pub fn has_explicit_mapping(&self, provider_name: &str) -> bool {
        // Check providers.json
        if let Some(ref config) = self.providers_config {
            if config.get_env_var(provider_name).is_some() {
                return true;
            }
        }

        // Check settings.json
        if self.settings.get_custom_env_var(provider_name).is_some() {
            return true;
        }

        // Check built-in mappings
        self.builtin_mappings.contains_key(provider_name)
    }

    /// Add or update a provider mapping in providers.json
    pub fn upsert_mapping(&mut self, provider_name: &str, env_var: &str) -> Result<()> {
        if self.providers_config.is_none() {
            self.providers_config = Some(ProvidersEnvConfig::default());
        }

        if let Some(ref mut config) = self.providers_config {
            config.upsert(provider_name.to_string(), env_var.to_string())?;
            config.save()?;
        }

        Ok(())
    }
}

impl Default for ProviderEnvResolver {
    fn default() -> Self {
        Self {
            providers_config: None,
            settings: Settings::default(),
            builtin_mappings: get_builtin_mappings(),
        }
    }
}

/// Simple function to get env var for a provider (convenience wrapper)
pub fn provider_to_env_var(provider_name: &str) -> Result<String> {
    let resolver = ProviderEnvResolver::new()?;
    resolver.get_env_var(provider_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_env_var() {
        assert_eq!(derive_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(derive_env_var("my-provider"), "MY_PROVIDER_API_KEY");
        assert_eq!(derive_env_var("google-vertex"), "GOOGLE_VERTEX_API_KEY");
    }

    #[test]
    fn test_builtin_mappings() {
        let mappings = get_builtin_mappings();
        assert_eq!(mappings.get("openai"), Some(&"OPENAI_API_KEY"));
        assert_eq!(mappings.get("anthropic"), Some(&"ANTHROPIC_API_KEY"));
        assert_eq!(mappings.get("google-vertex"), Some(&"GOOGLE_VERTEX_API_KEY"));
    }
}
