//! Provider name and env var validation

use crate::error::{PyxError, Result};
use regex::Regex;
use std::sync::LazyLock;

/// Validate provider name
/// Must match: ^[a-zA-Z0-9_-]{1,50}$
pub fn validate_provider_name(name: &str) -> Result<()> {
    static PROVIDER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[a-zA-Z0-9_-]{1,50}$").expect("provider regex should be valid")
    });

    if PROVIDER_REGEX.is_match(name) {
        Ok(())
    } else {
        Err(PyxError::Validation(format!(
            "Invalid provider name: '{name}' - must match ^[a-zA-Z0-9_-]{{1,50}}$"
        )))
    }
}

/// Validate environment variable name
/// Must match: ^[A-Z_][A-Z0-9_]*$
pub fn validate_env_var(name: &str) -> Result<()> {
    static ENV_VAR_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Z_][A-Z0-9_]*$").expect("env var regex should be valid"));

    if ENV_VAR_REGEX.is_match(name) {
        Ok(())
    } else {
        Err(PyxError::Validation(format!(
            "Invalid environment variable name: '{name}' - must match ^[A-Z_][A-Z0-9_]*$"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_provider_names() {
        assert!(validate_provider_name("openai").is_ok());
        assert!(validate_provider_name("anthropic").is_ok());
        assert!(validate_provider_name("google-vertex").is_ok());
        assert!(validate_provider_name("minimax-cn").is_ok());
        assert!(validate_provider_name("qwen-cli").is_ok());
    }

    #[test]
    fn test_invalid_provider_names() {
        assert!(validate_provider_name("").is_err());
        assert!(validate_provider_name("open ai").is_err());
        assert!(validate_provider_name("open@ai").is_err());
    }

    #[test]
    fn test_valid_env_vars() {
        assert!(validate_env_var("OPENAI_API_KEY").is_ok());
        assert!(validate_env_var("ANTHROPIC_API_KEY").is_ok());
        assert!(validate_env_var("GOOGLE_VERTEX_API_KEY").is_ok());
    }

    #[test]
    fn test_invalid_env_vars() {
        assert!(validate_env_var("").is_err());
        assert!(validate_env_var("openai_api_key").is_err());
        assert!(validate_env_var("OPENAI-API-KEY").is_err());
    }
}
