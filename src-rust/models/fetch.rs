//! Fetch models from remote source

use crate::error::Result;

/// Fetch models from pi-mono or other remote source
pub fn fetch_models() -> Result<Vec<String>> {
    // TODO: Implement model fetching
    Err(crate::error::PyxError::Network(
        "Model fetching not yet implemented".to_string(),
    ))
}
