//! Keyring backend abstraction for testability
//!
//! On Linux with KDE/kwallet, the OS keyring may not persist credentials
//! across Entry instances. We use a file-based fallback for reliability.

use crate::error::{PyxError, Result};
use crate::storage::paths::passphrase_path;
use secrecy::{ExposeSecret, SecretString};
use std::io::Write;
use std::process::{Command, Stdio};
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

/// Derive a machine-specific encryption key from platform-specific identifiers and username.
/// This binds the encrypted passphrase file to this specific machine/user.
fn derive_machine_key() -> SecretString {
    // Try multiple sources of machine identity in order of preference.
    // Each platform has its own reliable identifier.
    let machine_id = get_machine_id();

    // Get username
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".to_string());

    // Combine to create unique key for this machine/user
    let combined = format!("pyx-passphrase:{machine_id}:{user}");
    let hash = hex::encode(combined.as_bytes());

    SecretString::new(hash.into_boxed_str())
}

/// Get a unique machine identifier for the current platform.
fn get_machine_id() -> String {
    // Linux: /etc/machine-id is the standard
    #[cfg(target_os = "linux")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }

    // Windows: Try multiple sources
    #[cfg(target_os = "windows")]
    {
        // COMPOSER is a unique per-install identifier
        if let Ok(computer_name) = std::env::var("COMPUTERNAME") {
            if !computer_name.is_empty() {
                return computer_name;
            }
        }
        // USERDOMAIN and USERNAME combined
        let userdomain = std::env::var("USERDOMAIN").unwrap_or_default();
        let username = std::env::var("USERNAME").unwrap_or_default();
        if !userdomain.is_empty() || !username.is_empty() {
            return format!("{}-{}", userdomain, username);
        }
    }

    // macOS: platform ID or IOKit serial
    #[cfg(target_os = "macos")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
        // Try system_profiler as fallback
        if let Ok(output) = std::process::Command::new("system_profiler")
            .arg("SPHardwareDataType")
            .output()
        {
            if output.status.success() {
                let info = String::from_utf8_lossy(&output.stdout);
                if let Some(serial) = info.lines().find(|l| l.contains("Serial Number")) {
                    let serial = serial.split(':').nth(1).map(|s| s.trim()).unwrap_or("");
                    if !serial.is_empty() {
                        return serial.to_string();
                    }
                }
            }
        }
    }

    // Fallback for other Unix-like systems: try hostname
    #[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
    {
        if let Ok(hostname) = std::process::Command::new("hostname").output() {
            if hostname.status.success() {
                let name = String::from_utf8_lossy(&hostname.stdout).trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }
    }

    // Final fallback: use platform info with home directory.
    // This is less secure but still unique per user installation.
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| std::env::var("USERNAME").unwrap_or_default());
    format!(
        "pyx-fallback-{}-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        home
    )
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
    .map_err(|e| PyxError::Crypto(format!("Failed to encrypt passphrase file: {e}")))?;

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
/// 3. File fallback (opt-in via PYX_ALLOW_FILE_FALLBACK=1)
/// 4. None
///
/// The file fallback is disabled by default because it relies on machine-derived
/// keys (not secret material) for encryption, which weakens the security model.
/// Enable only when OS keyring persistence is known to be unreliable.
pub fn get_passphrase() -> Result<Option<SecretString>> {
    // 1. Try PYX_PASSPHRASE env var
    if let Some(passphrase) = env_passphrase() {
        return Ok(Some(passphrase));
    }

    // 2. Try OS keyring
    if let Some(pw) = with_backend(|b| b.get_password(SERVICE_NAME, USER_NAME))? {
        return Ok(Some(SecretString::new(pw.into_boxed_str())));
    }

    // 3. File fallback (only when explicitly enabled)
    if file_fallback_enabled() {
        if let Some(pw) = get_passphrase_file()? {
            return Ok(Some(pw));
        }
    }

    Ok(None)
}

/// Check if file fallback is enabled via environment variable.
fn file_fallback_enabled() -> bool {
    std::env::var("PYX_ALLOW_FILE_FALLBACK")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Store passphrase in keyring and optionally in file fallback.
///
/// The file fallback is disabled by default. Enable via PYX_ALLOW_FILE_FALLBACK=1
/// when OS keyring persistence is known to be unreliable on your system.
pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
    // Store in OS keyring first (primary)
    let keyring_result =
        with_backend(|b| b.set_password(SERVICE_NAME, USER_NAME, passphrase.expose_secret()));

    // Write file fallback only when explicitly enabled
    if file_fallback_enabled() {
        if let Err(e) = set_passphrase_file(passphrase) {
            eprintln!(
                "Warning: failed to write passphrase file fallback: {e}. \
                 Keyring storage may still be available."
            );
        }
    }

    // Report keyring errors only if we couldn't write the file fallback either
    if let Err(e) = keyring_result {
        if !file_fallback_enabled() {
            return Err(e);
        }
        // Log but don't fail if file fallback succeeded
        eprintln!("Warning: OS keyring unavailable: {e}");
    }

    Ok(())
}

/// Clear passphrase from keyring and file fallback.
///
/// Only clears the file fallback when PYX_ALLOW_FILE_FALLBACK is enabled.
pub fn clear_passphrase() -> Result<()> {
    // Delete from file only if fallback is enabled
    if file_fallback_enabled() {
        delete_passphrase_file()?;
    }

    // Delete from keyring
    let _ = with_backend(|b| b.delete_password(SERVICE_NAME, USER_NAME));

    Ok(())
}

/// Check if a passphrase entry exists.
///
/// Only checks the file when PYX_ALLOW_FILE_FALLBACK is enabled.
pub fn has_entry() -> bool {
    // Check keyring (always)
    let has_keyring = with_backend(|b| {
        b.get_password(SERVICE_NAME, USER_NAME)
            .ok()
            .flatten()
            .is_some()
    });

    if has_keyring {
        return true;
    }

    // Check file only when fallback is enabled
    if file_fallback_enabled() {
        return passphrase_path().map(|p| p.exists()).unwrap_or(false);
    }

    false
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
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
        }

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("file-test".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        // Should be retrievable (from file when fallback enabled)
        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "file-test");

        clear_passphrase().unwrap();
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }
    }

    #[test]
    fn test_has_entry_checks_file() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
        }

        set_backend(Box::new(MockKeyring::new()));

        assert!(!has_entry());

        let passphrase = SecretString::new("test".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        assert!(has_entry());

        clear_passphrase().unwrap();
        assert!(!has_entry());
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }
    }

    #[test]
    fn test_set_passphrase_does_not_write_file_by_default() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            // Do NOT set PYX_ALLOW_FILE_FALLBACK
        }

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("no-fallback".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        // File should NOT be created without fallback enabled
        let file_exists = passphrase_path().map(|p| p.exists()).unwrap_or(false);
        assert!(
            !file_exists,
            "file should not exist when fallback is disabled"
        );

        clear_passphrase().unwrap();
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn test_get_passphrase_uses_keyring_when_fallback_disabled() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            // Do NOT set PYX_ALLOW_FILE_FALLBACK
        }

        set_backend(Box::new(MockKeyring::new()));

        // Store via keyring (which always works in this test)
        let passphrase = SecretString::new("keyring-only".to_string().into_boxed_str());
        with_backend(|b| b.set_password(SERVICE_NAME, USER_NAME, passphrase.expose_secret()))
            .unwrap();

        // Should retrieve from keyring
        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "keyring-only");

        // Cleanup
        with_backend(|b| b.delete_password(SERVICE_NAME, USER_NAME)).unwrap();
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn test_file_fallback_enabled_env_var() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Not set by default
        assert!(!file_fallback_enabled());

        unsafe {
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
        }
        assert!(file_fallback_enabled());

        unsafe {
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "true");
        }
        assert!(file_fallback_enabled());

        unsafe {
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "0");
        }
        assert!(!file_fallback_enabled());

        unsafe {
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }
        assert!(!file_fallback_enabled());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_system_keyring_uses_secret_tool_when_available() {
        use std::fs;

        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let bin_dir = temp.path().join("bin");
        let store_dir = temp.path().join("store");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&store_dir).unwrap();

        let script_path = bin_dir.join("secret-tool");
        let script = r##"#!/usr/bin/env python3
import os
import pathlib
import sys

store_dir = pathlib.Path(os.environ["PYX_SECRET_TOOL_STORE_DIR"])
log_path = store_dir / "log.txt"
log_path.parent.mkdir(parents=True, exist_ok=True)

args = sys.argv[1:]
cmd = args[0]
attrs = args[1:]
if cmd == "store" and attrs[:2] == ["--label", "pyx passphrase"]:
    attrs = attrs[2:]
key = "__".join(attrs).replace("/", "_")
entry = store_dir / key

with log_path.open("a", encoding="utf-8") as fh:
    fh.write(cmd + "\n")

if cmd == "store":
    value = sys.stdin.read()
    entry.write_text(value, encoding="utf-8")
    sys.exit(0)
elif cmd == "lookup":
    if entry.exists():
        sys.stdout.write(entry.read_text(encoding="utf-8"))
        sys.exit(0)
    sys.exit(1)
elif cmd == "clear":
    if entry.exists():
        entry.unlink()
    sys.exit(0)
else:
    sys.exit(2)
"##;
        fs::write(&script_path, script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let original_path = std::env::var("PATH").unwrap_or_default();
        unsafe {
            std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), original_path));
            std::env::set_var("PYX_SECRET_TOOL_STORE_DIR", &store_dir);
            std::env::remove_var("PYX_PASSPHRASE");
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }
        reset_backend();

        let passphrase = SecretString::new("secret-tool-pass".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        let log = fs::read_to_string(store_dir.join("log.txt")).unwrap();
        assert!(log.contains("store"));

        let retrieved = get_passphrase().unwrap().unwrap();
        assert_eq!(retrieved.expose_secret(), "secret-tool-pass");

        let log = fs::read_to_string(store_dir.join("log.txt")).unwrap();
        assert!(log.contains("lookup"));

        clear_passphrase().unwrap();

        let log = fs::read_to_string(store_dir.join("log.txt")).unwrap();
        assert!(log.contains("clear"));

        unsafe {
            std::env::set_var("PATH", original_path);
            std::env::remove_var("PYX_SECRET_TOOL_STORE_DIR");
        }
    }
}
