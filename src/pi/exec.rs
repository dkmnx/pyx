//! Execute pi process

use crate::error::{PyxError, Result};
use clap::ValueEnum;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Shell type for completion installation
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellType::Bash => write!(f, "bash"),
            ShellType::Zsh => write!(f, "zsh"),
            ShellType::Fish => write!(f, "fish"),
            ShellType::Powershell => write!(f, "powershell"),
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
            "powershell" | "pwsh" => Ok(ShellType::Powershell),
            _ => Err(PyxError::Validation(format!("Unknown shell type: {s}"))),
        }
    }
}

impl ShellType {
    /// Convert to clap_complete Shell type
    pub fn to_clap_complete_shell(&self) -> clap_complete::Shell {
        match self {
            ShellType::Bash => clap_complete::Shell::Bash,
            ShellType::Zsh => clap_complete::Shell::Zsh,
            ShellType::Fish => clap_complete::Shell::Fish,
            ShellType::Powershell => clap_complete::Shell::PowerShell,
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

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let status = cmd
        .status()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to execute pi: {e}")))?;

    Ok(status.code().unwrap_or(1))
}

/// Install pi with package manager prompt (auto-selects if only one PM found)
pub fn install_pi_with_prompt() -> Result<()> {
    if find_pi().is_some() {
        println!("pi is already installed.");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {version}");
        }
        return Ok(());
    }

    let available: Vec<String> = ["npm", "pnpm", "yarn", "bun"]
        .iter()
        .filter(|pm| which::which(pm).is_ok())
        .map(|pm| pm.to_string())
        .collect();

    if available.is_empty() {
        return Err(PyxError::CommandExecution(
            "No package manager found (npm, pnpm, yarn, bun). Please install one first."
                .to_string(),
        ));
    }

    let pm = if available.len() == 1 {
        available[0].clone()
    } else {
        let choice = inquire::Select::new("Select a package manager", available.clone())
            .prompt()
            .map_err(|e| PyxError::Validation(format!("Failed to read selection: {e}")))?;
        choice
    };

    install_pi_impl(Some(&pm))
}

/// Install pi with optional specific package manager
fn install_pi_impl(pm_override: Option<&str>) -> Result<()> {
    if find_pi().is_some() {
        println!("pi is already installed.");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {version}");
        }
        return Ok(());
    }

    let pm = if let Some(pm) = pm_override {
        pm.to_string()
    } else {
        detect_package_manager()?
    };

    println!("Installing pi using {pm}...");

    let status = Command::new(&pm)
        .args(["install", "-g", "@mariozechner/pi-coding-agent"])
        .status()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to run {pm}: {e}")))?;

    if status.success() {
        println!("✓ Installation complete");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {version}");
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

/// Install pi if not already installed (auto-detect package manager)
pub fn install_pi() -> Result<()> {
    install_pi_with_prompt()
}

/// Check pi version
pub fn get_pi_version() -> Result<String> {
    let pi_path =
        find_pi().ok_or_else(|| PyxError::CommandExecution("pi not found in PATH".to_string()))?;

    let output = Command::new(&pi_path)
        .arg("--version")
        .output()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to get pi version: {e}")))?;

    let version = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let version = if version.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        version
    };

    Ok(version)
}

/// Show pi status (installed/not installed, version)
pub fn show_pi_status() -> Result<()> {
    match find_pi() {
        Some(path) => {
            println!("pi is installed");
            println!("  Path: {path}");
            if let Ok(version) = get_pi_version() {
                println!("  Version: {version}");
            }
            println!("  Platform: {}", platform_info());
        }
        None => {
            println!("pi is not installed");
            println!();
            println!("To install pi, run:");
            println!("  pyx pi install");
        }
    }
    Ok(())
}

/// Detect current shell type
pub fn detect_current_shell() -> ShellType {
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

    if env::var("__FISH_VERSION_DIR").is_ok() {
        return ShellType::Fish;
    }

    #[cfg(unix)]
    {
        ShellType::Bash
    }
    #[cfg(windows)]
    {
        ShellType::Powershell
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
        ShellType::Powershell => home.join("Documents").join("PowerShell").join("pyx.ps1"),
    };

    Ok(path)
}

/// Install shell completion for the specified shell
pub fn install_completion_for_shell(shell: ShellType) -> Result<()> {
    let output = Command::new("pyx")
        .args(["completion", &shell.to_string()])
        .output()
        .map_err(|e| {
            PyxError::CommandExecution(format!("Failed to generate completion script: {e}"))
        })?;

    if !output.status.success() {
        return Err(PyxError::CommandExecution(format!(
            "Failed to generate completion script: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let script_path = completion_script_install_path(shell)?;

    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&script_path, &output.stdout)?;

    println!("✓ Completion script installed for {shell} shell");
    println!("  Script location: {}", script_path.display());

    match shell {
        ShellType::Zsh => {
            println!();
            println!("  To enable completions, add to ~/.zshrc:");
            println!(
                "    fpath=({} $fpath)",
                script_path.parent().unwrap().display()
            );
            println!();
            println!("  Then restart your shell or run:");
            println!("    autoload -U compinit; compinit");
        }
        ShellType::Fish => {
            println!();
            println!("  Completions will be loaded automatically in new shell sessions.");
        }
        ShellType::Powershell => {
            println!();
            println!("  To enable completions for every new session, add to your profile:");
            println!(
                "    Add-Content -Path $PROFILE -Value '. {}'",
                script_path.display()
            );
        }
        ShellType::Bash => {
            println!();
            println!("  To enable completions, add to ~/.bashrc:");
            println!(
                "    [ -f {} ] && source {}",
                script_path.display(),
                script_path.display()
            );
        }
    }

    Ok(())
}

/// Install shell completion for pi using pyx (auto-detect current shell)
pub fn install_completion() -> Result<()> {
    let shell = detect_current_shell();
    install_completion_for_shell(shell)
}

/// Platform info string (OS/ARCH)
pub fn platform_info() -> String {
    format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ENV_MUTEX;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_pi_returns_option() {
        let result = find_pi();
        match result {
            Some(path) => {
                assert!(!path.is_empty());
            }
            None => {}
        }
    }

    #[test]
    fn test_platform_info_known_values() {
        let info = platform_info();
        assert!(!info.is_empty());
        assert!(info.contains('/'));
    }

    #[test]
    fn test_detect_current_shell() {
        let shell = detect_current_shell();
        match shell {
            ShellType::Bash | ShellType::Zsh | ShellType::Fish | ShellType::Powershell => {}
        }
    }

    #[test]
    fn test_install_completion_for_shell_writes_generated_script() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let fake_pyx = bin_dir.join("pyx");
        let script = r#"#!/usr/bin/env bash
printf '%s\n' '# bash completion for pyx'
"#;
        fs::write(&fake_pyx, script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&fake_pyx, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        let new_path = if original_path.is_empty() {
            bin_dir.display().to_string()
        } else {
            format!("{}:{}", bin_dir.display(), original_path)
        };

        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("PATH", new_path);
        }

        install_completion_for_shell(ShellType::Bash).unwrap();

        let installed = home.join(".bash_completions").join("pyx.bash");
        assert!(installed.exists());
        assert!(fs::read_to_string(installed)
            .unwrap()
            .contains("bash completion for pyx"));

        unsafe {
            std::env::set_var("PATH", original_path);
            std::env::remove_var("HOME");
        }
    }
}
