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

    /// Convert a Rust string to a wide string (UTF-16) for Win32 API calls.
    fn to_wide_string(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
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
            // CredReadW function signature:
            // DWORD CredReadW(LPCWSTR TargetName, DWORD Type, DWORD Flags, PCREDENTIALW *Credential)
            #[link(name = "credui")]
            extern "system" {
                fn CredReadW(
                    target_name: *const u16,
                    cred_type: u32,
                    flags: u32,
                    cred_ptr: *mut *mut std::ffi::c_void,
                ) -> i32;
                fn CredFree(cred: *mut std::ffi::c_void) -> i32;
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

            // Parse the credential blob
            // CREDENTIALW structure:
            // typedef struct _CREDENTIALW {
            //   DWORD Flags;
            //   DWORD Type;
            //   LPWSTR TargetName;
            //   ...
            //   DWORD CredentialBlobSize;
            //   LPBYTE CredentialBlob;
            //   ...
            // } CREDENTIALW, *PCREDENTIALW;
            let cred_blob_size = *(credential_ptr as *const u32).offset(5);
            let cred_blob = *(credential_ptr as *const *mut u8).offset(6);

            let password = if cred_blob.is_null() || cred_blob_size == 0 {
                String::new()
            } else {
                let slice = std::slice::from_raw_parts(cred_blob, cred_blob_size as usize);
                // Credential blob is UTF-16 encoded
                let utf16_slice: &[u16] = std::slice::from_raw_parts(
                    cred_blob as *const u16,
                    cred_blob_size as usize / 2,
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
        let target_wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let username_wide: Vec<u16> = OsStr::new(username)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let password_utf16: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();

        let password_bytes: Vec<u8> = password_utf16
            .iter()
            .flat_map(|&c| c.to_le_bytes())
            .collect();

        unsafe {
            #[link(name = "credui")]
            extern "system" {
                fn CredWriteW(cred: *const std::ffi::c_void, flags: u32) -> i32;
            }

            // Build credential structure inline
            // We need to allocate memory for the structure and copy strings
            let cred_size = 14 * std::mem::size_of::<*const u16>() + 6 * std::mem::size_of::<u32>();
            let mut cred_data = vec![0u8; cred_size];

            // CREDENTIALW layout:
            // 0: Flags (DWORD)
            // 4: Type (DWORD) = 1 (CRED_TYPE_GENERIC)
            // 8: TargetName (LPWSTR) - offset to allocated string
            // 12: Comment (LPWSTR)
            // 16: LastWritten (FILETIME)
            // 24: CredentialBlobSize (DWORD)
            // 28: CredentialBlob (LPBYTE)
            // 32: Persist (DWORD) = 2 (CRED_PERSIST_LOCAL_MACHINE)
            // 36: AttributeCount (DWORD)
            // 40: Attributes (PCREDENTIAL_ATTRIBUTE)
            // 44: TargetAlias (LPWSTR)
            // 48: UserName (LPWSTR)

            let type_offset = 4;
            let target_offset = 8;
            let blob_size_offset = 24;
            let blob_offset = 28;
            let persist_offset = 32;
            let username_offset = 48;

            // Set Type = CRED_TYPE_GENERIC (1)
            let type_val: u32 = 1;
            cred_data[type_offset..type_offset + 4].copy_from_slice(&type_val.to_le_bytes());

            // Copy target name after the structure
            let str_offset = cred_data.len();
            // Safely convert u16 slice to bytes using as_byte_slice
            cred_data.extend_from_slice(target_wide.as_byte_slice());
            let target_ptr = str_offset as isize;
            cred_data[target_offset..target_offset + 8]
                .copy_from_slice(&(target_ptr as u64).to_le_bytes());

            // Set CredentialBlobSize
            let blob_size = password_bytes.len() as u32;
            cred_data[blob_size_offset..blob_size_offset + 4]
                .copy_from_slice(&blob_size.to_le_bytes());

            // Copy credential blob after target name
            let blob_str_offset = cred_data.len();
            cred_data.extend_from_slice(&password_bytes);
            let blob_ptr = blob_str_offset as isize;
            cred_data[blob_offset..blob_offset + 8]
                .copy_from_slice(&(blob_ptr as u64).to_le_bytes());

            // Set Persist = CRED_PERSIST_LOCAL_MACHINE (2)
            let persist_val: u32 = 2;
            cred_data[persist_offset..persist_offset + 4]
                .copy_from_slice(&persist_val.to_le_bytes());

            // Copy username after credential blob
            let user_str_offset = cred_data.len();
            cred_data.extend_from_slice(username_wide.as_byte_slice());
            let username_ptr = user_str_offset as isize;
            cred_data[username_offset..username_offset + 8]
                .copy_from_slice(&(username_ptr as u64).to_le_bytes());

            let result = CredWriteW(cred_data.as_ptr() as *const _, 0);

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
            #[link(name = "credui")]
            extern "system" {
                fn CredDeleteW(target_name: *const u16, cred_type: u32, flags: u32) -> i32;
            }

            const CRED_TYPE_GENERIC: u32 = 1;

            let result = CredDeleteW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0);

            // 0 means success, 1168 (ERROR_NOT_FOUND) means already deleted (idempotent)
            if result == 0 || result == 1168 {
                Ok(())
            } else {
                Err(PyxError::Keyring(format!(
                    "CredDeleteW failed with code: {}",
                    result
                )))
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
