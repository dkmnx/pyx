//! Windows keyring backend using Credential Manager via WinCred API.
//!
//! Uses the native Windows Credential Manager API for secure credential storage.

use crate::error::{PyxError, Result};
use crate::validation::validate_keyring_args;

use super::KeyringBackend;

/// Windows keyring backend using Credential Manager API.
pub(crate) struct WindowsKeyring;

impl WindowsKeyring {
    /// Check if we can access the Windows Credential Manager.
    /// This is always true on Windows as long as the API is available.
    pub fn is_available() -> bool {
        // WinCred API is always available on modern Windows
        true
    }

    /// Format target name for Credential Manager.
    /// Uses service:username format as the target name.
    fn format_target(service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

/// CREDENTIALW structure for Windows Credential Manager.
/// This must match the Windows API layout exactly for x64.
#[repr(C)]
struct CredentialW {
    flags: u32,
    cred_type: u32,
    target_name: *const u16,
    comment: *const u16,
    last_written: u64,
    credential_blob_size: u32,
    credential_blob: *const u8,
    persist: u32,
    attribute_count: u32,
    attributes: *const u8,
    target_alias: *const u16,
    user_name: *const u16,
}

#[cfg(target_os = "windows")]
impl KeyringBackend for WindowsKeyring {
    fn get_password(&self, service: &str, username: &str) -> Result<Option<String>> {
        validate_keyring_args(service, username)?;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let target = Self::format_target(service, username);
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut credential_ptr: *mut std::ffi::c_void = std::ptr::null_mut();

        unsafe {
            #[link(name = "advapi32")]
            extern "system" {
                fn CredReadW(
                    target_name: *const u16,
                    cred_type: u32,
                    flags: u32,
                    cred_ptr: *mut *mut std::ffi::c_void,
                ) -> i32;
                fn CredFree(cred: *mut std::ffi::c_void);
            }

            const CRED_TYPE_GENERIC: u32 = 1;

            let result = CredReadW(
                target_wide.as_ptr(),
                CRED_TYPE_GENERIC,
                0,
                &mut credential_ptr,
            );

            if result == 0 {
                // CredReadW returns 0 (FALSE) on failure
                // Common errors: 1168 (ERROR_NOT_FOUND), 1312 (ERROR_NO_SUCH_LOGON_SESSION)
                return Ok(None);
            }

            if credential_ptr.is_null() {
                return Ok(None);
            }

            // Cast to our CREDENTIALW struct for safe field access
            let cred = &*(credential_ptr as *const CredentialW);

            let password = if cred.credential_blob.is_null() || cred.credential_blob_size == 0 {
                String::new()
            } else {
                // Credential blob is UTF-16 encoded
                let utf16_slice: &[u16] = std::slice::from_raw_parts(
                    cred.credential_blob as *const u16,
                    cred.credential_blob_size as usize / 2,
                );
                String::from_utf16_lossy(utf16_slice)
            };

            CredFree(credential_ptr);

            if password.is_empty() {
                Ok(None)
            } else {
                Ok(Some(password))
            }
        }
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        validate_keyring_args(service, username)?;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let target = Self::format_target(service, username);

        // Prepare wide strings (null-terminated)
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let username_wide: Vec<u16> = OsStr::new(username)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let password_utf16: Vec<u16> = password.encode_utf16().collect();
        let blob_size = (password_utf16.len() * 2) as u32;

        unsafe {
            #[link(name = "advapi32")]
            extern "system" {
                fn CredWriteW(cred: *const CredentialW, flags: u32) -> i32;
            }

            let cred = CredentialW {
                flags: 0,
                cred_type: 1,
                target_name: target_wide.as_ptr(),
                comment: std::ptr::null(),
                last_written: 0,
                credential_blob_size: blob_size,
                credential_blob: password_utf16.as_ptr() as *const u8,
                persist: 2,
                attribute_count: 0,
                attributes: std::ptr::null(),
                target_alias: std::ptr::null(),
                user_name: username_wide.as_ptr(),
            };

            let result = CredWriteW(&cred, 0);

            if result == 0 {
                let error = std::io::Error::last_os_error();
                Err(PyxError::Keyring(format!("CredWriteW failed: {}", error)))
            } else {
                Ok(())
            }
        }
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        validate_keyring_args(service, username)?;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let target = Self::format_target(service, username);
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            #[link(name = "advapi32")]
            extern "system" {
                fn CredDeleteW(target_name: *const u16, cred_type: u32, flags: u32) -> i32;
            }

            const CRED_TYPE_GENERIC: u32 = 1;

            let result = CredDeleteW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0);

            if result == 0 {
                // WinCred returns non-zero on success, zero on failure
                // Check if it failed because the credential wasn't found
                let error = std::io::Error::last_os_error();
                let error_code = error.raw_os_error().unwrap_or(0);
                // ERROR_NOT_FOUND (1168) means already deleted - treat as success (idempotent)
                if error_code == 1168 {
                    Ok(())
                } else {
                    Err(PyxError::Keyring(format!(
                        "CredDeleteW failed: {} (code: {})",
                        error, error_code
                    )))
                }
            } else {
                Ok(())
            }
        }
    }
}

// Non-Windows stubs - these will never be called but need to compile
#[cfg(not(target_os = "windows"))]
impl KeyringBackend for WindowsKeyring {
    fn get_password(&self, _service: &str, _username: &str) -> Result<Option<String>> {
        Err(PyxError::Keyring(
            "Windows keyring is only available on Windows".to_string(),
        ))
    }

    fn set_password(&self, _service: &str, _username: &str, _password: &str) -> Result<()> {
        Err(PyxError::Keyring(
            "Windows keyring is only available on Windows".to_string(),
        ))
    }

    fn delete_password(&self, _service: &str, _username: &str) -> Result<()> {
        Err(PyxError::Keyring(
            "Windows keyring is only available on Windows".to_string(),
        ))
    }
}
