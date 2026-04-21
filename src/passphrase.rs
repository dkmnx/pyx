//! Shared passphrase handling utilities
//!
//! This module centralizes passphrase prompting and validation logic
//! to avoid duplication across commands.

use crate::error::{PyxError, Result};
use crate::keys::keyring;
use crate::prompt;
use secrecy::SecretString;
use subtle::ConstantTimeEq;

/// Prompt for a new passphrase with confirmation.
/// Used during initial setup when creating a new passphrase.
pub fn prompt_new_passphrase() -> Result<SecretString> {
    prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: "Passphrase".to_string(),
        helper: Some("Choose a password to encrypt your API keys (input is hidden):".to_string()),
        confirmation: Some((
            "Confirm passphrase".to_string(),
            "Passphrases do not match".to_string(),
        )),
        empty_error: "Passphrase cannot be empty".to_string(),
    })
}

/// Prompt for an existing passphrase without confirmation.
/// Used when loading an existing passphrase (e.g., during setup or root command).
pub fn prompt_existing_passphrase(prompt_text: Option<&str>) -> Result<SecretString> {
    let prompt_text = prompt_text.unwrap_or("Passphrase");
    let helper_text = "Enter your passphrase (input is hidden):".to_string();

    prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: prompt_text.to_string(),
        helper: Some(helper_text),
        confirmation: None,
        empty_error: "Passphrase cannot be empty".to_string(),
    })
}

/// Load a key manager from a user-provided passphrase and restore the keyring entry.
pub(crate) fn load_key_manager_with_passphrase(
    passphrase: &SecretString,
) -> Result<crate::keys::manager::KeyManager> {
    use crate::keys::manager::KeyManager;
    use secrecy::ExposeSecret;
    use std::io::IsTerminal;

    let manager = KeyManager::load_with_passphrase(passphrase)?;

    let keyring_existing = match keyring::get_keyring_passphrase() {
        Ok(existing) => existing,
        Err(e) => {
            eprintln!(
                "Warning: could not query OS keyring, skipping keyring update to avoid overwrite: {e}"
            );
            return Ok(manager);
        }
    };

    let should_set = match &keyring_existing {
        Some(existing) => {
            let are_equal: bool = existing
                .expose_secret()
                .as_bytes()
                .ct_eq(passphrase.expose_secret().as_bytes())
                .into();
            !are_equal
        }
        None => true,
    };

    if should_set {
        if keyring_existing.is_some() {
            if std::io::stdin().is_terminal() {
                let confirmed = prompt::prompt_confirm(
                    "A passphrase already exists in the OS keyring. Overwrite?",
                )?;
                if !confirmed {
                    eprintln!("Skipping keyring update.");
                    return Ok(manager);
                }
            } else {
                return Err(PyxError::Keyring(
                    "A different passphrase already exists in the OS keyring. \
                     Run interactively to confirm overwrite."
                        .to_string(),
                ));
            }
        }

        if let Err(e) = KeyManager::set_passphrase(passphrase) {
            eprintln!("Warning: failed to store passphrase in OS keyring: {e}");
        }
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
            load_key_manager_with_passphrase(&passphrase).map_err(|e| match e {
                PyxError::Keyring(_) => e,
                _ => PyxError::Crypto(format!(
                    "Failed to decrypt master key: {e}. \
                     If you forgot your passphrase, run 'pyx reset' to start fresh."
                )),
            })
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::keyring::{
        get_passphrase, has_entry, reset_backend, set_backend, MockKeyring,
    };
    use crate::keys::manager::KeyManager;
    use crate::test_helpers::EnvGuard;
    use secrecy::ExposeSecret;
    use tempfile::tempdir;

    #[test]
    fn load_with_passphrase_sets_keyring_when_only_file_fallback_exists() {
        let temp = tempdir().unwrap();
        let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path());
        env.extend(EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1"));
        env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));
        env.extend(EnvGuard::remove_var("PYX_PASSPHRASE"));

        set_backend(Box::new(crate::keys::keyring::ReadNoneWriteFailsBackend));

        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
        let manager = KeyManager::generate().unwrap();
        let expected_key = manager.get_key_hex().to_string();
        manager.save_with_passphrase(&passphrase).unwrap();

        // Store only in file fallback (keyring write fails)
        crate::keys::keyring::set_passphrase(&passphrase).unwrap();

        // Switch to a working mock keyring for the actual test
        set_backend(Box::new(MockKeyring::new()));

        assert!(has_entry());
        assert!(get_passphrase().unwrap().is_some());
        assert!(
            keyring::get_keyring_passphrase().unwrap().is_none(),
            "keyring should be empty"
        );

        let loaded = load_key_manager_with_passphrase(&passphrase).unwrap();
        assert_eq!(loaded.get_key_hex(), expected_key);
        // Should have written to the keyring since no keyring entry existed
        assert_eq!(
            keyring::get_keyring_passphrase()
                .unwrap()
                .unwrap()
                .expose_secret(),
            passphrase.expose_secret()
        );

        crate::keys::keyring::clear_passphrase().unwrap();
        reset_backend();
    }

    #[test]
    fn load_with_passphrase_restores_keyring_entry() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path());

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
    }

    #[test]
    fn load_with_passphrase_skips_when_same_passphrase_exists() {
        let temp = tempdir().unwrap();
        let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path());
        env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
        let manager = KeyManager::generate().unwrap();
        let expected_key = manager.get_key_hex().to_string();
        manager.save_with_passphrase(&passphrase).unwrap();
        KeyManager::set_passphrase(&passphrase).unwrap();

        let loaded = load_key_manager_with_passphrase(&passphrase).unwrap();
        assert_eq!(loaded.get_key_hex(), expected_key);
        assert!(has_entry());
        assert_eq!(
            get_passphrase().unwrap().unwrap().expose_secret(),
            passphrase.expose_secret()
        );

        reset_backend();
    }

    #[test]
    fn load_with_passphrase_errors_when_different_passphrase_exists_non_interactive() {
        let temp = tempdir().unwrap();
        let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path());
        env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
        let other_passphrase = SecretString::new("other-passphrase".to_string().into_boxed_str());
        let manager = KeyManager::generate().unwrap();
        manager.save_with_passphrase(&passphrase).unwrap();
        KeyManager::set_passphrase(&other_passphrase).unwrap();

        let result = load_key_manager_with_passphrase(&passphrase);
        assert!(
            result.is_err(),
            "non-interactive should error when keyring has a different passphrase"
        );
        if let Err(err) = result {
            assert!(
                matches!(err, PyxError::Keyring(_)),
                "error should be Keyring variant"
            );
            let msg = format!("{err}");
            assert!(
                msg.contains("Run interactively to confirm overwrite"),
                "error should guide user to run interactively: {msg}"
            );
        }
        // Existing passphrase should be preserved
        assert_eq!(
            get_passphrase().unwrap().unwrap().expose_secret(),
            other_passphrase.expose_secret()
        );

        reset_backend();
    }
}
