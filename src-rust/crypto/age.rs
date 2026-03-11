//! Age encryption layer for pyx
//!
//! Uses age with scrypt passphrase encryption.
//! Compatible with age 0.11 API and Go implementation (base64-encoded output).

use crate::error::{PyxError, Result};
use age::{Encryptor, Decryptor};
use age::scrypt::{Recipient, Identity};
use secrecy::SecretString;
use std::io::{Read, Write};

/// Encrypt data using age with scrypt passphrase
///
/// Returns base64-encoded age ciphertext (matches Go implementation format)
pub fn encrypt_with_passphrase(
    plaintext: &[u8],
    passphrase: &SecretString,
) -> Result<String> {
    // Create scrypt recipient with SecretString (age 0.11 API)
    let recipient = Recipient::new(passphrase.clone());
    
    // Encrypt to binary
    let mut encrypted = Vec::new();
    let mut encryptor = Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
        .map_err(|e| PyxError::Crypto(format!("Failed to create encryptor: {}", e)))?
        .wrap_output(&mut encrypted)
        .map_err(|e| PyxError::Crypto(format!("Failed to wrap output: {}", e)))?;
    
    encryptor.write_all(plaintext)
        .map_err(|e| PyxError::Crypto(format!("Encryption write failed: {}", e)))?;
    
    encryptor.finish()
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
pub fn decrypt_with_passphrase(
    ciphertext: &str,
    passphrase: &SecretString,
) -> Result<Vec<u8>> {
    // Decode base64
    let encrypted = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        ciphertext,
    ).map_err(|e| PyxError::Crypto(format!("Base64 decode failed: {}", e)))?;
    
    // Create scrypt identity with SecretString (age 0.11 API)
    let identity = Identity::new(passphrase.clone());
    
    // Create decryptor
    let decryptor = Decryptor::new(encrypted.as_slice())
        .map_err(|e| PyxError::Crypto(format!("Failed to create decryptor: {}", e)))?;
    
    // Decrypt with identity iterator
    let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| PyxError::Crypto(format!("Decryption failed (wrong passphrase?): {}", e)))?;
    
    // Read decrypted data
    let mut decrypted = Vec::new();
    reader.read_to_end(&mut decrypted)
        .map_err(|e| PyxError::Crypto(format!("Decryption read failed: {}", e)))?;
    
    Ok(decrypted)
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
}
