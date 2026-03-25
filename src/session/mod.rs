//! Session hint management

use crate::error::{PyxError, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

static SESSION_FILENAME_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}-\d{3}Z)_([[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12})\.jsonl$",
    )
    .expect("session filename regex should be valid")
});

/// Parse session file name and return (uuid, timestamp).
///
/// Expected format:
/// `<timestamp>_<uuid>.jsonl`
///
/// Example:
/// `2026-02-18T04-01-19-316Z_f27fa890-b6f0-42ad-894f-08f0f1735fb9.jsonl`
pub fn parse_session_filename(filename: &str) -> Result<(String, String)> {
    let captures = SESSION_FILENAME_PATTERN.captures(filename).ok_or_else(|| {
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
    use tempfile::tempdir;

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

    #[test]
    fn find_most_recent_session_returns_none_for_empty_dir() {
        let temp = tempdir().unwrap();
        let result = find_most_recent_session(temp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn find_most_recent_session_returns_none_when_dir_not_exists() {
        let temp = tempdir().unwrap();
        let nonexistent = temp.path().join("nonexistent");
        let result = find_most_recent_session(&nonexistent).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn find_most_recent_session_returns_single_session() {
        let temp = tempdir().unwrap();
        let session_file = temp
            .path()
            .join("2026-03-01T10-00-00-000Z_00000000-0000-0000-0000-000000000001.jsonl");
        std::fs::write(&session_file, "").unwrap();

        let result = find_most_recent_session(temp.path()).unwrap();
        assert_eq!(
            result,
            Some("00000000-0000-0000-0000-000000000001".to_string())
        );
    }

    #[test]
    fn find_most_recent_session_returns_most_recent() {
        let temp = tempdir().unwrap();

        // Write older session
        let older = temp
            .path()
            .join("2026-02-01T10-00-00-000Z_00000000-0000-0000-0000-000000000001.jsonl");
        std::fs::write(&older, "").unwrap();

        // Write newer session
        let newer = temp
            .path()
            .join("2026-03-01T10-00-00-000Z_00000000-0000-0000-0000-000000000002.jsonl");
        std::fs::write(&newer, "").unwrap();

        let result = find_most_recent_session(temp.path()).unwrap();
        assert_eq!(
            result,
            Some("00000000-0000-0000-0000-000000000002".to_string())
        );
    }

    #[test]
    fn find_most_recent_session_skips_non_jsonl_files() {
        let temp = tempdir().unwrap();

        let jsonl = temp
            .path()
            .join("2026-03-01T10-00-00-000Z_00000000-0000-0000-0000-000000000001.jsonl");
        std::fs::write(&jsonl, "").unwrap();

        let other = temp.path().join("readme.txt");
        std::fs::write(&other, "").unwrap();

        let result = find_most_recent_session(temp.path()).unwrap();
        assert_eq!(
            result,
            Some("00000000-0000-0000-0000-000000000001".to_string())
        );
    }

    #[test]
    fn find_most_recent_session_skips_directories() {
        let temp = tempdir().unwrap();

        let session = temp
            .path()
            .join("2026-03-01T10-00-00-000Z_00000000-0000-0000-0000-000000000001.jsonl");
        std::fs::write(&session, "").unwrap();

        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let result = find_most_recent_session(temp.path()).unwrap();
        assert_eq!(
            result,
            Some("00000000-0000-0000-0000-000000000001".to_string())
        );
    }

    #[test]
    fn find_most_recent_session_skips_invalid_filenames() {
        let temp = tempdir().unwrap();

        let valid = temp
            .path()
            .join("2026-03-01T10-00-00-000Z_00000000-0000-0000-0000-000000000001.jsonl");
        std::fs::write(&valid, "").unwrap();

        let invalid = temp.path().join("invalid-filename.jsonl");
        std::fs::write(&invalid, "").unwrap();

        let result = find_most_recent_session(temp.path()).unwrap();
        assert_eq!(
            result,
            Some("00000000-0000-0000-0000-000000000001".to_string())
        );
    }
}
