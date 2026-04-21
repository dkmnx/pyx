//! Cryptography module for encryption/decryption

pub mod age;

pub use age::{decrypt_with_passphrase, encrypt_with_passphrase};

use std::sync::OnceLock;

static SCRYPT_WARNING_EMITTED: OnceLock<()> = OnceLock::new();

pub fn warn_scrypt_env_override(env_var: &str, default: u8, actual: u8) {
    if SCRYPT_WARNING_EMITTED.set(()).is_ok() {
        eprintln!(
            "Warning: {env_var}={actual} overrides default scrypt work factor ({default}). \
             This weakens encryption if the value is too low."
        );
    }
}

pub fn get_scrypt_work_factor_with_warning(env_var: &str, default: u8, min: u8, max: u8) -> u8 {
    if let Ok(value) = crate::env_vars::var(env_var) {
        if let Ok(parsed) = value.parse() {
            if (min..=max).contains(&parsed) {
                if parsed != default {
                    warn_scrypt_env_override(env_var, default, parsed);
                }
                return parsed;
            }
        }
    }
    default
}
