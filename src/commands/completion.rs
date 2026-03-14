//! Completion command implementation

use crate::error::Result;
use crate::pi::exec::{install_completion_for_shell, ShellType};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

/// Generate shell completion script
pub fn generate_completion(shell: &str) -> Result<()> {
    let shell = shell.parse::<Shell>().map_err(|_| {
        crate::error::PyxError::Validation(format!(
            "Unsupported shell: {}. Supported: bash, zsh, fish, powershell",
            shell
        ))
    })?;

    let mut cmd = crate::cli::Cli::command();
    let bin_name = cmd.get_name().to_string();

    let mut stdout = io::stdout();

    generate(shell, &mut cmd, bin_name, &mut stdout);

    Ok(())
}

/// Install completion script for the specified shell
pub fn install_completion(shell: &str) -> Result<()> {
    let shell_type: ShellType = shell.parse()?;
    install_completion_for_shell(shell_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_shells() {
        let supported = ["bash", "zsh", "fish", "powershell"];
        for shell in &supported {
            assert!(shell.parse::<Shell>().is_ok());
        }
    }

    #[test]
    fn test_unsupported_shell() {
        assert!("invalid".parse::<Shell>().is_err());
    }
}
