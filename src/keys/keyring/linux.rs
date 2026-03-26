//! Linux keyring backend using Secret Service API via secret-tool CLI.
//!
//! Uses libsecret/secret-tool as the primary backend, with fallback to
//! the generic keyring crate if secret-tool is unavailable.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{PyxError, Result};
use crate::validation::validate_keyring_args;

use super::KeyringBackend;

/// Linux keyring backend that uses secret-tool CLI for Secret Service access.
pub(crate) struct LinuxKeyring;

impl LinuxKeyring {
    /// Check if secret-tool is available in PATH.
    pub fn is_available() -> bool {
        which::which("secret-tool").is_ok()
    }

    /// Trim trailing whitespace from command output.
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

impl KeyringBackend for LinuxKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        validate_keyring_args(service, username)?;
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
        validate_keyring_args(service, username)?;
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
        validate_keyring_args(service, username)?;
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
