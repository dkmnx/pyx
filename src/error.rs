//! Error types for pyx

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PyxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Command execution error: {0}")]
    CommandExecution(String),

    #[error("Temp file persist error: {0}")]
    TempFilePersist(String),

    #[error("Cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, PyxError>;
