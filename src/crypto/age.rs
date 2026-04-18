//! Age encryption layer for pyx
//!
//! Uses age with scrypt passphrase encryption.
//! Compatible with age 0.11 API and Go implementation (base64-encoded output).
//!
//! # Key-Based Encryption
//!
//! For key-based encryption (`encrypt_with_key`/`decrypt_with_key`), the current
//! format uses direct ChaCha20-Poly1305 with a random nonce (v2). Legacy entries
//! encrypted with scrypt-based key wrapping are transparently decrypted via the
//! `pyx2:` format prefix.

use crate::crypto::get_scrypt_work_factor_with_warning;
use crate::error::{PyxError, Result};
use age::scrypt::{Identity, Recipient};
use age::{DecryptError, Decryptor, Encryptor};

#[cfg(test)]
use age::EncryptError;
use age_core::format::{FileKey, Stanza};
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305,
};
use scrypt::{scrypt, Params as ScryptParams};
use secrecy::SecretString;

#[cfg(test)]
use secrecy::ExposeSecret;
use std::io::{Read, Write};

#[cfg(test)]
use std::collections::HashSet;

/// Default scrypt work factor (N = 2^18 = 262144 iterations)
/// This provides strong protection against brute-force attacks while maintaining
/// acceptable performance for interactive use.
/// Can be overridden via PYX_SCRYPT_WORK_FACTOR environment variable (value 15-30).
const DEFAULT_SCRYPT_WORK_FACTOR: u8 = 18;
const MIN_SCRYPT_WORK_FACTOR: u8 = 15;

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
const SCRYPT_TAG: &str = "scrypt";
const SCRYPT_SALT_LABEL: &[u8] = b"age-encryption.org/v1/scrypt";
#[cfg(test)]
/// 16-byte salt for scrypt key derivation
/// Can be overridden via PYX_SCRYPT_SALT_LEN environment variable (value 8-32).
const DEFAULT_SCRYPT_SALT_LEN: usize = 16;

#[cfg(test)]
/// Get scrypt salt length from environment or use default
fn get_scrypt_salt_len() -> usize {
    std::env::var("PYX_SCRYPT_SALT_LEN")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| (8..=32).contains(&n))
        .unwrap_or(DEFAULT_SCRYPT_SALT_LEN)
}

/// File key length (16 bytes for ChaCha20-Poly1305, matching age_core)
const FILE_KEY_BYTES: usize = 16;
/// ChaCha20-Poly1305 tag size (16 bytes)
const CHACHA_TAG_BYTES: usize = 16;
/// File key length + ChaCha20-Poly1305 tag
const ENCRYPTED_FILE_KEY_BYTES: usize = FILE_KEY_BYTES + CHACHA_TAG_BYTES;
/// Scrypt block size parameter (CPU/memory cost multiplier)
const SCRYPT_R: u32 = 8;
/// Scrypt parallelization parameter
const SCRYPT_P: u32 = 1;
#[cfg(test)]
const RAW_SCRYPT_LABEL: &str = "raw-scrypt";

const V2_PREFIX: &str = "pyx2:";
const V2_NONCE_BYTES: usize = 12;

/// Zero nonce for ChaCha20-Poly1305 (matching age_core internal implementation)
const ZERO_NONCE: &[u8; 12] = &[0; 12];

/// AEAD decryption using ChaCha20-Poly1305 with zero nonce.
/// Matches age_core::primitives::aead_decrypt behavior.
fn aead_decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(ZERO_NONCE.into(), ciphertext)
        .map_err(|_| PyxError::Crypto("Decryption failed (wrong key?)".to_string()))
}

#[cfg(test)]
fn aead_encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher
        .encrypt(ZERO_NONCE.into(), plaintext)
        .map_err(|e| PyxError::Crypto(format!("AEAD encryption failed: {e}")))
}

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

#[cfg(test)]
struct RawScryptRecipient {
    passphrase: Vec<u8>,
    log_n: u8,
}

#[cfg(test)]
impl RawScryptRecipient {
    fn new(passphrase: Vec<u8>, log_n: u8) -> Self {
        Self { passphrase, log_n }
    }
}

#[cfg(test)]
impl age::Recipient for RawScryptRecipient {
    fn wrap_file_key(
        &self,
        file_key: &FileKey,
    ) -> std::result::Result<(Vec<Stanza>, HashSet<String>), EncryptError> {
        let salt_len = get_scrypt_salt_len();
        let mut salt = vec![0u8; salt_len];
        getrandom::fill(&mut salt)
            .map_err(|_| EncryptError::Io(std::io::Error::other("random generation failed")))?;

        let mut inner_salt = Vec::with_capacity(SCRYPT_SALT_LABEL.len() + salt_len);
        inner_salt.extend_from_slice(SCRYPT_SALT_LABEL);
        inner_salt.extend_from_slice(&salt);

        let enc_key = derive_scrypt_key(&inner_salt, self.log_n, &self.passphrase)
            .map_err(|_| EncryptError::Io(std::io::Error::other("scrypt key derivation failed")))?;

        let encrypted_file_key = aead_encrypt(&enc_key, file_key.expose_secret()).map_err(|e| {
            EncryptError::Io(std::io::Error::other(format!("encryption failed: {e}")))
        })?;

        let stanza = Stanza {
            tag: SCRYPT_TAG.to_string(),
            args: vec![
                base64::engine::general_purpose::STANDARD_NO_PAD.encode(salt),
                self.log_n.to_string(),
            ],
            body: encrypted_file_key,
        };

        Ok((
            vec![stanza],
            std::iter::once(RAW_SCRYPT_LABEL.to_string()).collect(),
        ))
    }
}

#[cfg(test)]
fn encrypt_with_raw_key_recipient(plaintext: &[u8], key: &[u8]) -> Result<String> {
    let recipient = RawScryptRecipient::new(key.to_vec(), get_scrypt_work_factor());

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

    Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

/// Custom scrypt-based identity for legacy key-based decryption.
/// Uses ChaCha20-Poly1305 for decryption operations, avoiding age_core internals.
/// Required for backward compatibility with entries encrypted by the Go implementation
/// or older pyx versions that used scrypt-based key wrapping.
#[derive(Clone)]
struct RawScryptIdentity {
    passphrase: Vec<u8>,
    max_work_factor: u8,
}

impl RawScryptIdentity {
    fn new(passphrase: Vec<u8>) -> Self {
        Self {
            passphrase,
            max_work_factor: MAX_ACCEPTED_SCRYPT_WORK_FACTOR,
        }
    }
}

impl age::Identity for RawScryptIdentity {
    fn unwrap_stanza(&self, stanza: &Stanza) -> Option<std::result::Result<FileKey, DecryptError>> {
        if stanza.tag != SCRYPT_TAG {
            return None;
        }

        let (salt_arg, log_n_arg) = match &stanza.args[..] {
            [salt, log_n] => (salt, log_n),
            _ => return Some(Err(DecryptError::InvalidHeader)),
        };

        let salt = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(salt_arg) {
            Ok(salt) if !salt.is_empty() && salt.len() <= 32 => salt,
            _ => return Some(Err(DecryptError::InvalidHeader)),
        };

        let log_n = match log_n_arg.parse::<u8>() {
            Ok(value) if value > 0 && value < 64 => value,
            _ => return Some(Err(DecryptError::InvalidHeader)),
        };

        if stanza.body.len() != ENCRYPTED_FILE_KEY_BYTES {
            return Some(Err(DecryptError::InvalidHeader));
        }

        if log_n > self.max_work_factor {
            return Some(Err(DecryptError::ExcessiveWork {
                required: log_n,
                target: self.max_work_factor,
            }));
        }

        let mut inner_salt = Vec::with_capacity(SCRYPT_SALT_LABEL.len() + salt.len());
        inner_salt.extend_from_slice(SCRYPT_SALT_LABEL);
        inner_salt.extend_from_slice(&salt);

        let enc_key = match derive_scrypt_key(&inner_salt, log_n, &self.passphrase) {
            Ok(key) => key,
            Err(_) => return Some(Err(DecryptError::DecryptionFailed)),
        };

        let file_key_bytes = match aead_decrypt(&enc_key, &stanza.body) {
            Ok(bytes) => bytes,
            Err(_) => return Some(Err(DecryptError::DecryptionFailed)),
        };

        let mut array = [0u8; FILE_KEY_BYTES];
        array.copy_from_slice(&file_key_bytes);
        Some(Ok(FileKey::new(Box::new(array))))
    }
}

fn derive_scrypt_key(inner_salt: &[u8], log_n: u8, passphrase: &[u8]) -> Result<[u8; 32]> {
    let params = ScryptParams::new(log_n, SCRYPT_R, SCRYPT_P, 32)
        .map_err(|e| PyxError::Crypto(format!("Invalid scrypt params: {e}")))?;

    let mut output = [0u8; 32];
    scrypt(passphrase, inner_salt, &params, &mut output)
        .map_err(|e| PyxError::Crypto(format!("Scrypt key derivation failed: {e}")))?;

    Ok(output)
}

fn decrypt_with_raw_key_identity(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(ciphertext)
        .map_err(|e| PyxError::Crypto(format!("Base64 decode failed: {e}")))?;

    let decryptor = Decryptor::new(encrypted.as_slice())
        .map_err(|e| PyxError::Crypto(format!("Failed to create decryptor: {e}")))?;

    let identity = RawScryptIdentity::new(key.to_vec());
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| PyxError::Crypto(format!("Decryption failed (wrong key?): {e}")))?;

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
/// Detects v2 format by prefix and uses fast direct decryption.
/// Falls back to scrypt-based decryption for legacy entries.
pub fn decrypt_with_key(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
    if ciphertext.starts_with(V2_PREFIX) {
        return decrypt_direct(ciphertext, key);
    }
    decrypt_with_raw_key_identity(ciphertext, key)
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
    fn test_decrypt_with_key_legacy_format() {
        let key = [42u8; 32];
        let plaintext = b"legacy-encrypted-data";
        let old_cipher = encrypt_with_raw_key_recipient(plaintext, &key).unwrap();
        assert!(!old_cipher.starts_with("pyx2:"));

        let decrypted = decrypt_with_key(&old_cipher, &key).unwrap();
        assert_eq!(decrypted, plaintext);
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
