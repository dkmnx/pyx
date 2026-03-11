//! Execute pi process

use crate::error::Result;

/// Check if pi is available in PATH
pub fn find_pi() -> Option<String> {
    which::which("pi")
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
}

/// Spawn pi process with environment variables
pub fn spawn_pi(env_vars: &[(String, String)], args: &[String]) -> Result<i32> {
    // TODO: Implement pi process execution
    // - No shell invocation
    // - Pass environment variables explicitly
    // - Preserve child exit code
    // - Handle pass-through arguments
    
    let _ = (env_vars, args);
    Err(crate::error::PyxError::CommandExecution(
        "pi execution not yet implemented".to_string(),
    ))
}
