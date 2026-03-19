//! Session hint management

use crate::error::{PyxError, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Parse session file name and return (uuid, timestamp).
///
/// Expected format:
/// `<timestamp>_<uuid>.jsonl`
///
/// Example:
/// `2026-02-18T04-01-19-316Z_f27fa890-b6f0-42ad-894f-08f0f1735fb9.jsonl`
pub fn parse_session_filename(filename: &str) -> Result<(String, String)> {
    let pattern = Regex::new(
        r"^(\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}-\d{3}Z)_([[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12})\.jsonl$",
    )
    .map_err(|e| PyxError::Validation(format!("Invalid session filename regex: {e}")))?;

    let captures = pattern.captures(filename).ok_or_else(|| {
        PyxError::Validation(format!("Invalid session filename format: {filename}"))
    })?;

    let timestamp = captures
        .get(1)
        .ok_or_else(|| PyxError::Validation("Missing timestamp in session filename".to_string()))?
        .as_str()
        .to_string();

    let uuid = captures
        .get(2)
        .ok_or_else(|| PyxError::Validation("Missing UUID in session filename".to_string()))?
        .as_str()
        .to_string();

    Ok((uuid, timestamp))
}

/// Find most recent session UUID from sessions directory.
pub fn find_most_recent_session(sessions_dir: &Path) -> Result<Option<String>> {
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let mut best: Option<(String, String)> = None;

    for entry in std::fs::read_dir(sessions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            continue;
        }

        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        if !filename.ends_with(".jsonl") {
            continue;
        }

        let Ok((uuid, timestamp)) = parse_session_filename(&filename) else {
            continue;
        };

        match &best {
            Some((best_ts, _)) if timestamp <= *best_ts => {}
            _ => best = Some((timestamp, uuid)),
        }
    }

    Ok(best.map(|(_, uuid)| uuid))
}

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
    fn parse_session_filename_valid() {
        let (uuid, ts) = parse_session_filename(
            "2026-02-18T04-01-19-316Z_f27fa890-b6f0-42ad-894f-08f0f1735fb9.jsonl",
        )
        .unwrap();

        assert_eq!(uuid, "f27fa890-b6f0-42ad-894f-08f0f1735fb9");
        assert_eq!(ts, "2026-02-18T04-01-19-316Z");
    }

    #[test]
    fn parse_session_filename_invalid() {
        assert!(parse_session_filename("invalid.jsonl").is_err());
    }

    #[test]
    fn encode_cwd_rejects_invalid() {
        assert!(encode_cwd("../etc/passwd").is_err());
        assert!(encode_cwd("").is_err());
    }
}
