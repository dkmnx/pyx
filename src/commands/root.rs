//! Root command execution - run pi with configured providers

use crate::crypto::age::{decrypt_with_key, decrypt_with_passphrase};
use crate::error::{PyxError, Result};
use crate::keys::keyring::get_passphrase;
use crate::keys::manager::KeyManager;
use crate::passphrase;
use crate::pi::exec::{
    find_pi, get_pi_version, install_completion, install_pi, platform_info, spawn_pi,
};
use crate::providers::mapping::ProviderEnvResolver;
use crate::storage::database::Database;
use std::collections::BTreeMap;

/// Load key manager with passphrase fallback
fn load_key_manager() -> Result<KeyManager> {
    passphrase::load_key_manager_with_fallback()
}

/// Execute the root command (run pi with providers)
pub fn execute(provider: Option<&str>, session: Option<&str>, pi_args: &[String]) -> Result<i32> {
    // Check if initialized
    if !KeyManager::master_key_exists() {
        return Err(PyxError::Config(
            "Pyx not initialized. Run 'pyx setup' first.".to_string(),
        ));
    }

    // Check if pi is installed, attempt auto-install if missing.
    let pi_was_just_installed = if find_pi().is_none() {
        eprintln!("pi not found in PATH. Attempting installation...");
        install_pi()?;

        if find_pi().is_none() {
            return Err(PyxError::CommandExecution(
                "pi not found in PATH after installation attempt".to_string(),
            ));
        }
        true
    } else {
        false
    };

    // If pi was just installed, show platform info and install completions
    if pi_was_just_installed {
        eprintln!("Platform: {}", platform_info());
        if let Ok(version) = get_pi_version() {
            eprintln!("pi version: {}", version);
        }
        eprintln!();

        // Install shell completion for detected shell
        if let Err(e) = install_completion() {
            eprintln!("Warning: failed to install shell completions: {}", e);
        }
    }

    // Load database
    let db = Database::load().map_err(|e| match e {
        PyxError::Config(_) => {
            PyxError::Config("No providers configured. Run 'pyx setup' first.".to_string())
        }
        _ => e,
    })?;

    if db.is_empty() {
        return Err(PyxError::Config(
            "No providers configured. Add providers first.".to_string(),
        ));
    }

    // Determine which providers to use
    let providers_to_use = determine_providers(provider, &db)?;

    // Load master key once (with passphrase fallback if keyring unavailable)
    let manager = load_key_manager()?;
    let mut master_key = manager.get_key_bytes()?;

    // Create provider env resolver once (avoid repeated disk loads)
    let resolver = ProviderEnvResolver::new()?;

    // Build environment variables with conflict detection
    let mut env_map: BTreeMap<String, String> = BTreeMap::new();
    for provider_name in &providers_to_use {
        let entry = db.get(provider_name).ok_or_else(|| {
            PyxError::ProviderNotFound(format!("Provider '{}' not found", provider_name))
        })?;

        // Decrypt API key with master key.
        // Fallback to passphrase-based decryption for legacy Rust-written entries.
        let api_key_bytes = match decrypt_with_key(&entry.cipher, &master_key) {
            Ok(bytes) => bytes,
            Err(primary_error) => {
                let passphrase = get_passphrase()?.ok_or_else(|| {
                    PyxError::Crypto(format!(
                        "Failed to decrypt API key with master key and no passphrase fallback is available: {}",
                        primary_error
                    ))
                })?;

                decrypt_with_passphrase(&entry.cipher, &passphrase).map_err(|fallback_error| {
                    PyxError::Crypto(format!(
                        "Failed to decrypt API key with master key ({}) and passphrase fallback ({})",
                        primary_error, fallback_error
                    ))
                })?
            }
        };

        let api_key = String::from_utf8_lossy(&api_key_bytes).to_string();

        // Resolve env var for this provider using cached resolver
        let env_var = resolver.get_env_var(provider_name)?;

        if let Some(existing) = env_map.get(&env_var) {
            if existing != &api_key {
                return Err(PyxError::Validation(format!(
                    "Conflicting API keys for environment variable {}",
                    env_var
                )));
            }
        } else {
            env_map.insert(env_var, api_key);
        }
    }

    // Zero master key bytes after decryption
    master_key.fill(0);

    // Build pi arguments
    let mut args = pi_args.to_vec();

    // Add session if provided
    if let Some(session_id) = session {
        args.push("--session".to_string());
        args.push(session_id.to_string());
    }

    // Spawn pi process
    let env_vars: Vec<(String, String)> = env_map.into_iter().collect();
    let exit_code = spawn_pi(&env_vars, &args)?;

    display_session_hint();

    Ok(exit_code)
}

/// Determine which providers to use based on CLI args and database
fn determine_providers(provider_arg: Option<&str>, db: &Database) -> Result<Vec<String>> {
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
        Ok(db
            .get_provider_names()
            .iter()
            .map(|s| s.to_string())
            .collect())
    }
}

fn display_session_hint() {
    let Ok(cwd) = std::env::current_dir() else {
        return;
    };

    let Some(cwd_str) = cwd.to_str() else {
        return;
    };

    let Ok(session_dir) = crate::session::dir_for_cwd(cwd_str) else {
        return;
    };

    let Ok(Some(uuid)) = crate::session::find_most_recent_session(&session_dir) else {
        return;
    };

    eprintln!("  ██████  ██");
    eprintln!("  ██  ██  ██    To continue this session, run:");
    eprintln!("  ████  ██  ██  pyx -s {}", uuid);
    eprintln!("  ██    ██  ██\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::ProviderEntry;

    #[test]
    fn test_determine_providers_single() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "cipher".to_string(),
        ));
        db.upsert(ProviderEntry::new(
            "anthropic".to_string(),
            "cipher".to_string(),
        ));

        let result = determine_providers(Some("openai"), &db).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "openai");
    }

    #[test]
    fn test_determine_providers_all() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "cipher".to_string(),
        ));
        db.upsert(ProviderEntry::new(
            "anthropic".to_string(),
            "cipher".to_string(),
        ));

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
