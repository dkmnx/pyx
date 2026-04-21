//! Root command execution - run pi with configured providers

use owo_colors::OwoColorize;

use crate::crypto::age::{decrypt_with_key, decrypt_with_passphrase};
use crate::error::{PyxError, Result};
use crate::keys::keyring::get_passphrase;
use crate::keys::manager::KeyManager;
use crate::passphrase;
use crate::pi::exec::{
    find_pi, get_pi_version, install_completion, install_pi, platform_info, spawn_pi,
};
use crate::providers::mapping::ProviderEnvResolver;
use crate::storage::database::Database;
use secrecy::{ExposeSecret, SecretString};
use std::collections::BTreeMap;

/// Arguments for the root command (run pi with providers).
pub struct RootCommandArgs<'a> {
    pub provider: Option<&'a str>,
    pub session: Option<&'a str>,
    pub continue_session: bool,
    pub resume_session: bool,
    pub pi_args: &'a [String],
}

/// Execute the root command (run pi with providers)
pub fn execute(args: RootCommandArgs) -> Result<i32> {
    if !KeyManager::master_key_exists() {
        return Err(PyxError::Config(
            "Pyx not initialized. Run 'pyx init' first.".to_string(),
        ));
    }

    let pi_was_just_installed = ensure_pi_installed()?;

    if pi_was_just_installed {
        display_installation_info();
    }

    let db = Database::load_or_error()?;

    if db.is_empty() {
        return Err(PyxError::Config(
            "No providers configured. Add providers first.".to_string(),
        ));
    }

    let providers_to_use = determine_providers(args.provider, &db)?;
    let env_vars = build_provider_env_vars(&db, &providers_to_use)?;
    let spawn_args = build_pi_args(&args);

    let exit_code = spawn_pi(&env_vars, &spawn_args)?;

    if !is_non_interactive(&stdin_is_piped(), args.pi_args)
        && !args.continue_session
        && !args.resume_session
    {
        display_session_hint();
    }

    Ok(exit_code)
}

fn ensure_pi_installed() -> Result<bool> {
    if find_pi().is_some() {
        return Ok(false);
    }

    eprintln!("pi not found in PATH. Attempting installation...");
    install_pi()?;

    if find_pi().is_none() {
        return Err(PyxError::CommandExecution(
            "pi not found in PATH after installation attempt".to_string(),
        ));
    }

    Ok(true)
}

fn display_installation_info() {
    eprintln!("Platform: {}", platform_info());
    if let Ok(version) = get_pi_version() {
        eprintln!("pi version: {version}");
    }
    eprintln!();

    if let Err(e) = install_completion() {
        eprintln!("Warning: failed to install shell completions: {e}");
    }
}

fn build_provider_env_vars(
    db: &Database,
    providers: &[String],
) -> Result<Vec<(String, SecretString)>> {
    let manager = passphrase::load_key_manager_with_fallback()?;
    let master_key = manager.get_key_bytes()?;
    let resolver = ProviderEnvResolver::new()?;

    let mut env_map: BTreeMap<String, SecretString> = BTreeMap::new();

    for provider_name in providers {
        let entry = db.get(provider_name).ok_or_else(|| {
            PyxError::ProviderNotFound(format!("Provider '{provider_name}' not found"))
        })?;

        let api_key = decrypt_api_key(&entry.cipher, master_key.as_slice())?;
        let env_var = resolver.get_env_var(provider_name)?;

        if let Some(existing) = env_map.get(&env_var) {
            if existing.expose_secret() != api_key.expose_secret() {
                return Err(PyxError::Validation(format!(
                    "Conflicting API keys for environment variable {env_var}"
                )));
            }
        } else {
            env_map.insert(env_var, api_key);
        }
    }

    Ok(env_map.into_iter().collect())
}

/// Decrypt an API key, with passphrase fallback for entries not yet migrated to v2.
fn decrypt_api_key(cipher: &str, master_key: &[u8]) -> Result<SecretString> {
    let api_key_bytes = match decrypt_with_key(cipher, master_key) {
        Ok(bytes) => bytes,
        Err(primary_error) => {
            let passphrase = get_passphrase()?.ok_or_else(|| {
                PyxError::Crypto(format!(
                    "Failed to decrypt API key with master key and no passphrase fallback: {primary_error}"
                ))
            })?;

            decrypt_with_passphrase(cipher, &passphrase).map_err(|fallback_error| {
                PyxError::Crypto(format!(
                    "Failed to decrypt API key: {primary_error} ({fallback_error})"
                ))
            })?
        }
    };

    let api_key_string = String::from_utf8(api_key_bytes)
        .map_err(|e| PyxError::Crypto(format!("Decrypted API key is not valid UTF-8: {e}")))?;

    Ok(SecretString::new(api_key_string.into_boxed_str()))
}

fn determine_providers(provider_arg: Option<&str>, db: &Database) -> Result<Vec<String>> {
    if let Some(provider_name) = provider_arg {
        // Single provider specified
        if !db.has_provider(provider_name) {
            return Err(PyxError::ProviderNotFound(format!(
                "Provider '{provider_name}' not found. Run 'pyx list' to see available providers."
            )));
        }
        Ok(vec![provider_name.to_string()])
    } else {
        // Use all configured providers
        Ok(db
            .get_provider_names()
            .into_iter()
            .map(str::to_owned)
            .collect())
    }
}

fn build_pi_args(args: &RootCommandArgs) -> Vec<String> {
    let mut result = args.pi_args.to_vec();
    if let Some(session_id) = args.session {
        result.push("--session".to_string());
        result.push(session_id.to_string());
    }
    if args.continue_session {
        result.push("--continue".to_string());
    }
    if args.resume_session {
        result.push("--resume".to_string());
    }
    result
}

/// Returns a closure that returns true when stdin is not a terminal (i.e., piped).
/// Extracted for testability — in production, uses `std::io::stdin().is_terminal()`.
fn stdin_is_piped() -> Box<dyn Fn() -> bool> {
    use std::io::IsTerminal;
    Box::new(|| !std::io::stdin().is_terminal())
}

/// Detect if pi is being invoked in non-interactive mode.
/// `is_piped` returns true when stdin is not a TTY,
/// allowing both paths to be tested independently.
fn is_non_interactive(is_piped: &dyn Fn() -> bool, pi_args: &[String]) -> bool {
    if is_piped() {
        return true;
    }

    pi_args
        .windows(2)
        .any(|w| w[0] == "-p" || w[0] == "--prompt")
        || pi_args
            .iter()
            .any(|a| a.starts_with("-p=") || a.starts_with("--prompt="))
}

fn display_session_hint() {
    eprintln!("  ██████  ██");
    eprintln!(
        "  ██  ██  ██    {}",
        "To continue this session, run:".white().dimmed()
    );
    eprintln!("  ████  ██  ██  {}", "pyx -c | pyx --continue".yellow());
    eprintln!("  ██    ██  ██\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::age::encrypt_with_key;
    use crate::storage::database::ProviderEntry;

    #[test]
    fn test_determine_providers_single() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "cipher".to_string(),
        ));
        db.upsert(ProviderEntry::new(
            "anthropic".to_string(),
            "cipher".to_string(),
        ));

        let result = determine_providers(Some("openai"), &db).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "openai");
    }

    #[test]
    fn test_determine_providers_all() {
        let mut db = Database::default();
        db.upsert(ProviderEntry::new(
            "openai".to_string(),
            "cipher".to_string(),
        ));
        db.upsert(ProviderEntry::new(
            "anthropic".to_string(),
            "cipher".to_string(),
        ));

        let result = determine_providers(None, &db).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_determine_providers_not_found() {
        let db = Database::default();
        let result = determine_providers(Some("nonexistent"), &db);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_api_key_master_key_success() {
        let key = [7u8; 32];
        let plaintext = b"sk-test-api-key";
        let cipher = encrypt_with_key(plaintext, &key).unwrap();

        let result = decrypt_api_key(&cipher, &key).unwrap();
        assert_eq!(result.expose_secret(), "sk-test-api-key");
    }

    #[test]
    fn test_decrypt_api_key_both_paths_fail() {
        // PYX_PASSPHRASE ensures get_passphrase() returns immediately without
        // hitting the OS keyring (which may be unavailable in test environments).
        // This dependency relies on get_passphrase()'s env-var-first priority.
        let _scrypt_guard = crate::test_helpers::EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "14");
        let _pass_guard =
            crate::test_helpers::EnvGuard::set_var("PYX_PASSPHRASE", "wrong-passphrase");

        let master_key = [0u8; 32];
        let wrong_key = [1u8; 32];
        let plaintext = b"secret";

        // Encrypt with wrong key so master key decryption fails
        let cipher = encrypt_with_key(plaintext, &wrong_key).unwrap();

        // Passphrase is wrong too, so fallback also fails
        let result = decrypt_api_key(&cipher, &master_key);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PyxError::Crypto(_)));
    }

    #[test]
    fn test_decrypt_api_key_rejects_invalid_utf8() {
        let key = [7u8; 32];
        let cipher = encrypt_with_key(&[0xff, 0xfe, 0xfd], &key).unwrap();

        let err = decrypt_api_key(&cipher, &key).expect_err("invalid utf-8 should fail");
        assert!(matches!(err, PyxError::Crypto(_)));
    }

    fn s(val: &str) -> String {
        val.to_string()
    }

    fn root_args<'a>(
        pi_args: &'a [String],
        session: Option<&'a str>,
        continue_session: bool,
        resume_session: bool,
    ) -> RootCommandArgs<'a> {
        RootCommandArgs {
            provider: None,
            session,
            continue_session,
            resume_session,
            pi_args,
        }
    }

    #[test]
    fn test_build_pi_args_no_session() {
        let args = build_pi_args(&root_args(&[s("--model"), s("gpt-4")], None, false, false));
        assert_eq!(args, &[s("--model"), s("gpt-4")]);
    }

    #[test]
    fn test_build_pi_args_with_session() {
        let args = build_pi_args(&root_args(
            &[s("--model"), s("gpt-4")],
            Some("abc-123"),
            false,
            false,
        ));
        assert_eq!(
            args,
            &[s("--model"), s("gpt-4"), s("--session"), s("abc-123")]
        );
    }

    #[test]
    fn test_build_pi_args_continue() {
        let args = build_pi_args(&root_args(&[s("--model"), s("gpt-4")], None, true, false));
        assert_eq!(args, &[s("--model"), s("gpt-4"), s("--continue")]);
    }

    #[test]
    fn test_build_pi_args_resume() {
        let args = build_pi_args(&root_args(&[s("--model"), s("gpt-4")], None, false, true));
        assert_eq!(args, &[s("--model"), s("gpt-4"), s("--resume")]);
    }

    #[test]
    fn test_build_pi_args_with_session_and_provider() {
        let args = build_pi_args(&root_args(
            &[s("--model"), s("gpt-4")],
            Some("abc-123"),
            false,
            false,
        ));
        assert!(args.contains(&s("--session")));
        assert!(args.contains(&s("abc-123")));
    }

    #[test]
    fn test_build_pi_args_empty_pi_args() {
        let args = build_pi_args(&root_args(&[], None, true, false));
        assert_eq!(args, &[s("--continue")]);
    }

    #[test]
    fn test_is_non_interactive_detects_prompt_flag() {
        let interactive = || false;
        assert!(is_non_interactive(&interactive, &[s("-p"), s("hello")]));
        assert!(is_non_interactive(
            &interactive,
            &[s("--prompt"), s("hello")]
        ));
        assert!(is_non_interactive(&interactive, &[s("-p=hello")]));
        assert!(is_non_interactive(&interactive, &[s("--prompt=hello")]));
        assert!(!is_non_interactive(
            &interactive,
            &[s("--model"), s("gpt-4")]
        ));
    }

    #[test]
    fn test_is_non_interactive_piped_stdin() {
        let piped = || true;
        assert!(is_non_interactive(&piped, &[s("--model"), s("gpt-4")]));
        assert!(is_non_interactive(&piped, &[]));
    }

    #[test]
    fn test_is_non_interactive_terminal_no_prompt_flags() {
        let terminal = || false;
        assert!(!is_non_interactive(&terminal, &[s("--model"), s("gpt-4")]));
        assert!(!is_non_interactive(&terminal, &[]));
    }
}
