//! Execute pi process

use crate::error::{PyxError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Shell type for completion installation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellType::Bash => write!(f, "bash"),
            ShellType::Zsh => write!(f, "zsh"),
            ShellType::Fish => write!(f, "fish"),
            ShellType::PowerShell => write!(f, "powershell"),
        }
    }
}

impl std::str::FromStr for ShellType {
    type Err = PyxError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(ShellType::Bash),
            "zsh" => Ok(ShellType::Zsh),
            "fish" => Ok(ShellType::Fish),
            "powershell" | "pwsh" => Ok(ShellType::PowerShell),
            _ => Err(PyxError::Validation(format!("Unknown shell type: {}", s))),
        }
    }
}

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

/// Detect current shell type
pub fn detect_current_shell() -> ShellType {
    // Check SHELL environment variable
    if let Ok(shell) = env::var("SHELL") {
        let shell_path = PathBuf::from(&shell);
        if let Some(name) = shell_path.file_name().and_then(|n| n.to_str()) {
            match name {
                "bash" => return ShellType::Bash,
                "zsh" => return ShellType::Zsh,
                "fish" => return ShellType::Fish,
                _ => {}
            }
        }
    }

    // Check for fish-specific environment variable
    if env::var("__FISH_VERSION_DIR").is_ok() {
        return ShellType::Fish;
    }

    // Platform-specific defaults
    #[cfg(unix)]
    {
        ShellType::Bash
    }
    #[cfg(windows)]
    {
        ShellType::PowerShell
    }
    #[cfg(not(any(unix, windows)))]
    {
        ShellType::Bash
    }
}

/// Get completion script install path for a shell type
pub fn completion_script_install_path(shell: ShellType) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| PyxError::Config("Could not determine home directory".to_string()))?;

    let path = match shell {
        ShellType::Bash => home.join(".bash_completions").join("pyx.bash"),
        ShellType::Zsh => home.join(".zsh").join("completions").join("_pyx"),
        ShellType::Fish => home
            .join(".config")
            .join("fish")
            .join("completions")
            .join("pyx.fish"),
        ShellType::PowerShell => home.join("Documents").join("PowerShell").join("pyx.ps1"),
    };

    Ok(path)
}

/// Install shell completion for pi using pyx
pub fn install_completion() -> Result<()> {
    let shell = detect_current_shell();

    // Generate completion script using pyx
    let output = Command::new("pyx")
        .args(["completion", &shell.to_string()])
        .output()
        .map_err(|e| {
            PyxError::CommandExecution(format!("Failed to generate completion script: {}", e))
        })?;

    if !output.status.success() {
        return Err(PyxError::CommandExecution(format!(
            "Failed to generate completion script: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // Get install path
    let script_path = match completion_script_install_path(shell) {
        Ok(path) => path,
        Err(_) => {
            println!("Completion installation not available for {} shell", shell);
            return Ok(());
        }
    };

    // Skip if already installed
    if script_path.exists() {
        return Ok(());
    }

    // Ensure directory exists
    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write completion script
    fs::write(&script_path, &output.stdout)?;

    println!("✓ Completion script installed for {} shell", shell);
    println!("  Script location: {}", script_path.display());

    // Print activation instructions
    match shell {
        ShellType::Zsh => {
            println!("  To enable completions, restart your shell or run:");
            println!("    autoload -U compinit; compinit");
        }
        ShellType::Fish => {
            println!("  To enable completions, restart your shell or run:");
            println!("    source \"{}\"", script_path.display());
        }
        ShellType::PowerShell => {
            println!("  To enable completions for every new session, add to your profile:");
            println!(
                "    Add-Content -Path $PROFILE -Value '. {}'",
                script_path.display()
            );
        }
        ShellType::Bash => {
            println!("  Completions will be loaded automatically");
        }
    }

    Ok(())
}

/// Platform info string (OS/ARCH)
pub fn platform_info() -> String {
    #[cfg(unix)]
    {
        format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
    }
    #[cfg(windows)]
    {
        format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
    }
    #[cfg(not(any(unix, windows)))]
    {
        format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_pi_returns_option() {
        // find_pi returns Option<String> - verify the interface works
        // The actual result depends on whether pi is in PATH
        let result = find_pi();
        // Should return Some(path) if pi is installed, None otherwise
        // Just verify it doesn't panic and returns correct type
        match result {
            Some(path) => {
                // If found, verify it's a non-empty string
                assert!(!path.is_empty());
            }
            None => {
                // If not found, that's also valid (pi not installed)
            }
        }
    }

    #[test]
    fn test_platform_info_known_values() {
        let info = platform_info();
        // Platform info should be non-empty and contain OS/arch format
        assert!(!info.is_empty());
        assert!(info.contains('/'));
    }

    #[test]
    fn test_detect_current_shell() {
        let shell = detect_current_shell();
        // Should return a valid shell type
        match shell {
            ShellType::Bash | ShellType::Zsh | ShellType::Fish | ShellType::PowerShell => {}
        }
    }
}
