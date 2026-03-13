//! Settings storage

use crate::error::Result;
use crate::storage::atomic_write::atomic_write_with_backup;
use crate::storage::paths::settings_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Settings structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// GitHub source overrides
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_source: Option<GitHubSource>,

    /// Custom provider environment variable mappings (legacy)
    #[serde(default, rename = "customProviderEnvVars")]
    pub custom_provider_env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSource {
    pub owner: String,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

impl Settings {
    /// Load settings from file
    pub fn load() -> Result<Self> {
        let path = settings_path()?;
        Self::load_from_path(&path)
    }

    /// Load settings from specific path (for testing)
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let settings: Self = serde_json::from_str(&content)?;
        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self) -> Result<()> {
        let path = settings_path()?;
        self.save_to_path(&path)
    }

    /// Save settings to specific path (for testing)
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        atomic_write_with_backup(path, content.as_bytes(), 0o600)?;
        Ok(())
    }

    /// Get custom env var for a provider (legacy support)
    pub fn get_custom_env_var(&self, provider_name: &str) -> Option<&String> {
        self.custom_provider_env_vars.get(provider_name)
    }

    /// Set custom env var for a provider
    pub fn set_custom_env_var(&mut self, provider_name: String, env_var: String) {
        self.custom_provider_env_vars.insert(provider_name, env_var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_nonexistent_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let settings = Settings::load_from_path(&path).unwrap();
        assert_eq!(settings.custom_provider_env_vars.len(), 0);
        assert!(settings.github_source.is_none());
    }

    #[test]
    fn test_save_and_load_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let mut settings = Settings::default();
        settings.set_custom_env_var("openai".to_string(), "OPENAI_API_KEY".to_string());
        settings.set_custom_env_var("qwen-cli".to_string(), "QWEN_CLI_API_KEY".to_string());

        settings.save_to_path(&path).unwrap();

        let loaded = Settings::load_from_path(&path).unwrap();
        assert_eq!(loaded.custom_provider_env_vars.len(), 2);
        assert_eq!(
            loaded.get_custom_env_var("openai"),
            Some(&"OPENAI_API_KEY".to_string())
        );
    }

    #[test]
    fn test_github_source_serialization() {
        let settings = Settings {
            github_source: Some(GitHubSource {
                owner: "test".to_string(),
                repo: "repo".to_string(),
                branch: Some("main".to_string()),
            }),
            custom_provider_env_vars: HashMap::new(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("github_source"));
        assert!(json.contains("main"));
    }
}
