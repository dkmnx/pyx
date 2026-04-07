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
mod tests;
