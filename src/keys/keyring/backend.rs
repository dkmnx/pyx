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
use std::sync::OnceLock;

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

/// Cached result of the keyring availability probe.
///
/// Probing involves D-Bus IPC on Linux which can be slow (tens to hundreds of
/// milliseconds). Caching avoids repeating this on every `with_backend` call
/// within the same process.
static KEYRING_AVAILABLE: OnceLock<bool> = OnceLock::new();

impl NativeKeyring {
    /// Check if a native keyring backend is available.
    ///
    /// The result is cached after the first probe, so this is cheap to call
    /// repeatedly. The initial probe may involve D-Bus IPC on Linux.
    pub fn is_available() -> bool {
        *KEYRING_AVAILABLE.get_or_init(|| probe_keyring().is_ok())
    }
}

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

/// Unsupported keyring backend for platforms where no backend is available.
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

/// Create a new keyring entry for the given service and username.
///
/// This is a helper because `Entry::new` can fail on some platforms
/// (e.g., empty service/user is rejected by macOS Keychain).
fn entry_new(service: &str, username: &str) -> Result<Entry> {
    Entry::new(service, username)
        .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {e}")))
}

/// Probe whether a keyring backend is available by performing a write-read-delete
/// roundtrip on a temporary entry.
///
/// This catches cases where:
/// - The platform has no keyring daemon (e.g., headless Linux without gnome-keyring
///   or kwallet) or D-Bus is not accessible.
/// - The backend appears available for reads but silently fails to persist writes
///   (e.g., macOS/Windows CI runners where the keyring daemon responds to reads
///   but credentials don't actually persist after set_password).
///
/// The probe creates a temporary entry, writes a test password, reads it back to
/// verify persistence, and then cleans up to avoid accumulating stale entries.
/// Note: if the process crashes between `set_password` and `delete_credential`,
/// a stale probe entry (service "pyx-availability-check") may remain in the OS
/// keyring. It is harmless — the next run overwrites it — but not automatically
/// removed.
fn probe_keyring() -> Result<()> {
    let entry = Entry::new("pyx-availability-check", "probe")
        .map_err(|e| PyxError::Keyring(format!("Keyring backend unavailable: {e}")))?;

    // Write a test password to verify the backend can actually persist credentials.
    let test_password = "pyx-probe-check";
    entry
        .set_password(test_password)
        .map_err(|e| PyxError::Keyring(format!("Keyring probe write failed: {e}")))?;

    // Read it back to confirm the write was actually persisted.
    match entry.get_password() {
        Ok(pw) if pw == test_password => {}
        Ok(pw) => {
            // Unexpected password — clean up and report failure.
            let _ = entry.delete_credential();
            return Err(PyxError::Keyring(format!(
                "Keyring probe read mismatch: expected '{test_password}', got '{pw}'"
            )));
        }
        Err(KeyringError::NoEntry) => {
            // set_password returned Ok but the credential wasn't persisted.
            // This happens on macOS/Windows CI runners where the keyring
            // daemon appears available but writes silently fail.
            let _ = entry.delete_credential();
            return Err(PyxError::Keyring(
                "Keyring probe write did not persist: credential missing after set_password"
                    .to_string(),
            ));
        }
        Err(e) => {
            let _ = entry.delete_credential();
            return Err(PyxError::Keyring(format!("Keyring probe read failed: {e}")));
        }
    }

    // Clean up the probe entry to avoid accumulating stale entries.
    let _ = entry.delete_credential();
    Ok(())
}

/// Execute a function with the current backend, or the default platform backend.
///
/// In tests, uses the injected test backend if one is set. Otherwise, uses
/// `NativeKeyring` if available, falling back to `UnsupportedKeyring`.
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

    if NativeKeyring::is_available() {
        return f(&NativeKeyring);
    }

    f(&UnsupportedKeyring)
}
