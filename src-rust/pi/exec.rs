//! Execute pi process

use crate::error::{PyxError, Result};
use std::process::Command;

/// Check if pi is available in PATH
pub fn find_pi() -> Option<String> {
    which::which("pi")
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
}

/// Spawn pi process with environment variables
pub fn spawn_pi(env_vars: &[(String, String)], args: &[String]) -> Result<i32> {
    let pi_path = find_pi().ok_or_else(|| {
        PyxError::CommandExecution(
            "pi not found in PATH. Run 'pyx pi install' to install.".to_string(),
        )
    })?;

    let mut cmd = Command::new(&pi_path);
    cmd.args(args);

    // Inject environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Execute and preserve exit code
    let status = cmd
        .status()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to execute pi: {}", e)))?;

    Ok(status.code().unwrap_or(1))
}

/// Install pi if not already installed
pub fn install_pi() -> Result<()> {
    // Check if pi is already installed
    if find_pi().is_some() {
        println!("pi is already installed.");
        return Ok(());
    }

    // Detect package manager
    let package_managers = ["npm", "pnpm", "yarn", "bun"];
    let detected = package_managers.iter().find(|&pm| which::which(pm).is_ok());

    let pm = detected.ok_or_else(|| {
        PyxError::CommandExecution(
            "No package manager found (npm, pnpm, yarn, bun). Please install one first."
                .to_string(),
        )
    })?;

    println!("Installing pi using {}...", pm);

    // Install pi globally
    let status = Command::new(pm)
        .args(["install", "-g", "@anthropics/pi"])
        .status()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to run {}: {}", pm, e)))?;

    if status.success() {
        println!("✓ pi installed successfully");
        Ok(())
    } else {
        Err(PyxError::CommandExecution(format!(
            "Failed to install pi. Exit code: {:?}",
            status.code()
        )))
    }
}

/// Check pi version
pub fn get_pi_version() -> Result<String> {
    let pi_path =
        find_pi().ok_or_else(|| PyxError::CommandExecution("pi not found in PATH".to_string()))?;

    let output = Command::new(&pi_path)
        .arg("--version")
        .output()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to get pi version: {}", e)))?;

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_pi() {
        // This test will pass or fail depending on whether pi is installed
        let result = find_pi();
        println!("pi found: {:?}", result.is_some());
    }
}
