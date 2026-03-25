//! macOS keyring backend using Security.framework.
//!
//! Uses the native macOS Keychain for secure credential storage.

use crate::error::{PyxError, Result};
use std::process::Command;

use super::KeyringBackend;

/// macOS keyring backend using the security CLI for Keychain access.
///
/// Note: The security CLI passes the password via command-line argument (-w),
/// which means it may be visible in process listings. For production use,
/// consider using the Security.framework directly via FFI or a crate.
pub(crate) struct MacOsKeyring;

impl MacOsKeyring {
    /// Check if the security CLI is available (always true on macOS).
    pub fn is_available() -> bool {
        which::which("security").is_ok()
    }
}

impl KeyringBackend for MacOsKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        // Use security find-generic-password to retrieve the password
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                service,
                "-a",
                username,
                "-w", // Output only the password
            ])
            .output()
            .map_err(|e| {
                PyxError::Keyring(format!("Failed to run security find-generic-password: {e}"))
            })?;

        if output.status.success() {
            let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if password.is_empty() {
                Ok(None)
            } else {
                Ok(Some(password))
            }
        } else {
            // Exit code 36 means item not found - this is expected
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("SecKeychainSearchCopyNext") || stderr.contains("could not be found")
            {
                Ok(None)
            } else {
                Err(PyxError::Keyring(format!(
                    "security find-generic-password failed: {}",
                    stderr.trim()
                )))
            }
        }
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        // First, try to delete any existing password (ignore errors if not found)
        let _ = Command::new("security")
            .args(["delete-generic-password", "-s", service, "-a", username])
            .output();

        // Add the new password
        let output = Command::new("security")
            .args([
                "add-generic-password",
                "-s",
                service,
                "-a",
                username,
                "-w",
                password,
                "-U", // Update if exists
            ])
            .output()
            .map_err(|e| {
                PyxError::Keyring(format!("Failed to run security add-generic-password: {e}"))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(PyxError::Keyring(format!(
                "security add-generic-password failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )))
        }
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let output = Command::new("security")
            .args(["delete-generic-password", "-s", service, "-a", username])
            .output()
            .map_err(|e| {
                PyxError::Keyring(format!(
                    "Failed to run security delete-generic-password: {e}"
                ))
            })?;

        // Exit code 36 means item not found - treat as success (idempotent)
        if output.status.success()
            || String::from_utf8_lossy(&output.stderr).contains("could not be found")
        {
            Ok(())
        } else {
            Err(PyxError::Keyring(format!(
                "security delete-generic-password failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )))
        }
    }
}
