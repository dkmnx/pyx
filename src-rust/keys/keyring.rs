//! OS keyring integration

use crate::error::{PyxError, Result};
use secrecy::{ExposeSecret, SecretString};

const SERVICE_NAME: &str = "ply";
const USER_NAME: &str = "master-key";
const LEGACY_PASSPHRASE: &str = "default";
const ENV_PASSPHRASE: &str = "PLY_PASSPHRASE";
const ENV_FORCE_PASSPHRASE: &str = "PLY_PASSPHRASE_FORCE";

fn env_passphrase() -> Option<SecretString> {
    std::env::var(ENV_PASSPHRASE)
        .ok()
        .filter(|v| !v.is_empty())
        .map(|v| SecretString::new(v.into_boxed_str()))
}

fn force_env_passphrase() -> bool {
    matches!(
        std::env::var(ENV_FORCE_PASSPHRASE)
            .ok()
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes")
    )
}

/// Get passphrase from OS keyring
pub fn get_passphrase() -> Result<Option<SecretString>> {
    // Optional force-override for deterministic non-interactive usage.
    if force_env_passphrase()
        && let Some(passphrase) = env_passphrase()
    {
        return Ok(Some(passphrase));
    }

    // Try to get from keyring
    match keyring::Entry::new(SERVICE_NAME, USER_NAME) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => {
                if !password.is_empty() {
                    return Ok(Some(SecretString::new(password.into_boxed_str())));
                }
            }
            Err(keyring::Error::NoEntry) => {
                // No entry in keyring, will try fallback
            }
            Err(e) => {
                return Err(PyxError::Keyring(format!(
                    "Failed to get password from keyring: {}",
                    e
                )));
            }
        },
        Err(e) => {
            return Err(PyxError::Keyring(format!(
                "Failed to create keyring entry: {}",
                e
            )));
        }
    }

    // Fallback to PLY_PASSPHRASE environment variable
    if let Some(passphrase) = env_passphrase() {
        return Ok(Some(passphrase));
    }

    // Legacy fallback: "default" passphrase
    Ok(Some(SecretString::new(
        LEGACY_PASSPHRASE.to_string().into_boxed_str(),
    )))
}

/// Store passphrase in OS keyring
pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, USER_NAME)
        .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {}", e)))?;

    entry
        .set_password(passphrase.expose_secret())
        .map_err(|e| PyxError::Keyring(format!("Failed to set password in keyring: {}", e)))?;

    Ok(())
}

/// Clear passphrase from OS keyring
pub fn clear_passphrase() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, USER_NAME)
        .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {}", e)))?;

    // Delete by setting empty password (workaround for keyring crate)
    entry
        .set_password("")
        .map_err(|e| PyxError::Keyring(format!("Failed to clear password in keyring: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_force_env_passphrase_values() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            std::env::set_var(ENV_FORCE_PASSPHRASE, "1");
        }
        assert!(force_env_passphrase());

        unsafe {
            std::env::set_var(ENV_FORCE_PASSPHRASE, "true");
        }
        assert!(force_env_passphrase());

        unsafe {
            std::env::set_var(ENV_FORCE_PASSPHRASE, "yes");
        }
        assert!(force_env_passphrase());

        unsafe {
            std::env::set_var(ENV_FORCE_PASSPHRASE, "0");
        }
        assert!(!force_env_passphrase());

        unsafe {
            std::env::remove_var(ENV_FORCE_PASSPHRASE);
        }
    }

    #[test]
    fn test_env_passphrase_reads_non_empty() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            std::env::set_var(ENV_PASSPHRASE, "from-env");
        }
        let passphrase = env_passphrase().unwrap();
        assert_eq!(passphrase.expose_secret(), "from-env");

        unsafe {
            std::env::set_var(ENV_PASSPHRASE, "");
        }
        assert!(env_passphrase().is_none());

        unsafe {
            std::env::remove_var(ENV_PASSPHRASE);
        }
    }

    #[test]
    #[ignore = "Requires actual keyring access"]
    fn test_keyring_roundtrip() {
        // This test requires actual keyring access and should be run manually
        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());

        set_passphrase(&passphrase).unwrap();

        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "test-passphrase");

        clear_passphrase().unwrap();
    }
}
