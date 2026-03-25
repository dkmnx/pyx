use crate::error::{PyxError, Result};
use crate::storage::paths::passphrase_path;
use secrecy::{ExposeSecret, SecretString};

pub(super) fn file_fallback_enabled() -> bool {
    std::env::var("PYX_ALLOW_FILE_FALLBACK")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

pub(super) fn set_passphrase_file(passphrase: &SecretString) -> Result<()> {
    let path = passphrase_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

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

pub(super) fn get_passphrase_file() -> Result<Option<SecretString>> {
    let path = passphrase_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let encrypted = std::fs::read_to_string(&path)?;
    if encrypted.is_empty() {
        return Ok(None);
    }

    let machine_key = derive_machine_key();
    match crate::crypto::age::decrypt_with_passphrase(&encrypted, &machine_key) {
        Ok(decrypted) => {
            let passphrase = String::from_utf8_lossy(&decrypted).to_string();
            Ok(Some(SecretString::new(passphrase.into_boxed_str())))
        }
        Err(_) => Ok(None),
    }
}

pub(super) fn delete_passphrase_file() -> Result<()> {
    let path = passphrase_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

fn derive_machine_key() -> SecretString {
    let machine_id = get_machine_id();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".to_string());

    let combined = format!("pyx-passphrase:{machine_id}:{user}");
    let hash = hex::encode(combined.as_bytes());

    SecretString::new(hash.into_boxed_str())
}

fn get_machine_id() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(computer_name) = std::env::var("COMPUTERNAME") {
            if !computer_name.is_empty() {
                return computer_name;
            }
        }
        let userdomain = std::env::var("USERDOMAIN").unwrap_or_default();
        let username = std::env::var("USERNAME").unwrap_or_default();
        if !userdomain.is_empty() || !username.is_empty() {
            return format!("{}-{}", userdomain, username);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
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
