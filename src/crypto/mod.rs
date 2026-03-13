//! Cryptography module for encryption/decryption

pub mod age;

pub use age::{decrypt_with_passphrase, encrypt_with_passphrase};
