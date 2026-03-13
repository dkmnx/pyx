//! Completion command implementation

use crate::error::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
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

/// Print installation instructions for completion scripts
pub fn print_install_instructions(shell: &str) {
    println!("To install {} completions, run:", shell);
    println!();

    match shell {
        "bash" => {
            println!("  # Add to ~/.bashrc:");
            println!("  echo 'source <(pyx completion bash)' >> ~/.bashrc");
            println!("  source ~/.bashrc");
        }
        "zsh" => {
            println!("  # Add to ~/.zshrc:");
            println!("  echo 'source <(pyx completion zsh)' >> ~/.zshrc");
            println!("  source ~/.zshrc");
        }
        "fish" => {
            println!("  # Generate completion file:");
            println!("  pyx completion fish > ~/.config/fish/completions/pyx.fish");
        }
        "powershell" => {
            println!("  # Add to PowerShell profile:");
            println!("  pyx completion powershell | Out-String | Invoke-Expression");
        }
        _ => {}
    }
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
