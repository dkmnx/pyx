//! Root command argument parsing helpers.

use crate::cli::get_subcommand_names;
use crate::error::{PyxError, Result};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RootInvocation {
    pub provider: Option<String>,
    pub session: Option<String>,
    pub continue_session: bool,
    pub resume_session: bool,
    pub pi_args: Vec<String>,
}

pub fn should_use_clap(args: &[String]) -> bool {
    if args.is_empty() {
        return false;
    }

    let first = args[0].as_str();
    if matches!(first, "-h" | "--help" | "-V" | "--version") {
        return true;
    }

    // Check if any positional-like argument (before --) is a known subcommand.
    for arg in args.iter().take_while(|a| *a != "--") {
        if arg.starts_with('-') {
            continue;
        }

        if get_subcommand_names().iter().any(|name| name == arg) {
            return true;
        }
    }

    false
}

/// Parse root invocation arguments with Go-compatible behavior.
///
/// - Supports pyx-level `-s/--session`, `-c/--continue`, `-r/--resume` extraction.
/// - First positional (when not starting with '-') is treated as provider.
/// - Remaining args are forwarded to pi.
pub fn parse_root_invocation(args: &[String]) -> Result<RootInvocation> {
    let mut session: Option<String> = None;
    let mut continue_session = false;
    let mut resume_session = false;
    let mut filtered: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "-s" || arg == "--session" {
            let value = args.get(i + 1).ok_or_else(|| {
                PyxError::Validation("Missing value for --session/-s".to_string())
            })?;
            session = Some(value.clone());
            i += 2;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--session=") {
            if value.is_empty() {
                return Err(PyxError::Validation(
                    "Missing value for --session".to_string(),
                ));
            }
            session = Some(value.to_string());
            i += 1;
            continue;
        }

        if arg == "-c" || arg == "--continue" {
            continue_session = true;
            i += 1;
            continue;
        }

        if arg == "-r" || arg == "--resume" {
            resume_session = true;
            i += 1;
            continue;
        }

        filtered.push(arg.clone());
        i += 1;
    }

    if continue_session && session.is_some() {
        return Err(PyxError::Validation(
            "Cannot use both --continue and --session".to_string(),
        ));
    }
    if resume_session && session.is_some() {
        return Err(PyxError::Validation(
            "Cannot use both --resume and --session".to_string(),
        ));
    }
    if continue_session && resume_session {
        return Err(PyxError::Validation(
            "Cannot use both --continue and --resume".to_string(),
        ));
    }

    // Match Go parseArgs behavior for provider/pi args split.
    for (index, arg) in filtered.iter().enumerate() {
        if arg == "--" {
            let provider = if index > 0 {
                Some(filtered[0].clone())
            } else {
                None
            };
            let pi_args = filtered[(index + 1)..].to_vec();
            return Ok(RootInvocation {
                provider,
                session,
                continue_session,
                resume_session,
                pi_args,
            });
        }
    }

    if filtered.is_empty() {
        return Ok(RootInvocation {
            provider: None,
            session,
            continue_session,
            resume_session,
            pi_args: Vec::new(),
        });
    }

    if filtered[0].starts_with('-') {
        return Ok(RootInvocation {
            provider: None,
            session,
            continue_session,
            resume_session,
            pi_args: filtered,
        });
    }

    Ok(RootInvocation {
        provider: Some(filtered[0].clone()),
        session,
        continue_session,
        resume_session,
        pi_args: filtered[1..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vecs(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn parse_provider_and_pi_args() {
        let args = vecs(&["openai", "--model", "gpt-4"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert_eq!(parsed.provider.as_deref(), Some("openai"));
        assert_eq!(parsed.pi_args, vecs(&["--model", "gpt-4"]));
    }

    #[test]
    fn parse_only_pi_args() {
        let args = vecs(&["--model", "gpt-4"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert!(parsed.provider.is_none());
        assert_eq!(parsed.pi_args, args);
    }

    #[test]
    fn parse_double_dash_separator() {
        let args = vecs(&["openai", "--", "--model", "gpt-4"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert_eq!(parsed.provider.as_deref(), Some("openai"));
        assert_eq!(parsed.pi_args, vecs(&["--model", "gpt-4"]));
    }

    #[test]
    fn parse_session_flag() {
        let args = vecs(&["-s", "session-123", "openai", "--model", "gpt-4"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert_eq!(parsed.session.as_deref(), Some("session-123"));
        assert_eq!(parsed.provider.as_deref(), Some("openai"));
        assert_eq!(parsed.pi_args, vecs(&["--model", "gpt-4"]));
    }

    #[test]
    fn parse_continue_flag() {
        let args = vecs(&["-c", "openai", "--model", "gpt-4"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert!(parsed.continue_session);
        assert!(parsed.session.is_none());
        assert_eq!(parsed.provider.as_deref(), Some("openai"));
        assert_eq!(parsed.pi_args, vecs(&["--model", "gpt-4"]));
    }

    #[test]
    fn parse_continue_long_flag() {
        let args = vecs(&["--continue", "openai"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert!(parsed.continue_session);
        assert_eq!(parsed.provider.as_deref(), Some("openai"));
        assert!(parsed.pi_args.is_empty());
    }

    #[test]
    fn parse_resume_flag() {
        let args = vecs(&["-r", "openai", "--model", "gpt-4"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert!(parsed.resume_session);
        assert!(!parsed.continue_session);
        assert!(parsed.session.is_none());
        assert_eq!(parsed.provider.as_deref(), Some("openai"));
        assert_eq!(parsed.pi_args, vecs(&["--model", "gpt-4"]));
    }

    #[test]
    fn parse_resume_long_flag() {
        let args = vecs(&["--resume"]);
        let parsed = parse_root_invocation(&args).unwrap();

        assert!(parsed.resume_session);
        assert!(parsed.session.is_none());
        assert!(!parsed.continue_session);
        assert!(parsed.provider.is_none());
        assert!(parsed.pi_args.is_empty());
    }

    #[test]
    fn should_route_subcommand_to_clap() {
        assert!(should_use_clap(&vecs(&["list"])));
        assert!(should_use_clap(&vecs(&["--help"])));
        assert!(!should_use_clap(&vecs(&["openai", "--model"])));

        assert!(should_use_clap(&vecs(&["add"])));
    }

    #[test]
    fn should_route_subcommand_after_global_flags() {
        assert!(should_use_clap(&vecs(&["-s", "abc", "version"])));
        assert!(should_use_clap(&vecs(&["--session", "abc", "list"])));
        assert!(should_use_clap(&vecs(&["--session=abc", "delete"])));
        assert!(should_use_clap(&vecs(&["-c", "version"])));
        assert!(should_use_clap(&vecs(&["-r", "list"])));

        assert!(!should_use_clap(&vecs(&["-s", "abc", "--", "list"])));
    }

    #[test]
    fn parse_rejects_continue_and_session() {
        let result = parse_root_invocation(&vecs(&["-c", "-s", "abc"]));
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_resume_and_session() {
        let result = parse_root_invocation(&vecs(&["-r", "-s", "abc"]));
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_continue_and_resume() {
        let result = parse_root_invocation(&vecs(&["-c", "-r"]));
        assert!(result.is_err());
    }
}
