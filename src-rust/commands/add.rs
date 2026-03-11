//! Add provider command implementation

use crate::crypto::age::encrypt_with_passphrase;
use crate::error::{PyxError, Result};
use crate::keys::keyring::get_passphrase;
use crate::keys::manager::KeyManager;
use crate::storage::database::{Database, ProviderEntry};

/// Execute the add provider command
pub fn execute(provider_name: Option<&str>) -> Result<()> {
    // Check if initialized
    if !KeyManager::master_key_exists() {
        eprintln!("Pyx not initialized. Run 'pyx setup' first.");
        eprintln!();
        eprintln!("To initialize:");
        eprintln!("  pyx setup");
        std::process::exit(1);
    }

    // Prompt for provider name if not provided
    let provider_name = match provider_name {
        Some(name) => name.to_string(),
        None => prompt_for_provider_name()?,
    };

    // Validate provider name
    crate::providers::validate_provider_name(&provider_name)?;

    // Prompt for API key
    let api_key = prompt_for_api_key(&provider_name)?;

    // Load database
    let mut db = Database::load().map_err(|e| match e {
        PyxError::Config(_) => {
            PyxError::Config("Database not found. Run 'pyx setup' first.".to_string())
        }
        _ => e,
    })?;

    // Check if provider already exists
    if db.has_provider(&provider_name) {
        eprintln!("Provider '{}' already exists.", provider_name);
        eprintln!(
            "To update it, first delete with: pyx delete {}",
            provider_name
        );
        std::process::exit(1);
    }

    // Get passphrase
    let passphrase = get_passphrase()?
        .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;

    // Encrypt API key
    let cipher = encrypt_with_passphrase(api_key.as_bytes(), &passphrase)
        .map_err(|e| PyxError::Crypto(format!("Failed to encrypt API key: {}", e)))?;

    // Create provider entry
    let entry = ProviderEntry::new(provider_name.clone(), cipher);
    db.upsert(entry);

    // Save database
    db.save()?;

    println!("✓ Provider '{}' added successfully!", provider_name);
    println!();
    println!("You can now use it with:");
    println!("  pyx {}", provider_name);

    Ok(())
}

/// Prompt user for provider name
fn prompt_for_provider_name() -> Result<String> {
    use dialoguer::{Input, theme::ColorfulTheme};

    let theme = ColorfulTheme::default();

    println!("Select a provider to add:");
    println!("Common providers: openai, anthropic, google, azure, groq, mistral");
    println!();

    let provider: String = Input::with_theme(&theme)
        .with_prompt("Provider name")
        .interact_text()
        .map_err(|e| PyxError::Validation(format!("Failed to read provider name: {}", e)))?;

    if provider.is_empty() {
        return Err(PyxError::Validation("Provider name cannot be empty".into()));
    }

    Ok(provider)
}

/// Prompt user for API key
fn prompt_for_api_key(provider_name: &str) -> Result<String> {
    use dialoguer::{Password, theme::ColorfulTheme};

    let theme = ColorfulTheme::default();

    println!("Enter API key for provider: {}", provider_name);
    println!("(Input is hidden; the key will be encrypted and stored securely)");
    println!();

    let api_key = Password::with_theme(&theme)
        .with_prompt("API Key")
        .interact()
        .map_err(|e| PyxError::Validation(format!("Failed to read API key: {}", e)))?;

    if api_key.is_empty() {
        return Err(PyxError::Validation("API key cannot be empty".to_string()));
    }

    // Basic validation - should look like a key
    if api_key.len() < 10 {
        eprintln!(
            "Warning: API key seems very short ({} characters)",
            api_key.len()
        );
    }

    Ok(api_key)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_add_provider_workflow() {
        // This test would require full setup (master key, etc.)
        // Integration testing is done via CLI tests
    }
}
