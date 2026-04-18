//! Completion command implementation

use crate::cli::ShellType;
use crate::error::{PyxError, Result};
use crate::pi::exec::install_completion_for_shell;
use std::io;
use std::io::Write;

/// Generate shell completion script
pub fn generate_completion(shell: ShellType) -> Result<()> {
    let output = generate_completion_to_vec(shell)?;
    io::stdout().write_all(&output).map_err(|e| {
        PyxError::Io(std::io::Error::other(format!(
            "Failed to write completion script: {e}"
        )))
    })?;
    Ok(())
}

/// Generate shell completion script to a byte vector
pub fn generate_completion_to_vec(shell: ShellType) -> Result<Vec<u8>> {
    let mut cmd = crate::cli::Cli::clap_command();
    let bin_name = "pyx".to_string();

    let mut buffer: Vec<u8> = Vec::new();

    clap_complete::generate(
        shell.to_clap_complete_shell(),
        &mut cmd,
        bin_name,
        &mut buffer,
    );

    if buffer.is_empty() {
        return Err(PyxError::CommandExecution(
            "Generated completion script is empty".to_string(),
        ));
    }

    let content = String::from_utf8_lossy(&buffer);
    if !content.contains("pyx") {
        return Err(PyxError::CommandExecution(
            "Generated completion script does not contain expected markers".to_string(),
        ));
    }

    Ok(buffer)
}

/// Install completion script for the specified shell
pub fn install_completion(shell: ShellType) -> Result<()> {
    install_completion_for_shell(shell)
}
