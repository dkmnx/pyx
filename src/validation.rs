use crate::error::PyxError;

/// Validates a provider name.
///
/// Provider names must be:
/// - 1-50 characters
/// - Alphanumeric with hyphens and underscores allowed
pub fn validate_provider_name(name: &str) -> Result<(), PyxError> {
    if name.is_empty() {
        return Err(PyxError::Config(
            "Provider name cannot be empty".to_string(),
        ));
    }
    if name.len() > 50 {
        return Err(PyxError::Config(format!(
            "Provider name '{}' exceeds 50 character limit",
            name
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PyxError::Config(format!(
            "Provider name '{}' contains invalid characters. Only alphanumeric, hyphens, and underscores allowed",
            name
        )));
    }
    Ok(())
}

/// Validates an environment variable name.
///
/// Env var names must:
/// - Start with letter or underscore
/// - Contain only letters, digits, and underscores
/// - Be 1-100 characters
pub fn validate_env_var_name(name: &str) -> Result<(), PyxError> {
    if name.is_empty() {
        return Err(PyxError::Config(
            "Environment variable name cannot be empty".to_string(),
        ));
    }
    if name.len() > 100 {
        return Err(PyxError::Config(format!(
            "Environment variable name '{}' exceeds 100 character limit",
            name
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_alphabetic() || c.is_numeric() || c == '_')
    {
        return Err(PyxError::Config(format!(
            "Environment variable name '{}' contains invalid characters. Only letters, digits, and underscores allowed",
            name
        )));
    }
    // Must start with letter or underscore
    if name.chars().next().is_none_or(|c| c.is_numeric()) {
        return Err(PyxError::Config(format!(
            "Environment variable name '{}' must start with a letter or underscore",
            name
        )));
    }
    Ok(())
}

/// Validates keyring service and username arguments.
///
/// These are used as CLI arguments for secret-tool (Linux), security (macOS),
/// and as target names for WinCred (Windows). We sanitize to prevent:
/// - Command injection via newlines or special characters (Linux)
/// - Target name parsing issues (all platforms)
pub fn validate_keyring_args(service: &str, username: &str) -> Result<(), PyxError> {
    if service.is_empty() || username.is_empty() {
        return Err(PyxError::Keyring(
            "Service and username cannot be empty".to_string(),
        ));
    }
    // Check for newlines or control characters that could cause CLI injection
    if service
        .chars()
        .any(|c| c.is_control() || c == '\n' || c == '\r' || c == '\0')
    {
        return Err(PyxError::Keyring(format!(
            "Service name contains invalid characters: {:?}",
            service
        )));
    }
    if username
        .chars()
        .any(|c| c.is_control() || c == '\n' || c == '\r' || c == '\0')
    {
        return Err(PyxError::Keyring(format!(
            "Username contains invalid characters: {:?}",
            username
        )));
    }
    // Limit length to prevent issues with system APIs
    if service.len() > 256 {
        return Err(PyxError::Keyring(format!(
            "Service name '{}' exceeds 256 character limit",
            service
        )));
    }
    if username.len() > 256 {
        return Err(PyxError::Keyring(format!(
            "Username '{}' exceeds 256 character limit",
            username
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_provider_name_valid() {
        assert!(validate_provider_name("openai").is_ok());
        assert!(validate_provider_name("google-vertex").is_ok());
        assert!(validate_provider_name("my_provider_123").is_ok());
        assert!(validate_provider_name("a").is_ok());
    }

    #[test]
    fn test_validate_provider_name_invalid() {
        assert!(validate_provider_name("").is_err());
        assert!(validate_provider_name("has space").is_err());
        assert!(validate_provider_name("has.dot").is_err());
        assert!(validate_provider_name("has/slash").is_err());
        assert!(validate_provider_name("has@at").is_err());
    }

    #[test]
    fn test_validate_provider_name_too_long() {
        let long_name = "a".repeat(51);
        assert!(validate_provider_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_env_var_name_valid() {
        assert!(validate_env_var_name("OPENAI_API_KEY").is_ok());
        assert!(validate_env_var_name("MY_KEY").is_ok());
        assert!(validate_env_var_name("_PRIVATE").is_ok());
        assert!(validate_env_var_name("KEY1").is_ok());
    }

    #[test]
    fn test_validate_env_var_name_invalid() {
        assert!(validate_env_var_name("").is_err());
        assert!(validate_env_var_name("123_STARTS_WITH_DIGIT").is_err());
        assert!(validate_env_var_name("HAS-UNDERSCORE").is_err());
        assert!(validate_env_var_name("HAS SPACE").is_err());
    }

    #[test]
    fn test_validate_keyring_args_valid() {
        assert!(validate_keyring_args("pyx", "master-key").is_ok());
        assert!(validate_keyring_args("service", "user").is_ok());
    }

    #[test]
    fn test_validate_keyring_args_empty() {
        assert!(validate_keyring_args("", "user").is_err());
        assert!(validate_keyring_args("service", "").is_err());
    }

    #[test]
    fn test_validate_keyring_args_newline() {
        assert!(validate_keyring_args("service\n", "user").is_err());
        assert!(validate_keyring_args("service", "user\r").is_err());
        assert!(validate_keyring_args("service", "user\0").is_err());
    }
}
