//! Key manager - handles master key lifecycle

use crate::error::Result;
use secrecy::SecretString;

pub struct KeyManager {
    // TODO: Implement key manager state
    key: SecretString,
}

impl KeyManager {
    /// Load master key from encrypted file and keyring
    pub fn load() -> Result<Self> {
        // TODO: Implement master key loading
        // 1. Get passphrase from keyring/env
        // 2. Decrypt master.key
        // 3. Return KeyManager with decrypted key
        Err(crate::error::PyxError::Crypto(
            "KeyManager::load not yet implemented".to_string(),
        ))
    }

    /// Generate a new random master key
    pub fn generate() -> Result<Self> {
        // TODO: Generate 32 random bytes
        Err(crate::error::PyxError::Crypto(
            "KeyManager::generate not yet implemented".to_string(),
        ))
    }

    /// Get the decrypted master key
    pub fn get_key(&self) -> &SecretString {
        &self.key
    }
}
