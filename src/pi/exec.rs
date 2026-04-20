//! Execute pi process

use crate::error::{PyxError, Result};
use crate::storage::paths::pi_path_cache;
use clap::ValueEnum;
use secrecy::{ExposeSecret, SecretString};
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

/// Store the resolved pi path in the cache
fn store_pi_path(pi_path: &str) -> Result<()> {
    let cache_path = pi_path_cache()?;
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::storage::atomic_write::atomic_write_with_backup(&cache_path, pi_path.as_bytes(), 0o600)?;
    Ok(())
}

/// Read the cached pi path.
fn read_cached_path(cache_path: &std::path::Path) -> Option<String> {
    let content = fs::read_to_string(cache_path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Verify the resolved pi path against the cached path.
///
/// Pi auto-updates frequently (npm/bun), so we only verify the **path** stays
/// the same, not the binary content. If the path changes we silently update
/// the cache rather than blocking the user - a path change without a version
/// change is rare and harmless, and a version change doesn't warrant blocking.
fn verify_pi_path(pi_path: &str) -> Result<()> {
    let cache_path = pi_path_cache()?;
    if !cache_path.exists() {
        store_pi_path(pi_path)?;
        return Ok(());
    }

    let stored = match read_cached_path(&cache_path) {
        Some(p) => p,
        None => {
            // Corrupt or empty cache - rewrite it.
            store_pi_path(pi_path)?;
            return Ok(());
        }
    };

    if stored == pi_path {
        return Ok(());
    }

    // Resolve symlinks - /usr/local/bin/pi and ~/.bun/bin/pi might point at
    // the same binary after an update.
    let current = std::fs::canonicalize(pi_path).unwrap_or_else(|_| PathBuf::from(pi_path));
    let stored_canonical =
        std::fs::canonicalize(&stored).unwrap_or_else(|_| PathBuf::from(&stored));
    if stored_canonical == current {
        // Same binary, different path (e.g. symlink changed) - update cache.
        store_pi_path(pi_path)?;
        return Ok(());
    }

    // Path genuinely changed - pi may have been upgraded, reinstalled, or
    // moved. Auto-update the cache instead of blocking the user.
    eprintln!("Note: pi path changed ({stored} -> {pi_path}), updating cache.");
    store_pi_path(pi_path)?;
    Ok(())
}

/// Check if pi is available in PATH
pub fn find_pi() -> Option<String> {
    which::which("pi")
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
}

/// Spawn pi process with environment variables
pub fn spawn_pi(env_vars: &[(String, SecretString)], args: &[String]) -> Result<i32> {
    let pi_path = find_pi().ok_or_else(|| {
        PyxError::CommandExecution(
            "pi not found in PATH. Run 'pyx pi install' to install.".to_string(),
        )
    })?;

    verify_pi_path(&pi_path)?;

    let mut cmd = Command::new(&pi_path);
    cmd.args(args);

    // Don't clear the environment — pi needs the full terminal environment
    // (COLORTERM, TERM_PROGRAM, TERMINFO, etc.) for its TUI to function.
    // We only inject the provider API keys on top of the inherited env.
    for (key, value) in env_vars {
        cmd.env(key, value.expose_secret());
    }

    // Do NOT set process_group(0) — the TUI needs to be in the foreground
    // process group to receive terminal input and control the terminal.

    let mut child = cmd
        .spawn()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to execute pi: {e}")))?;

    let status = child
        .wait()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to wait for pi: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Ok(status
            .code()
            .or_else(|| status.signal().map(|sig| 128 + sig))
            .unwrap_or(1))
    }
    #[cfg(not(unix))]
    {
        Ok(status.code().unwrap_or(1))
    }
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

    if let Some(pi_path) = find_pi() {
        store_pi_path(&pi_path)?;
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

/// Extract the first version-like string (digits.digits[.digits...] ) from `input`.
///
/// Matches the same pattern as `\d+\.\d+(?:\.\d+)*` without requiring the
/// regex crate or a static compiled regex.
fn extract_version_string(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut start = None;
    let mut end = 0;
    let mut i = 0;

    // Scan for a sequence of digits followed by a dot and more digits
    while i < len {
        if bytes[i].is_ascii_digit() {
            // Found a digit — collect the full numeric run
            let num_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }

            // Must be followed by '.' and another digit sequence
            if i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                // This is the start of a version string
                start = Some(num_start);
                // Walk remaining "(.digits)*" groups
                while i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                    i += 1; // skip '.'
                    while i < len && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                end = i;
                break;
            }
            // Not a version start — continue scanning (i already advanced past digits)
        } else {
            i += 1;
        }
    }

    start.map(|s| input[s..end].to_string())
}

/// Check pi version
pub fn get_pi_version() -> Result<String> {
    let pi_path =
        find_pi().ok_or_else(|| PyxError::CommandExecution("pi not found in PATH".to_string()))?;

    let output = Command::new(&pi_path)
        .arg("--version")
        .output()
        .map_err(|e| PyxError::CommandExecution(format!("Failed to get pi version: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(PyxError::CommandExecution(format!(
            "pi --version exited with {}: stderr={:?} stdout={:?}",
            output
                .status
                .code()
                .map_or("unknown signal".to_string(), |c| c.to_string()),
            stderr.trim(),
            stdout.trim()
        )));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let version = extract_version_string(&stderr)
        .or_else(|| extract_version_string(&stdout))
        .ok_or_else(|| {
            PyxError::CommandExecution(format!(
                "Failed to parse pi version from output (stderr: {stderr:?}, stdout: {stdout:?})"
            ))
        })?;

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
    let output = crate::commands::completion::generate_completion_to_vec(shell)?;

    let script_path = completion_script_install_path(shell)?;

    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&script_path, &output)?;

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

        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();

        let _path_guard = EnvGuard::set_var("HOME", &home);

        install_completion_for_shell(ShellType::Bash).unwrap();

        let installed = home.join(".bash_completions").join("pyx.bash");
        assert!(installed.exists());
        let content = fs::read_to_string(&installed).unwrap();
        assert!(
            content.contains("pyx"),
            "completion script must contain 'pyx': {content}"
        );
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

    #[test]
    fn test_extract_version_string() {
        // Basic version
        assert_eq!(extract_version_string("pi 1.2"), Some("1.2".to_string()));
        // Semver
        assert_eq!(
            extract_version_string("pi 1.2.3"),
            Some("1.2.3".to_string())
        );
        // Four-part version
        assert_eq!(
            extract_version_string("1.2.3.4"),
            Some("1.2.3.4".to_string())
        );
        // Version embedded in text
        assert_eq!(
            extract_version_string("pi version 2.18.1 (build abc123)"),
            Some("2.18.1".to_string())
        );
        // No version present
        assert_eq!(extract_version_string("no version here"), None);
        // Just digits (no dot) — should NOT match
        assert_eq!(extract_version_string("42"), None);
        // Digit-dot but no second digit group
        assert_eq!(extract_version_string("1."), None);
        // Multiple version-like strings — returns first
        assert_eq!(
            extract_version_string("v1.0 and v2.3.4"),
            Some("1.0".to_string())
        );
        // Version at start
        assert_eq!(
            extract_version_string("3.14 is pi"),
            Some("3.14".to_string())
        );
    }
}
