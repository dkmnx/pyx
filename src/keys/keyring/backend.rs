//! Keyring backend trait and platform dispatch.
//!
//! This module defines the KeyringBackend trait and dispatches to the
//! platform-specific implementation (Linux, macOS, or Windows).

use std::sync::{Mutex, MutexGuard};

use crate::error::{PyxError, Result};

/// Trait for keyring backend implementations.
///
/// Implement this trait to provide platform-specific keyring functionality.
pub trait KeyringBackend: Send + Sync {
    /// Get a password from the keyring.
    ///
    /// Returns `Ok(Some(password))` if found, `Ok(None)` if not found,
    /// or an error if the backend is unavailable or an error occurs.
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>>;

    /// Set a password in the keyring.
    ///
    /// If a password already exists for the service/username, it should be overwritten.
    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()>;

    /// Delete a password from the keyring.
    ///
    /// This operation should be idempotent - deleting a non-existent password
    /// should succeed without error.
    fn delete_password(&self, service: &str, username: &str) -> Result<()>;
}

/// Mock keyring backend for testing.
#[derive(Default)]
pub struct MockKeyring {
    store: Mutex<std::collections::HashMap<String, String>>,
}

impl MockKeyring {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(service: &str, username: &str) -> String {
        format!("{service}:{username}")
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

/// Unsupported keyring backend for non-Windows/Linux/macOS platforms.
/// All operations return errors.
#[allow(dead_code)]
pub(crate) struct UnsupportedKeyring;

impl KeyringBackend for UnsupportedKeyring {
    fn get_password(&self, _service: &str, _username: &str) -> Result<Option<String>> {
        Err(PyxError::Keyring(
            "Keyring not supported on this platform".to_string(),
        ))
    }

    fn set_password(&self, _service: &str, _username: &str, _password: &str) -> Result<()> {
        Err(PyxError::Keyring(
            "Keyring not supported on this platform".to_string(),
        ))
    }

    fn delete_password(&self, _service: &str, _username: &str) -> Result<()> {
        Err(PyxError::Keyring(
            "Keyring not supported on this platform".to_string(),
        ))
    }
}

static BACKEND: Mutex<Option<Box<dyn KeyringBackend>>> = Mutex::new(None);

fn get_backend() -> MutexGuard<'static, Option<Box<dyn KeyringBackend>>> {
    BACKEND.lock().unwrap()
}

#[cfg(test)]
pub fn set_backend(backend: Box<dyn KeyringBackend>) {
    let mut current = get_backend();
    *current = Some(backend);
}

#[cfg(test)]
pub fn reset_backend() {
    let mut current = get_backend();
    *current = None;
}

/// Execute a function with the current backend, or the default platform backend.
pub(super) fn with_backend<F, T>(f: F) -> T
where
    F: FnOnce(&dyn KeyringBackend) -> T,
{
    let guard = get_backend();
    if let Some(ref backend) = *guard {
        return f(backend.as_ref());
    }

    drop(guard);

    // Dispatch to platform-specific backend
    #[cfg(target_os = "linux")]
    {
        use super::linux::LinuxKeyring;

        // Use secret-tool if available
        if LinuxKeyring::is_available() {
            return f(&LinuxKeyring);
        }

        // If secret-tool is not available, return error
        f(&UnsupportedKeyring)
    }

    #[cfg(target_os = "macos")]
    {
        use super::macos::MacOsKeyring;

        if MacOsKeyring::is_available() {
            return f(&MacOsKeyring);
        }

        // If security CLI is unavailable (shouldn't happen), return error
        f(&UnsupportedKeyring)
    }

    #[cfg(target_os = "windows")]
    {
        use super::windows::WindowsKeyring;

        if WindowsKeyring::is_available() {
            return f(&WindowsKeyring);
        }

        f(&UnsupportedKeyring)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Unsupported platform - return an error for all operations
        f(&UnsupportedKeyring)
    }
}
