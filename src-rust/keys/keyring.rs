//! OS keyring integration

use crate::error::{PyxError, Result};
use secrecy::SecretString;

const SERVICE_NAME: &str = "ply";
const USER_NAME: &str = "master-key";

/// Get passphrase from OS keyring
pub fn get_passphrase() -> Result<Option<SecretString>> {
    // TODO: Implement keyring access using keyring crate
    // Fallback to PLY_PASSPHRASE env var
    // Legacy fallback: "default"
    
    Err(PyxError::Keyring("Keyring access not yet implemented".to_string()))
}

/// Store passphrase in OS keyring
pub fn set_passphrase(passphrase: &SecretString) -> Result<()> {
    // TODO: Implement keyring storage
    
    let _ = passphrase;
    Err(PyxError::Keyring("Keyring access not yet implemented".to_string()))
}

/// Clear passphrase from OS keyring
pub fn clear_passphrase() -> Result<()> {
    // TODO: Implement keyring deletion
    Err(PyxError::Keyring("Keyring access not yet implemented".to_string()))
}
