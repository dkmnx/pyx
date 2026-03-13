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

/// Install pi if not already installed (auto-detect package manager)
pub fn install_pi_auto() -> Result<()> {
    install_pi_impl(None)
}

/// Install pi with package manager prompt
pub fn install_pi_with_prompt() -> Result<()> {
    // For now, auto-detect (prompt can be added later if needed)
    install_pi_impl(None)
}

/// Install pi with optional specific package manager
fn install_pi_impl(pm_override: Option<&str>) -> Result<()> {
    // Check if pi is already installed
    if find_pi().is_some() {
        println!("pi is already installed.");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {}", version);
        }
        return Ok(());
    }

    // Use specified package manager or detect
    let pm = if let Some(pm) = pm_override {
        pm.to_string()
    } else {
        detect_package_manager()?
    };

    println!("Installing pi using {}...", pm);

    // Install pi globally
    let status = Command::new(&pm)
        .args(["install", "-g", "@mariozechner/pi-coding-agent"])
        .status()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to run {}: {}", pm, e)))?;

    if status.success() {
        println!("✓ Installation complete");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {}", version);
        }
        Ok(())
    } else {
        Err(PyxError::CommandExecution(format!(
            "Failed to install pi. Exit code: {:?}",
            status.code()
        )))
    }
}

fn detect_package_manager() -> Result<String> {
    let package_managers = ["npm", "pnpm", "yarn", "bun"];
    let detected = package_managers.iter().find(|&pm| which::which(pm).is_ok());

    detected.map(|s| s.to_string()).ok_or_else(|| {
        PyxError::CommandExecution(
            "No package manager found (npm, pnpm, yarn, bun). Please install one first."
                .to_string(),
        )
    })
}

/// Install pi if not already installed (legacy function for backward compatibility)
pub fn install_pi() -> Result<()> {
    install_pi_auto()
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
