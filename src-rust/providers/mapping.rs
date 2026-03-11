//! Provider environment variable mapping

use crate::error::Result;

/// Map provider name to environment variable
pub fn provider_to_env_var(provider_name: &str) -> Result<String> {
    // TODO: Implement provider to env var mapping
    // 1. Check providers.json
    // 2. Check settings.json customProviderEnvVars (legacy)
    // 3. Use hardcoded mappings (compatibility)
    // 4. Derive from provider name (fallback)
    
    let _ = provider_name;
    Err(crate::error::PyxError::ProviderNotFound(
        "Provider mapping not yet implemented".to_string(),
    ))
}
