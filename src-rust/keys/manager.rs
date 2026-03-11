//! Key manager - handles master key lifecycle

use crate::crypto::age::{decrypt_with_passphrase, encrypt_with_passphrase};
use crate::error::{PyxError, Result};
use crate::keys::keyring;
use crate::storage::paths::master_key_path;
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use std::io::IsTerminal;

const LEGACY_PASSPHRASE: &str = "default";
const ENV_PASSPHRASE: &str = "PLY_PASSPHRASE";

/// Key manager - holds the decrypted master key
pub struct KeyManager {
    key: SecretString,
}

impl KeyManager {
    /// Load master key from encrypted file and available passphrase sources.
    pub fn load() -> Result<Self> {
        // Get primary passphrase from keyring/env resolution.
        let primary_passphrase = keyring::get_passphrase()?
            .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;

        // Read encrypted master.key
        let path = master_key_path()?;
        let encrypted_content = fs::read_to_string(&path)
            .map_err(|e| PyxError::Config(format!("Failed to read master.key: {}", e)))?;

        let passphrases = build_passphrase_candidates(&primary_passphrase);

        // Decrypt the master key using fallback candidates.
        let decrypted = match decrypt_master_key_with_candidates(&encrypted_content, &passphrases) {
            Ok(decrypted) => decrypted,
            Err(primary_error) => {
                if let Some(prompted_passphrase) = prompt_for_passphrase()? {
                    match decrypt_with_passphrase(&encrypted_content, &prompted_passphrase) {
                        Ok(decrypted) => {
                            // Best-effort keyring refresh for future runs.
                            let _ = keyring::set_passphrase(&prompted_passphrase);
                            decrypted
                        }
                        Err(prompt_error) => {
                            return Err(PyxError::Crypto(format!(
                                "Failed to decrypt master key: {}. Prompted passphrase also failed: {}",
                                primary_error, prompt_error
                            )));
                        }
                    }
                } else {
                    return Err(PyxError::Crypto(format!(
                        "Failed to decrypt master key: {}",
                        primary_error
                    )));
                }
            }
        };

        // Convert to hex string for storage (avoiding binary data issues)
        let master_key_hex = hex::encode(&decrypted);

        Ok(Self {
            key: SecretString::new(master_key_hex.into_boxed_str()),
        })
    }

    /// Generate a new random master key
    pub fn generate() -> Result<Self> {
        // Generate 32 random bytes using getrandom crate
        let mut key_bytes = vec![0u8; 32];
        getrandom::fill(&mut key_bytes)
            .map_err(|e| PyxError::Crypto(format!("Failed to generate random key: {}", e)))?;

        // Store as hex string
        let master_key_hex = hex::encode(&key_bytes);

        Ok(Self {
            key: SecretString::new(master_key_hex.into_boxed_str()),
        })
    }

    /// Save encrypted master key to disk
    pub fn save(&self) -> Result<()> {
        // Get passphrase
        let passphrase = keyring::get_passphrase()?
            .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;

        // Decode hex to bytes
        let key_bytes = hex::decode(self.key.expose_secret())
            .map_err(|e| PyxError::Crypto(format!("Invalid master key hex: {}", e)))?;

        // Encrypt with passphrase
        let encrypted = encrypt_with_passphrase(&key_bytes, &passphrase)
            .map_err(|e| PyxError::Crypto(format!("Failed to encrypt master key: {}", e)))?;

        // Write to file
        let path = master_key_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, &encrypted)?;

        // Set file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Get the master key as hex string
    pub fn get_key_hex(&self) -> &str {
        self.key.expose_secret()
    }

    /// Get the master key as bytes
    pub fn get_key_bytes(&self) -> Result<Vec<u8>> {
        hex::decode(self.key.expose_secret())
            .map_err(|e| PyxError::Crypto(format!("Invalid master key hex: {}", e)))
    }

    /// Set passphrase in keyring
    pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
        keyring::set_passphrase(passphrase)
    }

    /// Clear passphrase from keyring
    pub fn clear_passphrase() -> Result<()> {
        keyring::clear_passphrase()
    }

    /// Check if master key file exists
    pub fn master_key_exists() -> bool {
        master_key_path().map(|path| path.exists()).unwrap_or(false)
    }

    /// Delete master key file
    pub fn delete_master_key() -> Result<()> {
        let path = master_key_path()?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}

fn build_passphrase_candidates(primary: &SecretString) -> Vec<SecretString> {
    let mut candidates: Vec<SecretString> = vec![primary.clone()];

    if let Ok(env_value) = std::env::var(ENV_PASSPHRASE)
        && !env_value.is_empty()
        && env_value != primary.expose_secret()
    {
        candidates.push(SecretString::new(env_value.into_boxed_str()));
    }

    if primary.expose_secret() != LEGACY_PASSPHRASE {
        candidates.push(SecretString::new(
            LEGACY_PASSPHRASE.to_string().into_boxed_str(),
        ));
    }

    candidates
}

fn decrypt_master_key_with_candidates(
    encrypted_content: &str,
    passphrases: &[SecretString],
) -> Result<Vec<u8>> {
    let mut last_error = None;

    for passphrase in passphrases {
        match decrypt_with_passphrase(encrypted_content, passphrase) {
            Ok(decrypted) => return Ok(decrypted),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        PyxError::Crypto("No passphrase candidates available for decryption".to_string())
    }))
}

fn prompt_for_passphrase() -> Result<Option<SecretString>> {
    if std::env::var("PLY_NO_PROMPT")
        .ok()
        .as_deref()
        .is_some_and(|v| v == "1")
    {
        return Ok(None);
    }

    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Ok(None);
    }

    let input = crate::prompt::prompt_secret(crate::prompt::SecretPromptOptions {
        prompt: "Passphrase".to_string(),
        helper: Some("Enter passphrase to unlock your API keys (input is hidden):".to_string()),
        confirmation: None,
        empty_error: "Passphrase cannot be empty".to_string(),
        allow_empty: true,
    })?;

    if input.is_empty() {
        return Ok(None);
    }

    Ok(Some(SecretString::new(input.into_boxed_str())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_decrypt_master_key_with_legacy_fallback() {
        let wrong = SecretString::new("wrong-passphrase".to_string().into_boxed_str());
        let legacy = SecretString::new(LEGACY_PASSPHRASE.to_string().into_boxed_str());
        let plaintext = b"master-key-bytes";

        let encrypted = encrypt_with_passphrase(plaintext, &legacy).unwrap();
        let decrypted = decrypt_master_key_with_candidates(&encrypted, &[wrong, legacy]).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_build_passphrase_candidates_adds_env_and_legacy() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            std::env::set_var(ENV_PASSPHRASE, "env-pass");
        }

        let primary = SecretString::new("keyring-pass".to_string().into_boxed_str());
        let candidates = build_passphrase_candidates(&primary);

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].expose_secret(), "keyring-pass");
        assert_eq!(candidates[1].expose_secret(), "env-pass");
        assert_eq!(candidates[2].expose_secret(), LEGACY_PASSPHRASE);

        unsafe {
            std::env::remove_var(ENV_PASSPHRASE);
        }
    }

    #[test]
    fn test_prompt_for_passphrase_disabled_via_env() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            std::env::set_var("PLY_NO_PROMPT", "1");
        }

        let result = prompt_for_passphrase().unwrap();
        assert!(result.is_none());

        unsafe {
            std::env::remove_var("PLY_NO_PROMPT");
        }
    }

    #[test]
    #[ignore = "Requires keyring setup"]
    fn test_generate_and_load() {
        // This test requires keyring setup and should be run manually
        let manager = KeyManager::generate().unwrap();
        let key_hex = manager.get_key_hex().to_string();

        assert_eq!(key_hex.len(), 64); // 32 bytes = 64 hex chars

        // Save and reload would require keyring setup
    }
}
