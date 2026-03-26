//! Input validation for security-sensitive operations.

use crate::error::PyxError;

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
