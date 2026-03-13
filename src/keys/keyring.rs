//! OS keyring integration

use crate::error::{PyxError, Result};
use secrecy::{ExposeSecret, SecretString};

const SERVICE_NAME: &str = "pyx";
const USER_NAME: &str = "master-key";
const LEGACY_PASSPHRASE: &str = "default";
const ENV_PASSPHRASE: &str = "PYX_PASSPHRASE";

fn env_passphrase() -> Option<SecretString> {
    std::env::var(ENV_PASSPHRASE)
        .ok()
        .filter(|v| !v.is_empty())
        .map(|v| SecretString::new(v.into_boxed_str()))
}

/// Get passphrase - priority:
/// 1. PYX_PASSPHRASE env var
/// 2. OS Keyring
/// 3. Legacy "default" passphrase
pub fn get_passphrase() -> Result<Option<SecretString>> {
    // 1. Try PYX_PASSPHRASE env var
    if let Some(passphrase) = env_passphrase() {
        return Ok(Some(passphrase));
    }

    // 2. Try OS keyring
    match keyring::Entry::new(SERVICE_NAME, USER_NAME) {
        Ok(entry) => match entry.get_password() {
            Ok(password) if !password.is_empty() => {
                return Ok(Some(SecretString::new(password.into_boxed_str())));
            }
            Ok(_) => {}
            Err(keyring::Error::NoEntry) => {}
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

    // 3. Legacy "default" passphrase
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
        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());

        set_passphrase(&passphrase).unwrap();

        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "test-passphrase");

        clear_passphrase().unwrap();
    }
}
