//! Settings storage

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Settings structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// GitHub source overrides
    #[serde(default)]
    pub github_source: Option<GitHubSource>,

    /// Custom provider environment variable mappings (legacy)
    #[serde(default, rename = "customProviderEnvVars")]
    pub custom_provider_env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSource {
    pub owner: String,
    pub repo: String,
    pub branch: Option<String>,
}

impl Settings {
    /// Load settings from file
    pub fn load(path: &Path) -> Result<Self> {
        // TODO: Implement settings loading
        let _ = path;
        Err(crate::error::PyxError::Config(
            "Settings::load not yet implemented".to_string(),
        ))
    }

    /// Save settings to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // TODO: Implement settings saving
        let _ = path;
        Err(crate::error::PyxError::Config(
            "Settings::save not yet implemented".to_string(),
        ))
    }
}
