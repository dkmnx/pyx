//! OS keyring integration

use crate::error::{PyxError, Result};
use secrecy::{ExposeSecret, SecretString};

const SERVICE_NAME: &str = "ply";
const USER_NAME: &str = "master-key";
const LEGACY_PASSPHRASE: &str = "default";

/// Get passphrase from OS keyring
pub fn get_passphrase() -> Result<Option<SecretString>> {
    // Try to get from keyring
    match keyring::Entry::new(SERVICE_NAME, USER_NAME) {
        Ok(entry) => {
            match entry.get_password() {
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
            }
        }
        Err(e) => {
            return Err(PyxError::Keyring(format!(
                "Failed to create keyring entry: {}",
                e
            )));
        }
    }

    // Fallback to PLY_PASSPHRASE environment variable
    if let Ok(env_pass) = std::env::var("PLY_PASSPHRASE") {
        if !env_pass.is_empty() {
            return Ok(Some(SecretString::new(env_pass.into_boxed_str())));
        }
    }

    // Legacy fallback: "default" passphrase
    Ok(Some(SecretString::new(LEGACY_PASSPHRASE.to_string().into_boxed_str())))
}

/// Store passphrase in OS keyring
pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, USER_NAME).map_err(|e| {
        PyxError::Keyring(format!("Failed to create keyring entry: {}", e))
    })?;

    entry.set_password(passphrase.expose_secret()).map_err(|e| {
        PyxError::Keyring(format!("Failed to set password in keyring: {}", e))
    })?;

    Ok(())
}

/// Clear passphrase from OS keyring
pub fn clear_passphrase() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, USER_NAME).map_err(|e| {
        PyxError::Keyring(format!("Failed to create keyring entry: {}", e))
    })?;

    // Delete by setting empty password (workaround for keyring crate)
    entry.set_password("").map_err(|e| {
        PyxError::Keyring(format!("Failed to clear password in keyring: {}", e))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
