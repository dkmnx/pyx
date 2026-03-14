//! Shared passphrase handling utilities
//!
//! This module centralizes passphrase prompting and validation logic
//! to avoid duplication across commands.

use crate::error::{PyxError, Result};
use crate::prompt;
use secrecy::SecretString;

/// Prompt for a new passphrase with confirmation.
/// Used during initial setup when creating a new passphrase.
pub fn prompt_new_passphrase() -> Result<SecretString> {
    let passphrase = prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: "Passphrase".to_string(),
        helper: Some("Choose a password to encrypt your API keys (input is hidden):".to_string()),
        confirmation: Some((
            "Confirm passphrase".to_string(),
            "Passphrases do not match".to_string(),
        )),
        empty_error: "Passphrase cannot be empty".to_string(),
        allow_empty: false,
    })?;

    Ok(SecretString::new(passphrase.into_boxed_str()))
}

/// Prompt for an existing passphrase without confirmation.
/// Used when loading an existing passphrase (e.g., during setup or root command).
pub fn prompt_existing_passphrase(prompt_text: Option<&str>) -> Result<SecretString> {
    let prompt_text = prompt_text.unwrap_or("Passphrase");
    let helper_text = format!("Enter your passphrase (input is hidden):");

    let passphrase = prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: prompt_text.to_string(),
        helper: Some(helper_text),
        confirmation: None,
        empty_error: "Passphrase cannot be empty".to_string(),
        allow_empty: false,
    })?;

    Ok(SecretString::new(passphrase.into_boxed_str()))
}

/// Load key manager with passphrase fallback to interactive prompt.
/// If keyring is unavailable, prompts the user for the passphrase.
pub fn load_key_manager_with_fallback() -> Result<crate::keys::manager::KeyManager> {
    use crate::keys::manager::KeyManager;

    // First try normal load (uses keyring/env)
    match KeyManager::load() {
        Ok(manager) => Ok(manager),
        Err(PyxError::Keyring(msg)) if msg.contains("No passphrase available") => {
            // Keyring unavailable - prompt user
            eprintln!("Passphrase not found in OS keyring.");
            let passphrase = prompt_existing_passphrase(Some(
                "Enter your pyx passphrase",
            ))?;
            KeyManager::load_with_passphrase(&passphrase).map_err(|e| {
                PyxError::Crypto(format!(
                    "Failed to decrypt master key: {}. \
                     If you forgot your passphrase, run 'pyx reset' to start fresh.",
                    e
                ))
            })
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_prompt_new_passphrase_creates_secret() {
        // This test would require mocking the prompt system
        // For now, we just verify the function exists and compiles
        // Integration tests cover the actual prompting behavior
    }
}
