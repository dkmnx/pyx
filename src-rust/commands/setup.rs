//! Setup command implementation

use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::storage::paths::ensure_data_dir;
use secrecy::SecretString;

/// Execute the setup command
pub fn execute() -> Result<()> {
    println!("=== Pyx Setup ===");
    println!();

    // Check if already set up
    if KeyManager::master_key_exists() {
        println!("Pyx is already initialized.");
        println!("Data directory: {}", ensure_data_dir()?.display());
        println!();
        println!("To reset and start fresh, run: pyx reset");
        return Ok(());
    }

    println!("This will initialize pyx with secure encrypted storage.");
    println!();

    // Prompt for passphrase
    let passphrase = prompt_for_passphrase()?;

    // Generate master key
    println!("Generating master key...");
    let manager = KeyManager::generate()?;

    // Store passphrase in keyring
    println!("Storing passphrase in OS keyring...");
    KeyManager::set_passphrase(&passphrase)?;

    // Save encrypted master key
    println!("Saving encrypted master key...");
    manager.save()?;

    // Create empty database
    println!("Initializing provider database...");
    crate::storage::database::Database::default().save()?;

    println!();
    println!("✓ Setup complete!");
    println!();
    println!("Next steps:");
    println!("  1. Add providers with: pyx (will prompt for API keys)");
    println!("  2. List providers with: pyx list");
    println!("  3. Update models with: pyx models update");
    println!();
    println!("Your API keys are encrypted with AES-256-GCM via age.");
    println!("The master key is stored in your OS keyring.");

    Ok(())
}

/// Prompt user for passphrase
fn prompt_for_passphrase() -> Result<SecretString> {
    use dialoguer::{Password, theme::ColorfulTheme};

    let theme = ColorfulTheme::default();

    println!("Enter a passphrase to encrypt your API keys:");
    println!("(This will be stored in your OS keyring)");
    println!();

    let passphrase = Password::with_theme(&theme)
        .with_prompt("Passphrase")
        .with_confirmation("Confirm passphrase", "Passphrases do not match")
        .interact()
        .map_err(|e| PyxError::Validation(format!("Failed to read passphrase: {}", e)))?;

    if passphrase.is_empty() {
        return Err(PyxError::Validation(
            "Passphrase cannot be empty".to_string(),
        ));
    }

    Ok(SecretString::new(passphrase.into_boxed_str()))
}

/// Prompt user for API key
pub fn prompt_for_api_key(provider_name: &str) -> Result<String> {
    use dialoguer::{Password, theme::ColorfulTheme};

    let theme = ColorfulTheme::default();

    let api_key = Password::with_theme(&theme)
        .with_prompt(format!("Enter API key for {}", provider_name))
        .interact()
        .map_err(|e| PyxError::Validation(format!("Failed to read API key: {}", e)))?;

    if api_key.is_empty() {
        return Err(PyxError::Validation("API key cannot be empty".to_string()));
    }

    Ok(api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires interactive input"]
    fn test_prompt_for_passphrase() {
        // This test requires interactive input
        let result = prompt_for_passphrase();
        assert!(result.is_ok());
    }
}
