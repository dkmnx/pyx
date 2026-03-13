//! Pyx Rust - Secure API key management for pi
//!
//! This is a Rust rewrite of the Go-based pyx CLI tool.

pub mod cli;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod keys;
pub mod models;
pub mod pi;
pub mod prompt;
pub mod providers;
pub mod root_args;
pub mod session;
pub mod storage;
pub mod validation;

// Re-export commonly used types
pub use keys::manager::KeyManager;
pub use storage::database::{Database, ProviderEntry};
pub use storage::models_cache::ModelsCache;

/// Global mutex for tests that modify environment variables
#[cfg(test)]
pub static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
