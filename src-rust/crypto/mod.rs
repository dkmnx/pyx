//! Cryptography module for encryption/decryption

pub mod age;

pub use age::{encrypt_with_passphrase, decrypt_with_passphrase};
