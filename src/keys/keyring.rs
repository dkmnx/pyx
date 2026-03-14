//! Keyring backend abstraction for testability
//!
//! On Linux with KDE/kwallet, the OS keyring may not persist credentials
//! across Entry instances. We use a file-based fallback for reliability.

use crate::error::{PyxError, Result};
use crate::storage::paths::passphrase_path;
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
const ENV_PASSPHRASE: &str = "PYX_PASSPHRASE";

fn env_passphrase() -> Option<SecretString> {
    std::env::var(ENV_PASSPHRASE)
        .ok()
        .filter(|v| !v.is_empty())
        .map(|v| SecretString::new(v.into_boxed_str()))
}

// File-based passphrase storage (fallback for systems where OS keyring is unreliable)
// Uses encryption with a machine-derived key for security

/// Derive a machine-specific encryption key from /etc/machine-id and username
/// This binds the encrypted passphrase to this specific machine/user
fn derive_machine_key() -> SecretString {
    // Try to get machine-id (Linux)
    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .unwrap_or_else(|_| {
            // Fallback: use OS info + a fixed string
            // This is less secure but still provides some protection
            format!(
                "fallback-{}-{:?}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })
        .trim()
        .to_string();

    // Get username
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".to_string());

    // Combine to create unique key for this machine/user
    // Simple deterministic transformation to create a passphrase
    let combined = format!("pyx-passphrase:{}:{}", machine_id, user);
    let hash = hex::encode(combined.as_bytes());

    SecretString::new(hash.into_boxed_str())
}

/// Store passphrase to encrypted file with restricted permissions
fn set_passphrase_file(passphrase: &SecretString) -> Result<()> {
    let path = passphrase_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Encrypt with machine-derived key
    let machine_key = derive_machine_key();
    let encrypted = crate::crypto::age::encrypt_with_passphrase(
        passphrase.expose_secret().as_bytes(),
        &machine_key,
    )
    .map_err(|e| PyxError::Crypto(format!("Failed to encrypt passphrase file: {}", e)))?;

    std::fs::write(&path, &encrypted)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Get passphrase from encrypted file
fn get_passphrase_file() -> Result<Option<SecretString>> {
    let path = passphrase_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let encrypted = std::fs::read_to_string(&path)?;
    if encrypted.is_empty() {
        return Ok(None);
    }

    // Decrypt with machine-derived key
    let machine_key = derive_machine_key();
    match crate::crypto::age::decrypt_with_passphrase(&encrypted, &machine_key) {
        Ok(decrypted) => {
            let passphrase = String::from_utf8_lossy(&decrypted).to_string();
            Ok(Some(SecretString::new(passphrase.into_boxed_str())))
        }
        Err(_) => {
            // Decryption failed - file may be corrupted or from different machine
            // Return None to trigger re-prompt
            Ok(None)
        }
    }
}

/// Delete passphrase file
fn delete_passphrase_file() -> Result<()> {
    let path = passphrase_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Get passphrase - priority:
/// 1. PYX_PASSPHRASE env var
/// 2. OS Keyring
/// 3. File fallback (~/.local/share/pyx/.passphrase)
/// 4. None
///
/// Returns Ok(None) if no passphrase is found (user needs to enter one).
pub fn get_passphrase() -> Result<Option<SecretString>> {
    // 1. Try PYX_PASSPHRASE env var
    if let Some(passphrase) = env_passphrase() {
        return Ok(Some(passphrase));
    }

    // 2. Try OS keyring
    if let Some(pw) = with_backend(|b| b.get_password(SERVICE_NAME, USER_NAME))? {
        return Ok(Some(SecretString::new(pw.into_boxed_str())));
    }

    // 3. Try file fallback (for systems where keyring doesn't persist)
    if let Some(pw) = get_passphrase_file()? {
        return Ok(Some(pw));
    }

    Ok(None)
}

/// Store passphrase in keyring AND file (for reliability across systems)
pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
    // Always write to file fallback (ensures persistence on all systems)
    set_passphrase_file(passphrase)?;

    // Also try OS keyring (may fail silently on some systems)
    let _ = with_backend(|b| b.set_password(SERVICE_NAME, USER_NAME, passphrase.expose_secret()));

    Ok(())
}

/// Clear passphrase from both keyring and file
pub fn clear_passphrase() -> Result<()> {
    // Delete from file
    delete_passphrase_file()?;

    // Delete from keyring
    let _ = with_backend(|b| b.delete_password(SERVICE_NAME, USER_NAME));

    Ok(())
}

/// Check if a passphrase entry exists (checks keyring and file)
pub fn has_entry() -> bool {
    // Check file first (most reliable)
    if passphrase_path().map(|p| p.exists()).unwrap_or(false) {
        return true;
    }

    // Check keyring
    with_backend(|b| {
        b.get_password(SERVICE_NAME, USER_NAME)
            .ok()
            .flatten()
            .is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ENV_MUTEX;
    use tempfile::tempdir;

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
    fn test_file_passphrase_roundtrip() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
        }

        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());

        // Set passphrase (writes to file)
        set_passphrase_file(&passphrase).unwrap();

        // Get passphrase (reads from file)
        let retrieved = get_passphrase_file().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "test-passphrase");

        // Delete
        delete_passphrase_file().unwrap();
        let retrieved = get_passphrase_file().unwrap();
        assert!(retrieved.is_none());

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn test_set_passphrase_writes_to_file() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
        }

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("file-test".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        // Should be retrievable (from file)
        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "file-test");

        clear_passphrase().unwrap();
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn test_has_entry_checks_file() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
        }

        assert!(!has_entry());

        let passphrase = SecretString::new("test".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        assert!(has_entry());

        clear_passphrase().unwrap();
        assert!(!has_entry());

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
