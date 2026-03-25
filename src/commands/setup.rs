//! Setup command implementation - unified init + add provider flow

use crate::crypto::age::encrypt_with_key;
use crate::error::{PyxError, Result};
use crate::keys::keyring;
use crate::keys::manager::KeyManager;
use crate::models::fetch::fetch_models_from_remote;
use crate::passphrase;
use crate::prompt;
use crate::storage::database::{Database, ProviderEntry};
use crate::storage::models_cache::ModelsCache;
use crate::storage::paths::database_path;
use crate::storage::paths::ensure_data_dir;
use crate::storage::providers_env::ProvidersEnvConfig;
use std::collections::HashSet;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Load database, creating new if none exists, but erroring if file exists but is corrupted.
/// This prevents silent data loss from corrupted configuration files.
fn load_or_create_database() -> Result<Database> {
    let path = database_path()?;

    if !path.exists() {
        // No existing database - create fresh one
        return Ok(Database::default());
    }

    // Database file exists - try to load it
    // Fail loudly if it's corrupted rather than silently overwriting
    Database::load_from_path(&path).map_err(|e| {
        PyxError::Config(format!(
            "Failed to load existing database (may be corrupted): {e}. \
             Run 'pyx reset' to start fresh if this persists."
        ))
    })
}

/// Execute the setup command
pub fn execute() -> Result<()> {
    println!("Pyx Setup");
    println!();

    // Ensure data directory exists
    let data_dir = ensure_data_dir()?;
    println!("Data directory: {}", data_dir.display());
    println!();

    // Load or create master key
    let manager = load_or_create_master_key()?;

    // Load database - fail if exists but corrupted, create new if doesn't exist
    let mut db = load_or_create_database()?;

    // Fetch and cache provider models
    fetch_providers()?;

    // Get provider list from cache
    let providers = get_provider_list()?;

    // Prompt for provider selection
    let provider = prompt_provider_selection(&providers, &db)?;

    // Prompt for API key
    let api_key = prompt_api_key(&provider)?;

    // Encrypt and store provider entry
    let is_update = store_provider_entry(&manager, &mut db, &provider, &api_key)?;

    // Report success
    let action = if is_update { "Updated" } else { "Created" };
    println!();
    println!("  {}: {} ({})", provider, action, format_time_now());
    println!();
    println!("Setup complete!");

    Ok(())
}

/// Load existing master key or create a new one.
fn load_or_create_master_key() -> Result<KeyManager> {
    if KeyManager::master_key_exists() {
        println!("Using existing master key");
        println!();

        // Check if we can get a passphrase (from env or keyring)
        match keyring::get_passphrase()? {
            Some(_) => {
                // Passphrase available, load the key
                return KeyManager::load();
            }
            None => {
                // No passphrase in keyring - prompt user
                // This can happen if OS keyring is unavailable or wasn't persisted
                println!("Passphrase not found in OS keyring.");
                println!("Enter the passphrase you used during initial setup:");
                println!();

                let passphrase = passphrase::prompt_existing_passphrase(Some("Passphrase"))?;

                // Try to load with the provided passphrase and restore keyring entry
                return passphrase::load_key_manager_with_passphrase(&passphrase).map_err(|e| {
                    PyxError::Crypto(format!(
                        "Failed to decrypt master key: {e}. \
                         If you forgot your passphrase, run 'pyx reset' to start fresh."
                    ))
                });
            }
        }
    }

    println!("This will initialize pyx with secure encrypted storage.");
    println!();

    // Prompt for passphrase
    let passphrase = passphrase::prompt_new_passphrase()?;

    // Generate master key
    println!("Generating master key...");
    let manager = KeyManager::generate()?;

    // Save encrypted master key (using passphrase directly, before keyring storage)
    println!("Saving encrypted master key...");
    manager.save_with_passphrase(&passphrase)?;

    // Store passphrase in keyring (after master key is saved successfully)
    println!("Storing passphrase in OS keyring...");
    keyring::set_passphrase(&passphrase)?;

    println!();
    println!("Master key initialized!");
    println!();

    Ok(manager)
}

/// Fetch providers from remote and cache them.
fn fetch_providers() -> Result<()> {
    print!("Fetching providers... ");
    let start = Instant::now();

    let result = fetch_models_from_remote();

    match result {
        Ok(cache) => {
            println!("done ({}ms)", start.elapsed().as_millis());
            cache.save()?;
        }
        Err(e) => {
            // Try to fall back to cached models
            if let Ok(_cached) = ModelsCache::load() {
                println!("using cache (offline mode)");
            } else {
                println!("failed");
                return Err(PyxError::Network(format!(
                    "Failed to fetch providers and no cache available: {e}"
                )));
            }
        }
    }

    println!();
    Ok(())
}

/// Get provider list from cache + custom providers from providers.json.
fn get_provider_list() -> Result<Vec<String>> {
    let mut providers: HashSet<String> = HashSet::new();

    // Add providers from models cache
    if let Ok(cache) = ModelsCache::load() {
        for provider in cache.models.keys() {
            providers.insert(provider.clone());
        }
    }

    // Add custom providers from providers.json
    if let Ok(Some(config)) = ProvidersEnvConfig::load() {
        for name in config.provider_names() {
            providers.insert(name.to_string());
        }
    }

    let mut list: Vec<String> = providers.into_iter().collect();
    list.sort();
    Ok(list)
}

/// Prompt for provider selection, handling override confirmation.
/// Returns ErrCancelled on cancel (matches Go behavior).
fn prompt_provider_selection(providers: &[String], db: &Database) -> Result<String> {
    let provider = match prompt::prompt_provider(providers) {
        Ok(p) => p,
        Err(PyxError::Validation(msg)) if msg.contains("cancelled") => {
            println!("\n\nSetup cancelled!");
            return Err(PyxError::Cancelled);
        }
        Err(e) => return Err(e),
    };

    crate::providers::validate_provider_name(&provider)?;

    if db.has_provider(&provider) {
        let confirm = prompt::prompt_confirm(&format!(
            "Provider '{provider}' already configured. Override?"
        ))?;
        if !confirm {
            println!("\nProvider already configured!\n");
            return Err(PyxError::Cancelled);
        }
    }

    Ok(provider)
}

/// Prompt for API key.
fn prompt_api_key(provider: &str) -> Result<String> {
    let api_key = prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: "API key".to_string(),
        helper: Some(format!("Enter API key for {provider} (input is hidden):")),
        confirmation: None,
        empty_error: "API key cannot be empty".to_string(),
        allow_empty: false,
    })?;

    Ok(api_key)
}

/// Store provider entry in database.
/// Returns true if this was an update, false if it was a new entry.
fn store_provider_entry(
    manager: &KeyManager,
    db: &mut Database,
    provider: &str,
    api_key: &str,
) -> Result<bool> {
    let mut master_key = manager.get_key_bytes()?;

    // Encrypt API key
    let cipher = encrypt_with_key(api_key.as_bytes(), &master_key)
        .map_err(|e| PyxError::Crypto(format!("Failed to encrypt API key: {e}")))?;

    // Zero master key after use
    master_key.fill(0);

    // Check if update or new entry
    let is_update = db.has_provider(provider);

    // Create and store entry
    let entry = ProviderEntry::new(provider.to_string(), cipher);
    db.upsert(entry);

    // Save database
    db.save()?;

    Ok(is_update)
}

/// Format current time for display.
fn format_time_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    format!("{hours:02}:{mins:02}:{secs:02} UTC")
}
