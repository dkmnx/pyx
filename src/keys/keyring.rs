//! Keyring backend abstraction for testability

use crate::error::{PyxError, Result};
use secrecy::{ExposeSecret, SecretString};
use std::sync::{Mutex, MutexGuard};

/// Backend trait for keyring operations
pub trait KeyringBackend: Send + Sync {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>>;
    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()>;
    fn delete_password(&self, service: &str, username: &str) -> Result<()>;
}

/// OS keyring backend (production)
pub struct OsKeyring;

impl KeyringBackend for OsKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        match keyring::Entry::new(service, username) {
            Ok(entry) => match entry.get_password() {
                Ok(password) if !password.is_empty() => Ok(Some(password)),
                Ok(_) => Ok(None),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(PyxError::Keyring(format!(
                    "Failed to get password from keyring: {}",
                    e
                ))),
            },
            Err(e) => Err(PyxError::Keyring(format!(
                "Failed to create keyring entry: {}",
                e
            ))),
        }
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let entry = keyring::Entry::new(service, username)
            .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {}", e)))?;

        entry
            .set_password(password)
            .map_err(|e| PyxError::Keyring(format!("Failed to set password in keyring: {}", e)))?;

        Ok(())
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let entry = keyring::Entry::new(service, username)
            .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already gone, that's fine
            Err(e) => Err(PyxError::Keyring(format!(
                "Failed to delete password from keyring: {}",
                e
            ))),
        }
    }
}

/// In-memory mock keyring backend (for testing)
#[derive(Default)]
pub struct MockKeyring {
    store: Mutex<std::collections::HashMap<String, String>>,
}

impl MockKeyring {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

impl KeyringBackend for MockKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        let store = self.store.lock().unwrap();
        Ok(store.get(&Self::key(service, username)).cloned())
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let mut store = self.store.lock().unwrap();
        store.insert(Self::key(service, username), password.to_string());
        Ok(())
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let mut store = self.store.lock().unwrap();
        store.remove(&Self::key(service, username));
        Ok(())
    }
}

// Global backend singleton
static BACKEND: Mutex<Option<Box<dyn KeyringBackend>>> = Mutex::new(None);

/// Get the current backend (creates OS keyring if not set)
fn get_backend() -> MutexGuard<'static, Option<Box<dyn KeyringBackend>>> {
    BACKEND.lock().unwrap()
}

/// Set a custom backend (for testing)
#[cfg(test)]
pub fn set_backend(backend: Box<dyn KeyringBackend>) {
    let mut b = get_backend();
    *b = Some(backend);
}

/// Reset to default OS backend
#[cfg(test)]
pub fn reset_backend() {
    let mut b = get_backend();
    *b = None;
}

/// Execute an operation with the backend
fn with_backend<F, T>(f: F) -> T
where
    F: FnOnce(&dyn KeyringBackend) -> T,
{
    let guard = get_backend();
    if let Some(ref backend) = *guard {
        f(backend.as_ref())
    } else {
        // Drop the guard before creating OsKeyring to avoid deadlock
        drop(guard);
        f(&OsKeyring)
    }
}

// Re-export the public API with backend abstraction

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

    // 2. Try keyring backend
    let password = with_backend(|b| b.get_password(SERVICE_NAME, USER_NAME))?;
    if let Some(pw) = password {
        return Ok(Some(SecretString::new(pw.into_boxed_str())));
    }

    // 3. Legacy "default" passphrase
    Ok(Some(SecretString::new(
        LEGACY_PASSPHRASE.to_string().into_boxed_str(),
    )))
}

/// Store passphrase in keyring
pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
    with_backend(|b| b.set_password(SERVICE_NAME, USER_NAME, passphrase.expose_secret()))
}

/// Clear passphrase from keyring
pub fn clear_passphrase() -> Result<()> {
    with_backend(|b| b.delete_password(SERVICE_NAME, USER_NAME))
}

/// Check if a keyring entry exists (checks both pyx and legacy ply)
pub fn has_entry() -> bool {
    with_backend(|b| {
        b.get_password(SERVICE_NAME, USER_NAME)
            .ok()
            .flatten()
            .is_some()
            || b.get_password("ply", "master-key")
                .ok()
                .flatten()
                .is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ENV_MUTEX;

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
    fn test_mock_keyring_roundtrip() {
        let _guard = ENV_MUTEX.lock().unwrap();
        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());

        set_passphrase(&passphrase).unwrap();

        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "test-passphrase");

        clear_passphrase().unwrap();

        // After clearing, should fall back to legacy passphrase
        let retrieved = get_passphrase().unwrap();
        assert_eq!(retrieved.unwrap().expose_secret(), LEGACY_PASSPHRASE);

        reset_backend();
    }

    #[test]
    fn test_mock_keyring_has_entry() {
        let _guard = ENV_MUTEX.lock().unwrap();
        set_backend(Box::new(MockKeyring::new()));

        assert!(!has_entry());

        let passphrase = SecretString::new("test".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        assert!(has_entry());

        clear_passphrase().unwrap();
        assert!(!has_entry());

        reset_backend();
    }
}
