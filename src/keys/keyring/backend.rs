use crate::error::{PyxError, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

pub trait KeyringBackend: Send + Sync {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>>;
    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()>;
    fn delete_password(&self, service: &str, username: &str) -> Result<()>;
}

pub struct OsKeyring;

impl KeyringBackend for OsKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        match keyring::Entry::new(service, username) {
            Ok(entry) => match entry.get_password() {
                Ok(password) if !password.is_empty() => Ok(Some(password)),
                Ok(_) => Ok(None),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(PyxError::Keyring(format!(
                    "Failed to get password from keyring: {e}"
                ))),
            },
            Err(e) => Err(PyxError::Keyring(format!(
                "Failed to create keyring entry: {e}"
            ))),
        }
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let entry = keyring::Entry::new(service, username)
            .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {e}")))?;

        entry
            .set_password(password)
            .map_err(|e| PyxError::Keyring(format!("Failed to set password in keyring: {e}")))?;

        Ok(())
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let entry = keyring::Entry::new(service, username)
            .map_err(|e| PyxError::Keyring(format!("Failed to create keyring entry: {e}")))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(PyxError::Keyring(format!(
                "Failed to delete password from keyring: {e}"
            ))),
        }
    }
}

#[cfg(target_os = "linux")]
struct SecretToolKeyring;

#[cfg(target_os = "linux")]
impl SecretToolKeyring {
    fn is_available() -> bool {
        which::which("secret-tool").is_ok()
    }

    fn trim_output(output: Vec<u8>) -> Option<String> {
        let mut value = String::from_utf8_lossy(&output).to_string();
        while value.ends_with(['\n', '\r']) {
            value.pop();
        }
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
}

#[cfg(target_os = "linux")]
impl KeyringBackend for SecretToolKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        let output = Command::new("secret-tool")
            .args(["lookup", "service", service, "username", username])
            .output()
            .map_err(|e| PyxError::Keyring(format!("Failed to run secret-tool lookup: {e}")))?;

        if output.status.success() {
            return Ok(Self::trim_output(output.stdout));
        }

        Ok(None)
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let mut child = Command::new("secret-tool")
            .args([
                "store",
                "--label",
                "pyx passphrase",
                "service",
                service,
                "username",
                username,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PyxError::Keyring(format!("Failed to run secret-tool store: {e}")))?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            PyxError::Keyring("Failed to open stdin for secret-tool store".to_string())
        })?;
        stdin
            .write_all(password.as_bytes())
            .map_err(|e| PyxError::Keyring(format!("Failed to write secret-tool input: {e}")))?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .map_err(|e| PyxError::Keyring(format!("Failed to wait for secret-tool store: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(PyxError::Keyring(format!(
                "secret-tool store failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )))
        }
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let output = Command::new("secret-tool")
            .args(["clear", "service", service, "username", username])
            .output()
            .map_err(|e| PyxError::Keyring(format!("Failed to run secret-tool clear: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(PyxError::Keyring(format!(
                "secret-tool clear failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )))
        }
    }
}

#[cfg(target_os = "linux")]
struct LinuxKeyring;

#[cfg(target_os = "linux")]
impl KeyringBackend for LinuxKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        if SecretToolKeyring::is_available() {
            return SecretToolKeyring.get_password(service, username);
        }
        OsKeyring.get_password(service, username)
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        if SecretToolKeyring::is_available() {
            return SecretToolKeyring.set_password(service, username, password);
        }
        OsKeyring.set_password(service, username, password)
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        if SecretToolKeyring::is_available() {
            return SecretToolKeyring.delete_password(service, username);
        }
        OsKeyring.delete_password(service, username)
    }
}

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

pub(super) fn with_backend<F, T>(f: F) -> T
where
    F: FnOnce(&dyn KeyringBackend) -> T,
{
    let guard = get_backend();
    if let Some(ref backend) = *guard {
        return f(backend.as_ref());
    }

    drop(guard);

    #[cfg(target_os = "linux")]
    {
        f(&LinuxKeyring)
    }

    #[cfg(not(target_os = "linux"))]
    {
        f(&OsKeyring)
    }
}
