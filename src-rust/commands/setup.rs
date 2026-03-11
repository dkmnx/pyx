//! Setup command implementation

use crate::error::Result;
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
    println!("  1. Add providers with: pyx add");
    println!("  2. List providers with: pyx list");
    println!("  3. Update models with: pyx models update");
    println!();
    println!("Your API keys are encrypted with AES-256-GCM via age.");
    println!("The master key is stored in your OS keyring.");

    Ok(())
}

/// Prompt user for passphrase
fn prompt_for_passphrase() -> Result<SecretString> {
    let passphrase = crate::prompt::prompt_secret(crate::prompt::SecretPromptOptions {
        prompt: "Passphrase".to_string(),
        helper: Some("Enter passphrase to encrypt your API keys (input is hidden):".to_string()),
        confirmation: Some((
            "Confirm passphrase".to_string(),
            "Passphrases do not match".to_string(),
        )),
        empty_error: "Passphrase cannot be empty".to_string(),
        allow_empty: false,
    })?;

    Ok(SecretString::new(passphrase.into_boxed_str()))
}

/// Prompt user for API key
pub fn prompt_for_api_key(provider_name: &str) -> Result<String> {
    crate::prompt::prompt_secret(crate::prompt::SecretPromptOptions {
        prompt: "API key".to_string(),
        helper: Some(format!(
            "Enter API key for {} (input is hidden):",
            provider_name
        )),
        confirmation: Some((
            "Confirm API key".to_string(),
            "API keys do not match".to_string(),
        )),
        empty_error: "API key cannot be empty".to_string(),
        allow_empty: false,
    })
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
