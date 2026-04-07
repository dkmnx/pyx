//! Completion command implementation

use crate::cli::ShellType;
use crate::error::Result;
use crate::pi::exec::install_completion_for_shell;
use clap::CommandFactory;
use std::io;

/// Generate shell completion script
pub fn generate_completion(shell: ShellType) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();
    let bin_name = cmd.get_name().to_string();

    let mut stdout = io::stdout();

    clap_complete::generate(
        shell.to_clap_complete_shell(),
        &mut cmd,
        bin_name,
        &mut stdout,
    );

    Ok(())
}

/// Install completion script for the specified shell
pub fn install_completion(shell: ShellType) -> Result<()> {
    install_completion_for_shell(shell)
}
