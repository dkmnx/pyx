//! Provider environment variable configuration
//!
//! This module handles the new providers.json file for extension-backed providers.
//! Schema version 1 format:
//! ```json
//! {
//!   "schemaVersion": 1,
//!   "providers": [
//!     {
//!       "name": "qwen-cli",
//!       "envVar": "QWEN_CLI_API_KEY"
//!     }
//!   ]
//! }
//! ```

use crate::error::{PyxError, Result};
use crate::providers::validation::{validate_env_var, validate_provider_name};
use crate::storage::paths::providers_env_path;
use serde::{Deserialize, Serialize};

/// Provider environment variable mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEnvMapping {
    pub name: String,
    pub env_var: String,
}

/// Providers.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersEnvConfig {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub providers: Vec<ProviderEnvMapping>,
}

impl ProvidersEnvConfig {
    /// Load providers.json from disk
    pub fn load() -> Result<Option<Self>> {
        let path = providers_env_path()?;
        
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content)?;

        // Validate schema version
        if config.schema_version != 1 {
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

    /// Save providers.json to disk
    pub fn save(&self) -> Result<()> {
        let path = providers_env_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;

        // Set file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Find environment variable for a provider name
    pub fn get_env_var(&self, provider_name: &str) -> Option<&str> {
        self.providers
            .iter()
            .find(|p| p.name == provider_name)
            .map(|p| p.env_var.as_str())
    }

    /// Add or update a provider mapping
    pub fn upsert(&mut self, name: String, env_var: String) -> Result<()> {
        validate_provider_name(&name)?;
        validate_env_var(&env_var)?;

        // Remove existing entry
        self.providers.retain(|p| p.name != name);
        
        // Add new entry
        self.providers.push(ProviderEnvMapping { name, env_var });
        
        Ok(())
    }
}

impl Default for ProvidersEnvConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            providers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_valid_config() {
        let config = ProvidersEnvConfig {
            schema_version: 1,
            providers: vec![
                ProviderEnvMapping {
                    name: "qwen-cli".to_string(),
                    env_var: "QWEN_CLI_API_KEY".to_string(),
                },
            ],
        };

        assert!(config.providers.iter().all(|p| {
            validate_provider_name(&p.name).is_ok() && validate_env_var(&p.env_var).is_ok()
        }));
    }

    #[test]
    fn test_get_env_var() {
        let config = ProvidersEnvConfig {
            schema_version: 1,
            providers: vec![
                ProviderEnvMapping {
                    name: "openai".to_string(),
                    env_var: "OPENAI_API_KEY".to_string(),
                },
            ],
        };

        assert_eq!(config.get_env_var("openai"), Some("OPENAI_API_KEY"));
        assert_eq!(config.get_env_var("anthropic"), None);
    }
}
