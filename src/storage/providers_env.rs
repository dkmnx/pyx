//! Provider environment variable configuration
//!
//! Supports two formats:
//!
//! 1. Simple format (recommended):
//! ```json
//! {
//!   "qwen-cli": "QWEN_CLI_API_KEY",
//!   "my-provider": "MY_PROVIDER_API_KEY"
//! }
//! ```
//!
//! 2. Schema format:
//! ```json
//! {
//!   "schemaVersion": 1,
//!   "providers": [
//!     { "name": "qwen-cli", "envVar": "QWEN_CLI_API_KEY" }
//!   ]
//! }
//! ```

use crate::error::{PyxError, Result};
use crate::providers::validation::{validate_env_var, validate_provider_name};
use crate::storage::paths::providers_env_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider environment variable mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEnvMapping {
    pub name: String,
    pub env_var: String,
}

/// Serialization-only struct (no simple_map leakage)
#[derive(Debug, Serialize)]
struct ProvidersEnvOutput {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    providers: Vec<ProviderEnvMapping>,
}

/// Providers.json structure (schema format)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersEnvConfig {
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: u32,
    #[serde(default)]
    pub providers: Vec<ProviderEnvMapping>,
    /// Internal map for simple format (never written to disk)
    #[serde(flatten, default)]
    simple_map: HashMap<String, String>,
}

impl ProvidersEnvConfig {
    /// Load providers.json from disk
    pub fn load() -> Result<Option<Self>> {
        let path = providers_env_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;

        // Try to parse as simple format first
        let simple: std::result::Result<HashMap<String, String>, _> =
            serde_json::from_str(&content);
        if let Ok(map) = simple {
            let mut config = Self::default();
            for (name, env_var) in map {
                validate_provider_name(&name)?;
                validate_env_var(&env_var)?;
                config.simple_map.insert(name, env_var);
            }
            return Ok(Some(config));
        }

        // Try schema format
        let config: Self = serde_json::from_str(&content)?;

        // Validate schema version
        if config.schema_version > 0 && config.schema_version != 1 {
            return Err(PyxError::Validation(format!(
                "Unsupported providers.json schema version: {}",
                config.schema_version
            )));
        }

        // Validate all entries
        for provider in &config.providers {
            validate_provider_name(&provider.name)?;
            validate_env_var(&provider.env_var)?;
        }

        Ok(Some(config))
    }

    /// Save providers.json to disk (merges both formats)
    pub fn save(&self) -> Result<()> {
        let path = providers_env_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Merge simple_map entries into providers array to preserve all data
        let mut all_providers = self.providers.clone();
        for (name, env_var) in &self.simple_map {
            if !all_providers.iter().any(|p| &p.name == name) {
                all_providers.push(ProviderEnvMapping {
                    name: name.clone(),
                    env_var: env_var.clone(),
                });
            }
        }

        all_providers.sort_by(|a, b| a.name.cmp(&b.name));

        let output = ProvidersEnvOutput {
            schema_version: 1,
            providers: all_providers,
        };

        let content = serde_json::to_string_pretty(&output)?;
        crate::storage::atomic_write::atomic_write_with_backup(&path, content.as_bytes(), 0o600)?;

        Ok(())
    }

    /// Find environment variable for a provider name
    pub fn get_env_var(&self, provider_name: &str) -> Option<&str> {
        // Check simple map first
        if let Some(env_var) = self.simple_map.get(provider_name) {
            return Some(env_var);
        }
        // Then check providers array
        self.providers
            .iter()
            .find(|p| p.name == provider_name)
            .map(|p| p.env_var.as_str())
    }

    /// Get all provider names
    pub fn provider_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.simple_map.keys().map(|s| s.as_str()).collect();
        for p in &self.providers {
            if !names.contains(&p.name.as_str()) {
                names.push(&p.name);
            }
        }
        names
    }

    /// Add or update a provider mapping
    pub fn upsert(&mut self, name: String, env_var: String) -> Result<()> {
        validate_provider_name(&name)?;
        validate_env_var(&env_var)?;

        // Remove from providers array
        self.providers.retain(|p| p.name != name);

        // Add to simple map
        self.simple_map.insert(name, env_var);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = ProvidersEnvConfig {
            schema_version: 1,
            providers: vec![ProviderEnvMapping {
                name: "qwen-cli".to_string(),
                env_var: "QWEN_CLI_API_KEY".to_string(),
            }],
            simple_map: HashMap::new(),
        };

        assert!(config.providers.iter().all(|p| {
            validate_provider_name(&p.name).is_ok() && validate_env_var(&p.env_var).is_ok()
        }));
    }

    #[test]
    fn test_get_env_var() {
        let mut config = ProvidersEnvConfig::default();
        config
            .simple_map
            .insert("openai".to_string(), "OPENAI_API_KEY".to_string());

        assert_eq!(config.get_env_var("openai"), Some("OPENAI_API_KEY"));
        assert_eq!(config.get_env_var("anthropic"), None);
    }

    #[test]
    fn test_simple_format() {
        let mut config = ProvidersEnvConfig::default();
        config
            .simple_map
            .insert("my-provider".to_string(), "MY_PROVIDER_API_KEY".to_string());

        assert_eq!(
            config.get_env_var("my-provider"),
            Some("MY_PROVIDER_API_KEY")
        );
    }
}
