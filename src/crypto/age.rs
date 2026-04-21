//! Age encryption layer for pyx
//!
//! Uses age with scrypt passphrase encryption.
//! Compatible with age 0.11 API and Go implementation (base64-encoded output).
//!
//! # Key-Based Encryption
//!
//! For key-based encryption (`encrypt_with_key`/`decrypt_with_key`), the format
//! uses direct ChaCha20-Poly1305 with a random nonce (v2), identified by the
//! `pyx2:` format prefix.

use crate::crypto::get_scrypt_work_factor_with_warning;
use crate::error::{PyxError, Result};
use age::scrypt::{Identity, Recipient};
use age::{Decryptor, Encryptor};
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305,
};
use secrecy::SecretString;
use std::io::{Read, Write};

/// Default scrypt work factor (N = 2^18 = 262144 iterations)
/// This provides strong protection against brute-force attacks while maintaining
/// acceptable performance for interactive use.
/// Can be overridden via PYX_SCRYPT_WORK_FACTOR environment variable (value 15-30).
pub const DEFAULT_SCRYPT_WORK_FACTOR: u8 = 18;
pub const MIN_SCRYPT_WORK_FACTOR: u8 = 15;

/// Get scrypt work factor from environment or use default
fn get_scrypt_work_factor() -> u8 {
    get_scrypt_work_factor_with_warning(
        "PYX_SCRYPT_WORK_FACTOR",
        DEFAULT_SCRYPT_WORK_FACTOR,
        MIN_SCRYPT_WORK_FACTOR,
        30,
    )
}
/// Maximum accepted scrypt work factor for decryption (age library limit of 2^63)
const MAX_ACCEPTED_SCRYPT_WORK_FACTOR: u8 = 63;

const V2_PREFIX: &str = "pyx2:";
const V2_NONCE_BYTES: usize = 12;
/// ChaCha20-Poly1305 tag size (16 bytes)
const CHACHA_TAG_BYTES: usize = 16;

/// Direct ChaCha20-Poly1305 encryption with random nonce (v2 format).
/// Format: `pyx2:` + base64(nonce || ciphertext_with_tag)
fn encrypt_direct(plaintext: &[u8], key: &[u8]) -> Result<String> {
    if key.len() != 32 {
        return Err(PyxError::Crypto(
            "Key must be 32 bytes for direct encryption".to_string(),
        ));
    }

    let mut nonce = [0u8; V2_NONCE_BYTES];
    getrandom::fill(&mut nonce)
        .map_err(|e| PyxError::Crypto(format!("Random generation failed: {e}")))?;

    let key_array: &[u8; 32] = key.try_into().unwrap();
    let cipher = ChaCha20Poly1305::new(key_array.into());
    let ciphertext = cipher
        .encrypt((&nonce).into(), plaintext)
        .map_err(|e| PyxError::Crypto(format!("Encryption failed: {e}")))?;

    let mut payload = Vec::with_capacity(V2_NONCE_BYTES + ciphertext.len());
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);

    Ok(format!(
        "{V2_PREFIX}{}",
        base64::engine::general_purpose::STANDARD.encode(&payload)
    ))
}

/// Direct ChaCha20-Poly1305 decryption (v2 format).
fn decrypt_direct(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(PyxError::Crypto(
            "Key must be 32 bytes for direct decryption".to_string(),
        ));
    }

    let encoded = ciphertext
        .strip_prefix(V2_PREFIX)
        .ok_or_else(|| PyxError::Crypto("Invalid v2 ciphertext format".to_string()))?;

    let payload = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| PyxError::Crypto(format!("Base64 decode failed: {e}")))?;

    if payload.len() < V2_NONCE_BYTES + CHACHA_TAG_BYTES {
        return Err(PyxError::Crypto("Ciphertext too short".to_string()));
    }

    let (nonce, encrypted) = payload.split_at(V2_NONCE_BYTES);

    let key_array: &[u8; 32] = key.try_into().unwrap();
    let cipher = ChaCha20Poly1305::new(key_array.into());

    cipher
        .decrypt(nonce.into(), encrypted)
        .map_err(|_| PyxError::Crypto("Decryption failed (wrong key?)".to_string()))
}

/// Encrypt data using age with scrypt passphrase
///
/// Returns base64-encoded age ciphertext (matches Go implementation format)
pub fn encrypt_with_passphrase(plaintext: &[u8], passphrase: &SecretString) -> Result<String> {
    // Create scrypt recipient with explicit Go-compatible work factor.
    let mut recipient = Recipient::new(passphrase.clone());
    recipient.set_work_factor(get_scrypt_work_factor());

    let mut encrypted = Vec::new();
    let mut encryptor =
        Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|e| PyxError::Crypto(format!("Failed to create encryptor: {e}")))?
            .wrap_output(&mut encrypted)
            .map_err(|e| PyxError::Crypto(format!("Failed to wrap output: {e}")))?;

    encryptor
        .write_all(plaintext)
        .map_err(|e| PyxError::Crypto(format!("Encryption write failed: {e}")))?;

    encryptor
        .finish()
        .map_err(|e| PyxError::Crypto(format!("Encryption finish failed: {e}")))?;

    // Encode as base64 (matches Go implementation)
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &encrypted,
    ))
}

/// Decrypt data using age with scrypt passphrase
///
/// Accepts base64-encoded age ciphertext (matches Go implementation format)
pub fn decrypt_with_passphrase(ciphertext: &str, passphrase: &SecretString) -> Result<Vec<u8>> {
    let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, ciphertext)
        .map_err(|e| PyxError::Crypto(format!("Base64 decode failed: {e}")))?;

    // Accept high work factors from Go-generated data
    let mut identity = Identity::new(passphrase.clone());
    identity.set_max_work_factor(MAX_ACCEPTED_SCRYPT_WORK_FACTOR);

    let decryptor = Decryptor::new(encrypted.as_slice())
        .map_err(|e| PyxError::Crypto(format!("Failed to create decryptor: {e}")))?;

    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| {
            PyxError::Crypto("Decryption failed. Please check your passphrase.".to_string())
        })?;

    let mut decrypted = Vec::new();
    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| PyxError::Crypto(format!("Decryption read failed: {e}")))?;

    Ok(decrypted)
}

/// Encrypt data using key bytes (used for provider API key encryption).
/// Uses direct ChaCha20-Poly1305 with random nonce (v2 format).
pub fn encrypt_with_key(plaintext: &[u8], key: &[u8]) -> Result<String> {
    encrypt_direct(plaintext, key)
}

/// Decrypt data using key bytes (used for provider API key decryption).
/// Requires v2 format (`pyx2:` prefix).
pub fn decrypt_with_key(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
    if !ciphertext.starts_with(V2_PREFIX) {
        return Err(PyxError::Crypto(
            "Unsupported ciphertext format (requires v2/pyx2 prefix)".to_string(),
        ));
    }
    decrypt_direct(ciphertext, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
        let plaintext = b"Hello, World!";

        // Encrypt
        let ciphertext = encrypt_with_passphrase(plaintext, &passphrase).unwrap();
        assert!(!ciphertext.is_empty());

        // Verify base64 format
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &ciphertext).unwrap();

        // Decrypt
        let decrypted = decrypt_with_passphrase(&ciphertext, &passphrase).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let passphrase1 = SecretString::new("passphrase1".to_string().into_boxed_str());
        let passphrase2 = SecretString::new("passphrase2".to_string().into_boxed_str());
        let plaintext = b"Secret data";

        // Encrypt with first passphrase
        let ciphertext = encrypt_with_passphrase(plaintext, &passphrase1).unwrap();

        // Decrypt with wrong passphrase should fail
        let result = decrypt_with_passphrase(&ciphertext, &passphrase2);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let passphrase = SecretString::new("test-pass".to_string().into_boxed_str());
        let plaintext = b"";

        let ciphertext = encrypt_with_passphrase(plaintext, &passphrase).unwrap();
        let decrypted = decrypt_with_passphrase(&ciphertext, &passphrase).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_binary_data() {
        let passphrase = SecretString::new("test-pass".to_string().into_boxed_str());
        let plaintext: Vec<u8> = (0..=255u8).collect();

        let ciphertext = encrypt_with_passphrase(&plaintext, &passphrase).unwrap();
        let decrypted = decrypt_with_passphrase(&ciphertext, &passphrase).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_with_key_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"key-based encryption";

        let ciphertext = encrypt_with_key(plaintext, &key).unwrap();
        assert!(ciphertext.starts_with("pyx2:"));

        let decrypted = decrypt_with_key(&ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_with_key_rejects_non_32_byte_key() {
        let key = vec![0u8; 16];
        let result = encrypt_with_key(b"test", &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_with_key_rejects_legacy_format() {
        let key = [42u8; 32];
        // A non-pyx2-prefixed ciphertext should be rejected
        let result = decrypt_with_key("dGVzdA==", &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_with_key_empty() {
        let key = [7u8; 32];
        let plaintext = b"";

        let ciphertext = encrypt_with_key(plaintext, &key).unwrap();
        let decrypted = decrypt_with_key(&ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_key_wrong_key_fails() {
        let key = [42u8; 32];
        let wrong_key = [99u8; 32];
        let plaintext = b"secret data";

        let ciphertext = encrypt_with_key(plaintext, &key).unwrap();
        let result = decrypt_with_key(&ciphertext, &wrong_key);
        assert!(result.is_err());
    }
}
