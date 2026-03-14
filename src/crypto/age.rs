//! Age encryption layer for pyx
//!
//! Uses age with scrypt passphrase encryption.
//! Compatible with age 0.11 API and Go implementation (base64-encoded output).

use crate::error::{PyxError, Result};
use age::scrypt::{Identity, Recipient};
use age::{DecryptError, Decryptor, EncryptError, Encryptor};
use age_core::format::{FileKey, Stanza, FILE_KEY_BYTES};
use age_core::primitives::{aead_decrypt, aead_encrypt};
use base64::Engine;
use scrypt::{scrypt, Params as ScryptParams};
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashSet;
use std::io::{Read, Write};

/// Scrypt work factor for encryption (N = 2^14 = 16384 iterations)
/// Lowered from 18 to 14 for better performance while maintaining adequate security.
/// For passphrase-based encryption, this provides a good balance between security and usability.
/// Individual provider key encryption uses separate random keys, so this work factor
/// primarily affects initial setup and master key operations.
const SCRYPT_WORK_FACTOR: u8 = 14;
/// Maximum accepted scrypt work factor for decryption (age library limit of 2^63)
const MAX_ACCEPTED_SCRYPT_WORK_FACTOR: u8 = 63;
const SCRYPT_TAG: &str = "scrypt";
const SCRYPT_SALT_LABEL: &[u8] = b"age-encryption.org/v1/scrypt";
/// 16-byte salt for scrypt key derivation
const SCRYPT_SALT_LEN: usize = 16;
/// File key length (32 bytes) + AES-GCM tag (16 bytes)
const ENCRYPTED_FILE_KEY_BYTES: usize = FILE_KEY_BYTES + 16;
/// Scrypt block size parameter (CPU/memory cost multiplier)
const SCRYPT_R: u32 = 8;
/// Scrypt parallelization parameter
const SCRYPT_P: u32 = 1;
const RAW_SCRYPT_LABEL: &str = "raw-scrypt";

/// Encrypt data using age with scrypt passphrase
///
/// Returns base64-encoded age ciphertext (matches Go implementation format)
pub fn encrypt_with_passphrase(plaintext: &[u8], passphrase: &SecretString) -> Result<String> {
    // Create scrypt recipient with explicit Go-compatible work factor.
    let mut recipient = Recipient::new(passphrase.clone());
    recipient.set_work_factor(SCRYPT_WORK_FACTOR);

    // Encrypt to binary
    let mut encrypted = Vec::new();
    let mut encryptor =
        Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|e| PyxError::Crypto(format!("Failed to create encryptor: {}", e)))?
            .wrap_output(&mut encrypted)
            .map_err(|e| PyxError::Crypto(format!("Failed to wrap output: {}", e)))?;

    encryptor
        .write_all(plaintext)
        .map_err(|e| PyxError::Crypto(format!("Encryption write failed: {}", e)))?;

    encryptor
        .finish()
        .map_err(|e| PyxError::Crypto(format!("Encryption finish failed: {}", e)))?;

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
    // Decode base64
    let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, ciphertext)
        .map_err(|e| PyxError::Crypto(format!("Base64 decode failed: {}", e)))?;

    // Create scrypt identity and accept high work factors from Go-generated data.
    let mut identity = Identity::new(passphrase.clone());
    identity.set_max_work_factor(MAX_ACCEPTED_SCRYPT_WORK_FACTOR);

    // Create decryptor
    let decryptor = Decryptor::new(encrypted.as_slice())
        .map_err(|e| PyxError::Crypto(format!("Failed to create decryptor: {}", e)))?;

    // Decrypt with identity iterator
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| PyxError::Crypto(format!("Decryption failed (wrong passphrase?): {}", e)))?;

    // Read decrypted data
    let mut decrypted = Vec::new();
    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| PyxError::Crypto(format!("Decryption read failed: {}", e)))?;

    Ok(decrypted)
}

#[derive(Clone)]
struct RawScryptRecipient {
    passphrase: Vec<u8>,
    log_n: u8,
}

impl RawScryptRecipient {
    fn new(passphrase: Vec<u8>, log_n: u8) -> Self {
        Self { passphrase, log_n }
    }
}

impl age::Recipient for RawScryptRecipient {
    fn wrap_file_key(
        &self,
        file_key: &FileKey,
    ) -> std::result::Result<(Vec<Stanza>, HashSet<String>), EncryptError> {
        let mut salt = [0u8; SCRYPT_SALT_LEN];
        getrandom::fill(&mut salt)
            .map_err(|_| EncryptError::Io(std::io::Error::other("random generation failed")))?;

        let mut inner_salt = Vec::with_capacity(SCRYPT_SALT_LABEL.len() + SCRYPT_SALT_LEN);
        inner_salt.extend_from_slice(SCRYPT_SALT_LABEL);
        inner_salt.extend_from_slice(&salt);

        let enc_key = derive_scrypt_key(&inner_salt, self.log_n, &self.passphrase)
            .map_err(|_| EncryptError::Io(std::io::Error::other("scrypt key derivation failed")))?;

        let encrypted_file_key = aead_encrypt(&enc_key, file_key.expose_secret());

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
            Ok(salt) if salt.len() == SCRYPT_SALT_LEN => salt,
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

        let mut inner_salt = Vec::with_capacity(SCRYPT_SALT_LABEL.len() + SCRYPT_SALT_LEN);
        inner_salt.extend_from_slice(SCRYPT_SALT_LABEL);
        inner_salt.extend_from_slice(&salt);

        let enc_key = match derive_scrypt_key(&inner_salt, log_n, &self.passphrase) {
            Ok(key) => key,
            Err(_) => return Some(Err(DecryptError::DecryptionFailed)),
        };

        let file_key_bytes = match aead_decrypt(&enc_key, FILE_KEY_BYTES, &stanza.body) {
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
        .map_err(|e| PyxError::Crypto(format!("Invalid scrypt params: {}", e)))?;

    let mut output = [0u8; 32];
    scrypt(passphrase, inner_salt, &params, &mut output)
        .map_err(|e| PyxError::Crypto(format!("Scrypt key derivation failed: {}", e)))?;

    Ok(output)
}

fn decrypt_with_raw_key_identity(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(ciphertext)
        .map_err(|e| PyxError::Crypto(format!("Base64 decode failed: {}", e)))?;

    let decryptor = Decryptor::new(encrypted.as_slice())
        .map_err(|e| PyxError::Crypto(format!("Failed to create decryptor: {}", e)))?;

    let identity = RawScryptIdentity::new(key.to_vec());
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| PyxError::Crypto(format!("Decryption failed (wrong passphrase?): {}", e)))?;

    let mut decrypted = Vec::new();
    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| PyxError::Crypto(format!("Decryption read failed: {}", e)))?;

    Ok(decrypted)
}

fn encrypt_with_raw_key_recipient(plaintext: &[u8], key: &[u8]) -> Result<String> {
    let recipient = RawScryptRecipient::new(key.to_vec(), SCRYPT_WORK_FACTOR);

    let mut encrypted = Vec::new();
    let mut encryptor =
        Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|e| PyxError::Crypto(format!("Failed to create encryptor: {}", e)))?
            .wrap_output(&mut encrypted)
            .map_err(|e| PyxError::Crypto(format!("Failed to wrap output: {}", e)))?;

    encryptor
        .write_all(plaintext)
        .map_err(|e| PyxError::Crypto(format!("Encryption write failed: {}", e)))?;

    encryptor
        .finish()
        .map_err(|e| PyxError::Crypto(format!("Encryption finish failed: {}", e)))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

/// Encrypt data using key bytes (used for provider API key encryption).
pub fn encrypt_with_key(plaintext: &[u8], key: &[u8]) -> Result<String> {
    encrypt_with_raw_key_recipient(plaintext, key)
}

/// Decrypt data using key bytes (used for provider API key decryption).
/// Uses raw byte identity for Go compatibility (no lossy string conversion).
pub fn decrypt_with_key(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
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
        let key = vec![0, 1, 2, 3, 255, 254, 128, 42];
        let plaintext = b"key-based encryption";

        let ciphertext = encrypt_with_key(plaintext, &key).unwrap();
        let decrypted = decrypt_with_key(&ciphertext, &key).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
