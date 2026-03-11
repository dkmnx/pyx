//! Key manager - handles master key lifecycle

use crate::crypto::legacy_age::{decrypt_with_passphrase, encrypt_with_passphrase};
use crate::error::{PyxError, Result};
use crate::keys::keyring;
use crate::storage::paths::master_key_path;
use secrecy::{ExposeSecret, SecretString};
use std::fs;

/// Key manager - holds the decrypted master key
pub struct KeyManager {
    key: SecretString,
}

impl KeyManager {
    /// Load master key from encrypted file and keyring
    pub fn load() -> Result<Self> {
        // Get passphrase from keyring/env
        let passphrase = keyring::get_passphrase()?
            .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;

        // Read encrypted master.key
        let path = master_key_path()?;
        let encrypted_content = fs::read_to_string(&path).map_err(|e| {
            PyxError::Config(format!("Failed to read master.key: {}", e))
        })?;

        // Decrypt the master key
        let decrypted = decrypt_with_passphrase(encrypted_content.as_bytes(), &passphrase)
            .map_err(|e| PyxError::Crypto(format!("Failed to decrypt master key: {}", e)))?;

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
        getrandom::fill(&mut key_bytes).map_err(|e| {
            PyxError::Crypto(format!("Failed to generate random key: {}", e))
        })?;

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
        let key_bytes = hex::decode(self.key.expose_secret()).map_err(|e| {
            PyxError::Crypto(format!("Invalid master key hex: {}", e))
        })?;

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
        hex::decode(self.key.expose_secret()).map_err(|e| {
            PyxError::Crypto(format!("Invalid master key hex: {}", e))
        })
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
        master_key_path()
            .map(|path| path.exists())
            .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;

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
