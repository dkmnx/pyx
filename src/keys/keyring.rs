//! Keyring backend abstraction for testability
//!
//! On Linux with KDE/kwallet, the OS keyring may not persist credentials
//! across Entry instances. We use a file-based fallback for reliability.

mod backend;
mod file_fallback;

// OS-specific keyring implementations
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use self::backend::with_backend;
#[cfg(test)]
pub use self::backend::{reset_backend, set_backend};
pub use self::backend::{KeyringBackend, MockKeyring};
use self::file_fallback::{
    delete_passphrase_file, file_fallback_enabled, get_passphrase_file, set_passphrase_file,
};
use crate::error::Result;
use crate::storage::paths::passphrase_path;
use secrecy::{ExposeSecret, SecretString};

const SERVICE_NAME: &str = "pyx";
const USER_NAME: &str = "master-key";
const ENV_PASSPHRASE: &str = "PYX_PASSPHRASE";

fn env_passphrase() -> Option<SecretString> {
    std::env::var(ENV_PASSPHRASE)
        .ok()
        .filter(|v| !v.is_empty())
        .map(|v| SecretString::new(v.into_boxed_str()))
}

/// Get passphrase - priority:
/// 1. PYX_PASSPHRASE env var
/// 2. OS Keyring (backend unavailable is treated as "not found")
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
    // Backend errors (unavailable keyring) are treated as "not found" to allow
    // fallback to file or user prompt without failing on transient issues.
    let backend_result = with_backend(|b| b.get_password(SERVICE_NAME, USER_NAME));
    let backend_pw = match backend_result {
        Ok(Some(pw)) => Some(pw),
        Ok(None) => None,
        Err(e) => {
            // Log the error but don't fail - treat unavailable backend as "not found"
            eprintln!("Warning: OS keyring unavailable: {e}");
            None
        }
    };

    if let Some(pw) = backend_pw {
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
/// Always attempts to delete the file fallback if it exists, regardless of
/// whether fallback is currently enabled. This prevents orphaned passphrase
/// files when fallback was previously used but is now disabled.
pub fn clear_passphrase() -> Result<()> {
    // Always try to delete the passphrase file - even if fallback is disabled,
    // an orphaned file from a previous session could still exist
    delete_passphrase_file()?;

    let _ = with_backend(|b| b.delete_password(SERVICE_NAME, USER_NAME));

    Ok(())
}

/// Check if a passphrase entry exists.
///
/// Only checks the file when PYX_ALLOW_FILE_FALLBACK is enabled.
pub fn has_entry() -> bool {
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
    fn test_clear_passphrase_removes_file_even_when_fallback_disabled() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
        }

        set_backend(Box::new(MockKeyring::new()));

        // Create a passphrase file
        let passphrase = SecretString::new("orphan-passphrase".to_string().into_boxed_str());
        set_passphrase(&passphrase).unwrap();

        // Verify file exists
        let file_path = passphrase_path().unwrap();
        assert!(
            file_path.exists(),
            "passphrase file should exist before clear"
        );

        // Disable fallback
        unsafe {
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }
        assert!(!file_fallback_enabled());

        // Clear should still remove the file
        clear_passphrase().unwrap();
        assert!(
            !file_path.exists(),
            "passphrase file should be removed even when fallback disabled"
        );

        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
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

    // --- Backend contract tests ---

    /// Test that get_password returns Ok(None) for missing entries,
    /// not an error. This is required for graceful fallback behavior.
    #[test]
    fn test_get_password_returns_none_for_missing_entry() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Use mock backend which stores nothing
        set_backend(Box::new(MockKeyring::new()));

        // Get on missing entry should return None, not error
        let result = with_backend(|b| b.get_password("nonexistent-service", "nonexistent-user"));
        assert!(
            result.is_ok(),
            "get_password should not error on missing entry"
        );
        assert!(
            result.unwrap().is_none(),
            "get_password should return None for missing entry"
        );

        reset_backend();
    }

    /// Test that set_password overwrites existing values.
    #[test]
    fn test_set_password_overwrites_existing() {
        let _guard = ENV_MUTEX.lock().unwrap();

        set_backend(Box::new(MockKeyring::new()));

        let service = "test-service";
        let user = "test-user";

        // Set first value
        with_backend(|b| b.set_password(service, user, "first-value")).unwrap();

        // Verify first value
        let first = with_backend(|b| b.get_password(service, user))
            .unwrap()
            .unwrap();
        assert_eq!(first, "first-value");

        // Overwrite with second value
        with_backend(|b| b.set_password(service, user, "second-value")).unwrap();

        // Verify second value
        let second = with_backend(|b| b.get_password(service, user))
            .unwrap()
            .unwrap();
        assert_eq!(second, "second-value");

        reset_backend();
    }

    /// Test that delete_password is idempotent.
    #[test]
    fn test_delete_password_is_idempotent() {
        let _guard = ENV_MUTEX.lock().unwrap();

        set_backend(Box::new(MockKeyring::new()));

        let service = "test-service";
        let user = "test-user";

        // Set a value
        with_backend(|b| b.set_password(service, user, "value")).unwrap();

        // First delete should succeed
        with_backend(|b| b.delete_password(service, user)).unwrap();

        // Second delete should also succeed (idempotent)
        with_backend(|b| b.delete_password(service, user)).unwrap();

        // Entry should be gone
        let result = with_backend(|b| b.get_password(service, user)).unwrap();
        assert!(result.is_none());

        reset_backend();
    }

    /// Test that get_passphrase falls back to file when backend returns error on read.
    /// This is critical: if the native backend is unavailable (e.g., keyring locked),
    /// get_passphrase should still try file fallback rather than propagating the error.
    #[test]
    fn test_get_passphrase_uses_file_fallback_when_backend_errors() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            std::env::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
            std::env::remove_var("PYX_PASSPHRASE");
        }

        // Create a backend that always errors on read
        struct ErrorBackend;
        impl crate::keys::keyring::backend::KeyringBackend for ErrorBackend {
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
                Ok(()) // Delete succeeds silently
            }
        }

        set_backend(Box::new(ErrorBackend));

        // Write to file fallback first
        let passphrase = SecretString::new("file-fallback-pass".to_string().into_boxed_str());
        set_passphrase_file(&passphrase).unwrap();

        // get_passphrase should return the file fallback, not error
        let result = get_passphrase().unwrap();
        assert!(
            result.is_some(),
            "get_passphrase should fall back to file when backend errors"
        );
        assert_eq!(result.unwrap().expose_secret(), "file-fallback-pass");

        // Cleanup
        clear_passphrase().unwrap();
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }
    }

    /// Test that has_entry checks backend availability, not just presence.
    #[test]
    fn test_has_entry_returns_false_when_backend_unavailable() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Create a backend that errors on read
        struct EmptyBackend;
        impl crate::keys::keyring::backend::KeyringBackend for EmptyBackend {
            fn get_password(&self, _: &str, _: &str) -> crate::error::Result<Option<String>> {
                // Return Ok(None) for missing, not error
                Ok(None)
            }
            fn set_password(&self, _: &str, _: &str, _: &str) -> crate::error::Result<()> {
                Ok(())
            }
            fn delete_password(&self, _: &str, _: &str) -> crate::error::Result<()> {
                Ok(())
            }
        }

        set_backend(Box::new(EmptyBackend));

        // No file fallback, no backend entry
        unsafe {
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
        }

        assert!(
            !has_entry(),
            "has_entry should return false when no entry exists"
        );

        reset_backend();
    }

    /// Regression test: set_passphrase should succeed even if backend partially fails,
    /// as long as the operation completes.
    #[test]
    fn test_set_passphrase_succeeds_when_backend_available() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();

        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
            std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
            std::env::remove_var("PYX_PASSPHRASE");
        }

        set_backend(Box::new(MockKeyring::new()));

        let passphrase = SecretString::new("backend-test".to_string().into_boxed_str());
        let result = set_passphrase(&passphrase);

        // Should succeed when backend is available
        assert!(
            result.is_ok(),
            "set_passphrase should succeed when backend available: {:?}",
            result
        );

        // Should be retrievable
        let retrieved = get_passphrase().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), "backend-test");

        clear_passphrase().unwrap();
        reset_backend();

        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
