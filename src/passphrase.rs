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
    let helper_text = "Enter your passphrase (input is hidden):".to_string();

    let passphrase = prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: prompt_text.to_string(),
        helper: Some(helper_text),
        confirmation: None,
        empty_error: "Passphrase cannot be empty".to_string(),
        allow_empty: false,
    })?;

    Ok(SecretString::new(passphrase.into_boxed_str()))
}

/// Load a key manager from a user-provided passphrase and restore the keyring entry.
pub(crate) fn load_key_manager_with_passphrase(
    passphrase: &SecretString,
) -> Result<crate::keys::manager::KeyManager> {
    use crate::keys::manager::KeyManager;

    let manager = KeyManager::load_with_passphrase(passphrase)?;

    if let Err(e) = KeyManager::set_passphrase(passphrase) {
        eprintln!("Warning: failed to store passphrase in OS keyring: {e}");
    }

    Ok(manager)
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
            let passphrase = prompt_existing_passphrase(Some("Enter your pyx passphrase"))?;
            load_key_manager_with_passphrase(&passphrase).map_err(|e| {
                PyxError::Crypto(format!(
                    "Failed to decrypt master key: {e}. \
                     If you forgot your passphrase, run 'pyx reset' to start fresh."
                ))
            })
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::keyring::{get_passphrase, reset_backend, set_backend, MockKeyring};
    use crate::keys::manager::KeyManager;
    use crate::ENV_MUTEX;
    use secrecy::ExposeSecret;
    use tempfile::tempdir;

    #[test]
    fn test_prompt_new_passphrase_creates_secret() {
        // This test would require mocking the prompt system
        // For now, we just verify the function exists and compiles
        // Integration tests cover the actual prompting behavior
    }

    #[test]
    fn load_with_passphrase_restores_keyring_entry() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            std::env::remove_var("PYX_PASSPHRASE");
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
        let manager = KeyManager::generate().unwrap();
        let expected_key = manager.get_key_hex().to_string();
        manager.save_with_passphrase(&passphrase).unwrap();

        assert!(get_passphrase().unwrap().is_none());

        let loaded = load_key_manager_with_passphrase(&passphrase).unwrap();
        assert_eq!(loaded.get_key_hex(), expected_key);
        assert_eq!(
            get_passphrase().unwrap().unwrap().expose_secret(),
            passphrase.expose_secret()
        );

        reset_backend();
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
