use crate::error::{PyxError, Result};
use crate::storage::atomic_write::atomic_write_with_backup;
use crate::storage::paths::passphrase_path;
use scrypt::{scrypt, Params as ScryptParams};
use secrecy::{ExposeSecret, SecretString};

const FILE_FALLBACK_KEY_PREFIX: &str = "pyx-passphrase";
const FILE_FALLBACK_KDF_SALT: &[u8] = b"pyx-file-fallback-v2";
const FILE_FALLBACK_KEY_LEN: usize = 32;
const FILE_FALLBACK_KDF_LOG_N_DEFAULT: u8 = 15;
const FILE_FALLBACK_KDF_R: u32 = 8;
const FILE_FALLBACK_KDF_P: u32 = 1;

/// Get scrypt work factor from environment or use default (same as age.rs)
fn get_kdf_log_n() -> u8 {
    std::env::var("PYX_SCRYPT_WORK_FACTOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| (14..=30).contains(&n))
        .unwrap_or(FILE_FALLBACK_KDF_LOG_N_DEFAULT)
}

pub(super) fn file_fallback_enabled() -> bool {
    std::env::var("PYX_ALLOW_FILE_FALLBACK")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

pub(super) fn set_passphrase_file(passphrase: &SecretString) -> Result<()> {
    eprintln!(
        "Warning: OS keyring unavailable. Storing passphrase in file with machine-derived key."
    );
    eprintln!("Warning: File fallback uses non-secret machine identifiers for encryption.");
    eprintln!("Warning: This is weaker than OS keyring. Set PYX_ALLOW_FILE_FALLBACK=0 to disable.");

    let path = passphrase_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    let machine_key = derive_machine_key();
    let encrypted = crate::crypto::age::encrypt_with_passphrase(
        passphrase.expose_secret().as_bytes(),
        &machine_key,
    )
    .map_err(|e| PyxError::Crypto(format!("Failed to encrypt passphrase file: {e}")))?;

    atomic_write_with_backup(&path, encrypted.as_bytes(), 0o600)?;

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

    let machine_id = get_machine_id();
    let user = resolve_user();

    if let Some(key_v2) = derive_machine_key_v2(&machine_id, &user) {
        if let Some(passphrase) = decrypt_passphrase_with_key(&encrypted, &key_v2) {
            return Ok(Some(passphrase));
        }
    }

    let legacy_key = derive_machine_key_legacy(&machine_id, &user);
    if let Some(passphrase) = decrypt_passphrase_with_key(&encrypted, &legacy_key) {
        eprintln!(
            "Warning: passphrase file uses deprecated legacy encryption. \
             Auto-migrating to v2 scrypt-based encryption."
        );
        if let Err(err) = set_passphrase_file(&passphrase) {
            eprintln!("Warning: failed to migrate passphrase file encryption: {err}");
        }
        return Ok(Some(passphrase));
    }

    Ok(None)
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
    let user = resolve_user();

    if let Some(key) = derive_machine_key_v2(&machine_id, &user) {
        return key;
    }

    eprintln!("Warning: v2 scrypt derivation failed, falling back to deprecated legacy method.");
    derive_machine_key_legacy(&machine_id, &user)
}

fn derive_machine_key_v2(machine_id: &str, user: &str) -> Option<SecretString> {
    let material = format!("{FILE_FALLBACK_KEY_PREFIX}:{machine_id}:{user}");
    let params = ScryptParams::new(
        get_kdf_log_n(),
        FILE_FALLBACK_KDF_R,
        FILE_FALLBACK_KDF_P,
        FILE_FALLBACK_KEY_LEN,
    )
    .ok()?;

    let mut output = [0u8; FILE_FALLBACK_KEY_LEN];
    scrypt(
        material.as_bytes(),
        FILE_FALLBACK_KDF_SALT,
        &params,
        &mut output,
    )
    .ok()?;

    Some(SecretString::new(hex::encode(output).into_boxed_str()))
}

fn derive_machine_key_legacy(machine_id: &str, user: &str) -> SecretString {
    // TODO: Remove legacy path after v0.3.0 release
    let combined = format!("{FILE_FALLBACK_KEY_PREFIX}:{machine_id}:{user}");
    SecretString::new(hex::encode(combined.as_bytes()).into_boxed_str())
}

fn resolve_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".to_string())
}

fn decrypt_passphrase_with_key(encrypted: &str, key: &SecretString) -> Option<SecretString> {
    let decrypted = crate::crypto::age::decrypt_with_passphrase(encrypted, key).ok()?;
    let passphrase = String::from_utf8(decrypted).ok()?;
    Some(SecretString::new(passphrase.into_boxed_str()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::EnvGuard;
    use crate::ENV_MUTEX;
    use secrecy::ExposeSecret;
    use tempfile::tempdir;

    fn legacy_machine_key_for_current_host() -> SecretString {
        let machine_id = get_machine_id();
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown-user".to_string());

        let combined = format!("pyx-passphrase:{machine_id}:{user}");
        SecretString::new(hex::encode(combined.as_bytes()).into_boxed_str())
    }

    #[test]
    fn derive_machine_key_is_not_plain_hex_encoding() {
        let machine_id = get_machine_id();
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown-user".to_string());
        let derived = derive_machine_key();

        let legacy = hex::encode(format!("pyx-passphrase:{machine_id}:{user}").as_bytes());

        assert_ne!(derived.expose_secret(), legacy);
    }

    #[test]
    fn get_passphrase_file_migrates_legacy_encryption_format() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let passphrase = SecretString::new("legacy-passphrase".to_string().into_boxed_str());
        let legacy_key = legacy_machine_key_for_current_host();

        let encrypted = crate::crypto::age::encrypt_with_passphrase(
            passphrase.expose_secret().as_bytes(),
            &legacy_key,
        )
        .unwrap();

        let path = passphrase_path().unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, &encrypted).unwrap();

        let before = std::fs::read_to_string(&path).unwrap();
        let loaded = get_passphrase_file().unwrap().unwrap();
        let after = std::fs::read_to_string(&path).unwrap();

        assert_eq!(loaded.expose_secret(), passphrase.expose_secret());
        assert_ne!(before, after);
    }
}
