//! Keyring backend abstraction for testability.
//!
//! Uses the `keyring` crate for native OS keyring access on all platforms:
//! - Linux: Secret Service (gnome-keyring, kwallet) via D-Bus + kernel keyutils fallback
//! - macOS: Keychain via Security.framework
//! - Windows: Credential Manager via WinCred API
//!
//! The `keyring` crate handles platform dispatch internally. On Linux it tries
//! Secret Service first (persistent, encrypted) then kernel keyutils (in-memory,
//! session-scoped). This eliminates the need for the `secret-tool` CLI, `security`
//! CLI, or hand-rolled WinCred FFI.

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::Arc;

use crate::error::{PyxError, Result};
use keyring::{Entry, Error as KeyringError};

/// Trait for keyring backend implementations.
///
/// Implement this trait to provide platform-specific keyring functionality.
/// In production, `NativeKeyring` is always used. For tests, `MockKeyring`
/// allows in-memory keyring simulation.
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

/// Native OS keyring backend using the `keyring` crate.
///
/// On Linux, this uses Secret Service (gnome-keyring or kwallet) as the primary
/// store with kernel keyutils as an in-memory session fallback. On macOS it
/// uses the system Keychain. On Windows it uses Credential Manager.
pub(crate) struct NativeKeyring;

impl KeyringBackend for NativeKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        let entry = entry_new(service, username)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(PyxError::Keyring(format!("Keyring get failed: {e}"))),
        }
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let entry = entry_new(service, username)?;
        entry
            .set_password(password)
            .map_err(|e| PyxError::Keyring(format!("Keyring set failed: {e}")))?;
        Ok(())
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let entry = entry_new(service, username)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(PyxError::Keyring(format!("Keyring delete failed: {e}"))),
        }
    }
}

/// Mock keyring backend for testing.
#[cfg(test)]
#[derive(Default)]
pub struct MockKeyring {
    store: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
impl MockKeyring {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(service: &str, username: &str) -> String {
        format!("{service}:{username}")
    }
}

#[cfg(test)]
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

#[cfg(test)]
thread_local! {
    static TEST_BACKEND: RefCell<Option<Arc<dyn KeyringBackend>>> = RefCell::new(None);
}

#[cfg(test)]
pub fn set_backend(backend: Box<dyn KeyringBackend>) {
    TEST_BACKEND.with(|current| {
        *current.borrow_mut() = Some(Arc::from(backend));
    });
}

#[cfg(test)]
pub fn reset_backend() {
    TEST_BACKEND.with(|current| {
        *current.borrow_mut() = None;
    });
}

/// Keyring backend that returns None for reads and fails all writes.
/// Useful for simulating a keyring that is available but has no entries.
#[cfg(test)]
pub struct ReadNoneWriteFailsBackend;

#[cfg(test)]
impl KeyringBackend for ReadNoneWriteFailsBackend {
    fn get_password(&self, _: &str, _: &str) -> crate::error::Result<Option<String>> {
        Ok(None)
    }
    fn set_password(&self, _: &str, _: &str, _: &str) -> crate::error::Result<()> {
        Err(crate::error::PyxError::Keyring(
            "Backend unavailable".to_string(),
        ))
    }
    fn delete_password(&self, _: &str, _: &str) -> crate::error::Result<()> {
        Ok(())
    }
}

/// Keyring backend that fails all operations.
/// Useful for simulating a completely unavailable keyring.
#[cfg(test)]
pub struct UnavailableBackend;

#[cfg(test)]
impl KeyringBackend for UnavailableBackend {
    fn get_password(&self, _: &str, _: &str) -> crate::error::Result<Option<String>> {
        Err(crate::error::PyxError::Keyring(
            "Backend unavailable".to_string(),
        ))
    }
    fn set_password(&self, _: &str, _: &str, _: &str) -> crate::error::Result<()> {
        Err(crate::error::PyxError::Keyring(
            "Backend unavailable".to_string(),
        ))
    }
    fn delete_password(&self, _: &str, _: &str) -> crate::error::Result<()> {
        Ok(())
    }
}

/// Create a new keyring entry for the given service and username.
///
/// This is a helper because `Entry::new` can fail on some platforms
/// (e.g., empty service/user is rejected by macOS Keychain).
fn entry_new(service: &str, username: &str) -> Result<Entry> {
    Entry::new(service, username)
        .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {e}")))
}

pub(super) fn with_backend<F, T>(f: F) -> T
where
    F: FnOnce(&dyn KeyringBackend) -> T,
{
    #[cfg(test)]
    {
        if let Some(backend) = TEST_BACKEND.with(|current| current.borrow().clone()) {
            return f(backend.as_ref());
        }
    }

    // Try the native keyring directly — no probe needed.
    // If the keyring is unavailable, the operation itself will
    // return an error, and callers handle it appropriately.
    // This avoids the latency of a write-read-delete roundtrip
    // on every cold start (which can add 1-3s on systems with slow D-Bus).
    f(&NativeKeyring)
}
