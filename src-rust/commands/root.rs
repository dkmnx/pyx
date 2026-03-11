//! Root command execution - run pi with configured providers

use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::crypto::legacy_age::decrypt_with_passphrase;
use crate::storage::database::Database;
use crate::providers::provider_to_env_var;
use crate::pi::exec::{find_pi, spawn_pi};
use crate::keys::keyring::get_passphrase;

/// Execute the root command (run pi with providers)
pub fn execute(provider: Option<&str>, session: Option<&str>) -> Result<()> {
    // Check if initialized
    if !KeyManager::master_key_exists() {
        return Err(PyxError::Config(
            "Pyx not initialized. Run 'pyx setup' first.".to_string(),
        ));
    }

    // Check if pi is installed
    if find_pi().is_none() {
        eprintln!("pi not found in PATH.");
        eprintln!("Run 'pyx pi-install' to install pi first.");
        std::process::exit(1);
    }

    // Load database
    let db = Database::load().map_err(|e| {
        match e {
            PyxError::Config(_) => PyxError::Config(
                "No providers configured. Run 'pyx setup' first.".to_string(),
            ),
            _ => e,
        }
    })?;

    if db.is_empty() {
        return Err(PyxError::Config(
            "No providers configured. Add providers first.".to_string(),
        ));
    }

    // Determine which providers to use
    let providers_to_use = determine_providers(provider, &db)?;

    // Load master key
    let manager = KeyManager::load()?;
    let _master_key_hex = manager.get_key_hex().to_string();

    // Build environment variables
    let mut env_vars = Vec::new();
    for provider_name in &providers_to_use {
        let entry = db.get(provider_name).ok_or_else(|| {
            PyxError::ProviderNotFound(format!("Provider '{}' not found", provider_name))
        })?;

        // Decrypt API key
        let passphrase = get_passphrase()?
            .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;
        
        let api_key_bytes = decrypt_with_passphrase(
            entry.cipher.as_bytes(),
            &passphrase,
        ).map_err(|e| PyxError::Crypto(format!("Failed to decrypt API key: {}", e)))?;

        let api_key = String::from_utf8_lossy(&api_key_bytes).to_string();

        // Get env var for this provider
        let env_var = provider_to_env_var(provider_name)?;
        
        env_vars.push((env_var, api_key));
        eprintln!("✓ Loaded provider: {}", provider_name);
    }

    // Build pi arguments
    let mut args = Vec::new();
    
    // Add session if provided
    if let Some(session_id) = session {
        args.push("--session".to_string());
        args.push(session_id.to_string());
    }

    // Spawn pi process
    eprintln!();
    eprintln!("Launching pi with {} provider(s)...", providers_to_use.len());
    
    let env_vars_str: Vec<(String, String)> = env_vars
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    let exit_code = spawn_pi(&env_vars_str, &args)?;
    
    std::process::exit(exit_code);
}

/// Determine which providers to use based on CLI args and database
fn determine_providers(
    provider_arg: Option<&str>,
    db: &Database,
) -> Result<Vec<String>> {
    if let Some(provider_name) = provider_arg {
        // Single provider specified
        if !db.has_provider(provider_name) {
            return Err(PyxError::ProviderNotFound(format!(
                "Provider '{}' not found. Run 'pyx list' to see available providers.",
                provider_name
            )));
        }
        Ok(vec![provider_name.to_string()])
    } else {
        // Use all configured providers
        Ok(db.get_provider_names().iter().map(|s| s.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::ProviderEntry;

    #[test]
    fn test_determine_providers_single() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("openai".to_string(), "cipher".to_string()));
        db.upsert(ProviderEntry::new("anthropic".to_string(), "cipher".to_string()));

        let result = determine_providers(Some("openai"), &db).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "openai");
    }

    #[test]
    fn test_determine_providers_all() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new("openai".to_string(), "cipher".to_string()));
        db.upsert(ProviderEntry::new("anthropic".to_string(), "cipher".to_string()));

        let result = determine_providers(None, &db).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_determine_providers_not_found() {
        let db = Database::default();
        let result = determine_providers(Some("nonexistent"), &db);
        assert!(result.is_err());
    }
}
