//! Key manager - handles master key lifecycle

use crate::crypto::age::{decrypt_with_passphrase, encrypt_with_passphrase};
use crate::error::{PyxError, Result};
use crate::keys::keyring;
use crate::storage::paths::master_key_path;
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use std::sync::Mutex;
use std::time::Instant;

const LEGACY_PASSPHRASE: &str = "default";
const ENV_PASSPHRASE: &str = "PYX_PASSPHRASE";
const MAX_FAILED_ATTEMPTS: u32 = 5;
const LOCKOUT_DURATION_SECS: u64 = 30;

struct RateLimitState {
    failed_attempts: u32,
    last_failed_attempt: Option<Instant>,
}

static RATE_LIMIT: Mutex<RateLimitState> = Mutex::new(RateLimitState {
    failed_attempts: 0,
    last_failed_attempt: None,
});

fn check_rate_limit() -> Result<()> {
    let mut state = RATE_LIMIT.lock().unwrap();

    if state.failed_attempts >= MAX_FAILED_ATTEMPTS {
        if let Some(last) = state.last_failed_attempt {
            let elapsed = last.elapsed().as_secs();
            if elapsed < LOCKOUT_DURATION_SECS {
                let remaining = LOCKOUT_DURATION_SECS - elapsed;
                return Err(PyxError::Crypto(format!(
                    "Too many failed attempts, please wait {remaining} seconds before retrying"
                )));
            }
            state.failed_attempts = 0;
            state.last_failed_attempt = None;
        }
    }

    Ok(())
}

fn record_failed_attempt() {
    let mut state = RATE_LIMIT.lock().unwrap();
    state.failed_attempts += 1;
    state.last_failed_attempt = Some(Instant::now());
}

fn reset_failed_attempts() {
    let mut state = RATE_LIMIT.lock().unwrap();
    state.failed_attempts = 0;
    state.last_failed_attempt = None;
}

/// Key manager - holds the decrypted master key
pub struct KeyManager {
    key: SecretString,
}

impl KeyManager {
    /// Load master key from encrypted file using passphrase resolution.
    /// Matches Go implementation: env var → keyring → legacy "default"
    /// Includes rate limiting for failed decryption attempts.
    pub fn load() -> Result<Self> {
        // Check rate limiting before attempting decryption
        check_rate_limit()?;

        // Get passphrase (matches Go's getPassphrase priority)
        let primary_passphrase = keyring::get_passphrase()?
            .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;

        // Read encrypted master.key
        let path = master_key_path()?;
        let encrypted_content = fs::read_to_string(&path)
            .map_err(|e| PyxError::Config(format!("Failed to read master.key: {e}")))?;

        // Build passphrase candidates (matches Go's fallback chain)
        let passphrases = build_passphrase_candidates(&primary_passphrase);

        // Decrypt with fallback candidates (no interactive prompt - matches Go)
        let decrypted = decrypt_master_key_with_candidates(&encrypted_content, &passphrases)
            .map_err(|e| {
                // Record failed attempt for rate limiting
                record_failed_attempt();
                PyxError::Crypto(format!(
                    "Failed to decrypt master key: {e}. Run 'pyx setup' to reconfigure."
                ))
            })?;

        // Reset failed attempts on success
        reset_failed_attempts();

        // Convert to hex string for storage (avoiding binary data issues)
        let master_key_hex = hex::encode(&decrypted);

        Ok(Self {
            key: SecretString::new(master_key_hex.into_boxed_str()),
        })
    }

    /// Load master key using a provided passphrase directly.
    /// Used when passphrase isn't available from keyring/env.
    pub fn load_with_passphrase(passphrase: &SecretString) -> Result<Self> {
        // Check rate limiting before attempting decryption
        check_rate_limit()?;

        // Read encrypted master.key
        let path = master_key_path()?;
        let encrypted_content = fs::read_to_string(&path)
            .map_err(|e| PyxError::Config(format!("Failed to read master.key: {e}")))?;

        // Build passphrase candidates (includes env var if set, and legacy if enabled)
        let passphrases = build_passphrase_candidates(passphrase);

        // Decrypt with fallback candidates
        let decrypted = decrypt_master_key_with_candidates(&encrypted_content, &passphrases)
            .map_err(|e| {
                record_failed_attempt();
                PyxError::Crypto(format!("Failed to decrypt master key: {e}"))
            })?;

        reset_failed_attempts();

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
            .map_err(|e| PyxError::Crypto(format!("Failed to generate random key: {e}")))?;

        // Store as hex string
        let master_key_hex = hex::encode(&key_bytes);

        Ok(Self {
            key: SecretString::new(master_key_hex.into_boxed_str()),
        })
    }

    /// Save encrypted master key to disk using the provided passphrase
    pub fn save_with_passphrase(&self, passphrase: &SecretString) -> Result<()> {
        // Decode hex to bytes
        let key_bytes = hex::decode(self.key.expose_secret())
            .map_err(|e| PyxError::Crypto(format!("Invalid master key hex: {e}")))?;

        // Encrypt with passphrase
        let encrypted = encrypt_with_passphrase(&key_bytes, passphrase)
            .map_err(|e| PyxError::Crypto(format!("Failed to encrypt master key: {e}")))?;

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

    /// Save encrypted master key to disk using passphrase from keyring/env
    pub fn save(&self) -> Result<()> {
        // Get passphrase
        let passphrase = keyring::get_passphrase()?
            .ok_or_else(|| PyxError::Keyring("No passphrase available".to_string()))?;

        self.save_with_passphrase(&passphrase)
    }

    /// Get the master key as hex string
    pub fn get_key_hex(&self) -> &str {
        self.key.expose_secret()
    }

    /// Get the master key as bytes
    pub fn get_key_bytes(&self) -> Result<Vec<u8>> {
        hex::decode(self.key.expose_secret())
            .map_err(|e| PyxError::Crypto(format!("Invalid master key hex: {e}")))
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

    if let Ok(env_value) = std::env::var(ENV_PASSPHRASE) {
        if !env_value.is_empty() && env_value != primary.expose_secret() {
            candidates.push(SecretString::new(env_value.into_boxed_str()));
        }
    }

    // Legacy passphrase fallback is deprecated and opt-in only
    // Set PYX_ALLOW_LEGACY_PASSPHRASE=1 to enable (for migration purposes)
    if primary.expose_secret() != LEGACY_PASSPHRASE
        && std::env::var("PYX_ALLOW_LEGACY_PASSPHRASE").as_deref() == Ok("1")
    {
        eprintln!(
            "Warning: Using deprecated legacy passphrase fallback. \
                   This will be removed in a future version."
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::keyring::{reset_backend, set_backend, MockKeyring};
    use crate::ENV_MUTEX;

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
    fn test_build_passphrase_candidates_adds_env_only_by_default() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            std::env::set_var(ENV_PASSPHRASE, "env-pass");
        }

        let primary = SecretString::new("keyring-pass".to_string().into_boxed_str());
        let candidates = build_passphrase_candidates(&primary);

        // By default, legacy passphrase is NOT added (opt-in only)
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].expose_secret(), "keyring-pass");
        assert_eq!(candidates[1].expose_secret(), "env-pass");

        unsafe {
            std::env::remove_var(ENV_PASSPHRASE);
        }
    }

    #[test]
    fn test_build_passphrase_candidates_adds_legacy_when_enabled() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            std::env::set_var("PYX_ALLOW_LEGACY_PASSPHRASE", "1");
        }

        let primary = SecretString::new("keyring-pass".to_string().into_boxed_str());
        let candidates = build_passphrase_candidates(&primary);

        // When legacy is enabled (but no env var set), we get:
        // 0: primary, 1: legacy
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].expose_secret(), "keyring-pass");
        assert_eq!(candidates[1].expose_secret(), LEGACY_PASSPHRASE);

        unsafe {
            std::env::remove_var("PYX_ALLOW_LEGACY_PASSPHRASE");
        }
    }

    #[test]
    fn test_generate_and_save_with_mock_keyring() {
        use tempfile::tempdir;

        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let _data_dir = temp.path().join("pyx");

        // Set up test environment
        set_backend(Box::new(MockKeyring::new()));
        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
        }

        // Generate a new key manager
        let manager = KeyManager::generate().unwrap();
        let key_hex = manager.get_key_hex().to_string();
        assert_eq!(key_hex.len(), 64); // 32 bytes = 64 hex chars

        // Set passphrase and save
        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
        KeyManager::set_passphrase(&passphrase).unwrap();
        manager.save().unwrap();

        // Load the key back
        let loaded = KeyManager::load().unwrap();
        assert_eq!(loaded.get_key_hex(), key_hex);

        // Cleanup
        KeyManager::delete_master_key().unwrap();
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
        reset_backend();
    }
}
