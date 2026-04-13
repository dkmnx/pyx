//! Session directory path helpers.
//!
//! These utilities encode the current working directory into a pi-compatible
//! session directory name and resolve the sessions root. All session file
//! scanning and UUID parsing has been removed — use pi's built-in `--continue`,
//! `--resume`, and `--session` flags instead.

use crate::error::{PyxError, Result};
use std::path::PathBuf;

/// Resolve pi sessions root directory (`~/.pi/agent/sessions`).
pub fn pi_sessions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| PyxError::Config("Could not determine home directory".to_string()))?;
    Ok(home.join(".pi").join("agent").join("sessions"))
}

/// Encode cwd to pi session directory format.
pub fn encode_cwd(cwd: &str) -> Result<String> {
    if cwd.is_empty() || cwd.contains("..") || cwd.contains('\0') {
        return Err(PyxError::Validation(
            "Invalid working directory".to_string(),
        ));
    }

    let normalized = cwd.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');
    let encoded = trimmed.replace(['/', ':'], "-");
    Ok(format!("--{encoded}--"))
}

/// Resolve session directory for a working directory.
pub fn dir_for_cwd(cwd: &str) -> Result<PathBuf> {
    Ok(pi_sessions_dir()?.join(encode_cwd(cwd)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_cwd_rejects_invalid() {
        assert!(encode_cwd("../etc/passwd").is_err());
        assert!(encode_cwd("").is_err());
    }

    #[test]
    fn encode_cwd_normalizes_path() {
        assert_eq!(
            encode_cwd("/home/user/projects/myapp").unwrap(),
            "--home-user-projects-myapp--"
        );
    }

    #[test]
    fn encode_cwd_handles_backslashes() {
        assert_eq!(
            encode_cwd("C:\\Users\\dev\\project").unwrap(),
            "--C--Users-dev-project--"
        );
    }
}
