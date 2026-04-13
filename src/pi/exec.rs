//! Execute pi process

use crate::error::{PyxError, Result};
use clap::ValueEnum;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PACKAGE_MANAGERS: &[&str] = &["npm", "pnpm", "yarn", "bun"];
const PI_PACKAGE_NAME: &str = "@mariozechner/pi-coding-agent";

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
pub fn install_pi_with_prompt(force: bool) -> Result<()> {
    if !force && find_pi().is_some() {
        println!("pi is already installed.");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {version}");
        }
        return Ok(());
    }

    let available: Vec<String> = PACKAGE_MANAGERS
        .iter()
        .filter(|pm| which::which(pm).is_ok())
        .map(|pm| pm.to_string())
        .collect();

    if available.is_empty() {
        return Err(no_package_manager_error());
    }

    let pm = if available.len() == 1 {
        available[0].clone()
    } else {
        let choice = inquire::Select::new("Select a package manager", available.clone())
            .prompt()
            .map_err(|e| PyxError::Validation(format!("Failed to read selection: {e}")))?;
        choice
    };

    install_pi_impl(Some(&pm), force)
}

/// Install pi with optional specific package manager
fn install_pi_impl(pm_override: Option<&str>, force: bool) -> Result<()> {
    if !force && find_pi().is_some() {
        println!("pi is already installed.");
        if let Ok(version) = get_pi_version() {
            println!("pi version: {version}");
        }
        return Ok(());
    }

    let pm = match pm_override {
        Some(pm) => pm.to_string(),
        None => detect_package_manager()?,
    };

    println!("Installing pi using {pm}...");

    let status = Command::new(&pm)
        .args(["install", "-g", PI_PACKAGE_NAME])
        .status()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to run {pm}: {e}")))?;

    if !status.success() {
        return Err(PyxError::CommandExecution(format!(
            "Failed to install pi. Exit code: {:?}",
            status.code()
        )));
    }

    println!("✓ Installation complete");
    if let Ok(version) = get_pi_version() {
        println!("pi version: {version}");
    }
    Ok(())
}

fn detect_package_manager() -> Result<String> {
    PACKAGE_MANAGERS
        .iter()
        .find(|pm| which::which(pm).is_ok())
        .map(|s| s.to_string())
        .ok_or_else(no_package_manager_error)
}

fn no_package_manager_error() -> PyxError {
    PyxError::CommandExecution(
        "No package manager found (npm, pnpm, yarn, bun). Please install one first.".to_string(),
    )
}

/// Install pi if not already installed (auto-detect package manager)
pub fn install_pi() -> Result<()> {
    install_pi_with_prompt(false)
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
    use crate::test_helpers::EnvGuard;
    use crate::ENV_MUTEX;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_pi_returns_option() {
        let result = find_pi();
        if let Some(path) = result {
            assert!(!path.is_empty());
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
    #[cfg(unix)]
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

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake_pyx, fs::Permissions::from_mode(0o755)).unwrap();

        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();

        let mut path_guard = EnvGuard::set_var("HOME", &home);
        let current_path = std::env::var("PATH").unwrap_or_default();
        let new_path = if current_path.is_empty() {
            bin_dir.display().to_string()
        } else {
            format!("{}:{}", bin_dir.display(), current_path)
        };
        path_guard.extend(EnvGuard::set_var("PATH", new_path));

        install_completion_for_shell(ShellType::Bash).unwrap();

        let installed = home.join(".bash_completions").join("pyx.bash");
        assert!(installed.exists());
        assert!(fs::read_to_string(installed)
            .unwrap()
            .contains("bash completion for pyx"));
    }

    #[test]
    fn test_shell_type_from_str_valid() {
        assert_eq!("bash".parse::<ShellType>().unwrap(), ShellType::Bash);
        assert_eq!("zsh".parse::<ShellType>().unwrap(), ShellType::Zsh);
        assert_eq!("fish".parse::<ShellType>().unwrap(), ShellType::Fish);
        assert_eq!(
            "powershell".parse::<ShellType>().unwrap(),
            ShellType::Powershell
        );
        assert_eq!("pwsh".parse::<ShellType>().unwrap(), ShellType::Powershell);
    }

    #[test]
    fn test_shell_type_from_str_case_insensitive() {
        assert_eq!("BASH".parse::<ShellType>().unwrap(), ShellType::Bash);
        assert_eq!("Zsh".parse::<ShellType>().unwrap(), ShellType::Zsh);
        assert_eq!("FISH".parse::<ShellType>().unwrap(), ShellType::Fish);
    }

    #[test]
    fn test_shell_type_from_str_invalid() {
        assert!("csh".parse::<ShellType>().is_err());
        assert!("tcsh".parse::<ShellType>().is_err());
        assert!("invalid".parse::<ShellType>().is_err());
        assert!("".parse::<ShellType>().is_err());
    }

    #[test]
    fn test_shell_type_display() {
        assert_eq!(ShellType::Bash.to_string(), "bash");
        assert_eq!(ShellType::Zsh.to_string(), "zsh");
        assert_eq!(ShellType::Fish.to_string(), "fish");
        assert_eq!(ShellType::Powershell.to_string(), "powershell");
    }

    #[test]
    fn test_shell_type_to_clap_complete_shell() {
        assert_eq!(
            ShellType::Bash.to_clap_complete_shell(),
            clap_complete::Shell::Bash
        );
        assert_eq!(
            ShellType::Zsh.to_clap_complete_shell(),
            clap_complete::Shell::Zsh
        );
        assert_eq!(
            ShellType::Fish.to_clap_complete_shell(),
            clap_complete::Shell::Fish
        );
        assert_eq!(
            ShellType::Powershell.to_clap_complete_shell(),
            clap_complete::Shell::PowerShell
        );
    }

    #[test]
    fn test_completion_script_install_path_bash() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let home = temp.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let _env = EnvGuard::set_var("HOME", &home);
        let path = completion_script_install_path(ShellType::Bash).unwrap();
        assert!(path.to_str().unwrap().contains("bash_completions"));
        assert!(path.to_str().unwrap().ends_with("pyx.bash"));
    }

    #[test]
    fn test_completion_script_install_path_zsh() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let home = temp.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let _env = EnvGuard::set_var("HOME", &home);
        let path = completion_script_install_path(ShellType::Zsh).unwrap();
        assert!(path.to_str().unwrap().contains(".zsh"));
        assert!(path.to_str().unwrap().contains("completions"));
        assert!(path.to_str().unwrap().ends_with("_pyx"));
    }

    #[test]
    fn test_completion_script_install_path_fish() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let home = temp.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let _env = EnvGuard::set_var("HOME", &home);
        let path = completion_script_install_path(ShellType::Fish).unwrap();
        assert!(path.to_str().unwrap().contains("fish"));
        assert!(path.to_str().unwrap().ends_with("pyx.fish"));
    }

    #[test]
    fn test_completion_script_install_path_powershell() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp = tempdir().unwrap();
        let home = temp.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let _env = EnvGuard::set_var("HOME", &home);
        let path = completion_script_install_path(ShellType::Powershell).unwrap();
        assert!(path.to_str().unwrap().contains("PowerShell"));
        assert!(path.to_str().unwrap().ends_with("pyx.ps1"));
    }

    #[test]
    fn test_shell_type_clone_and_copy() {
        let shell = ShellType::Bash;
        let cloned = shell;
        let _copied = shell;
        assert_eq!(cloned, ShellType::Bash);
    }

    #[test]
    fn test_shell_type_equality() {
        assert_eq!(ShellType::Bash, ShellType::Bash);
        assert_ne!(ShellType::Bash, ShellType::Zsh);
        assert_ne!(ShellType::Fish, ShellType::Powershell);
    }
}
