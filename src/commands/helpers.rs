//! Shared helpers for provider credential commands.

use crate::crypto::age::encrypt_with_key;
use crate::error::{PyxError, Result};
use crate::keys::keyring;
use crate::keys::manager::KeyManager;
use crate::models::fetch::fetch_models_from_remote;
use crate::passphrase;
use crate::prompt;
use crate::storage::database::{Database, ProviderEntry};
use crate::storage::models_cache::ModelsCache;
use crate::storage::paths::database_path;
use crate::storage::providers_env::ProvidersEnvConfig;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::time::Instant;

pub fn load_or_create_database() -> Result<Database> {
    let path = database_path()?;

    if !path.exists() {
        return Ok(Database::default());
    }

    Database::load_from_path(&path).map_err(|e| {
        PyxError::Config(format!(
            "Failed to load existing database (may be corrupted): {e}. \
             Run 'pyx reset' to start fresh if this persists."
        ))
    })
}

pub fn load_or_create_master_key() -> Result<KeyManager> {
    if !KeyManager::master_key_exists() {
        return create_new_master_key();
    }

    eprintln!("Using existing master key");
    eprintln!();

    if keyring::get_passphrase()?.is_some() {
        return KeyManager::load();
    }

    prompt_for_passphrase()
}

pub fn load_existing_master_key() -> Result<KeyManager> {
    if keyring::get_passphrase()?.is_some() {
        return KeyManager::load();
    }

    prompt_for_passphrase()
}

fn prompt_for_passphrase() -> Result<KeyManager> {
    eprintln!("Passphrase not found in OS keyring.");
    eprintln!("Enter the passphrase you used during initial setup:");
    eprintln!();

    let passphrase = passphrase::prompt_existing_passphrase(Some("Passphrase"))?;

    passphrase::load_key_manager_with_passphrase(&passphrase).map_err(|e| {
        PyxError::Crypto(format!(
            "Failed to decrypt master key: {e}. \
             If you forgot your passphrase, run 'pyx reset' to start fresh."
        ))
    })
}

pub fn create_new_master_key() -> Result<KeyManager> {
    eprintln!("This will initialize pyx with secure encrypted storage.");
    eprintln!();

    let passphrase = passphrase::prompt_new_passphrase()?;

    eprintln!("Generating master key...");
    let manager = KeyManager::generate()?;

    eprintln!("Saving encrypted master key...");
    manager.save_with_passphrase(&passphrase)?;

    eprintln!("Storing passphrase in OS keyring...");
    keyring::set_passphrase(&passphrase)?;

    eprintln!();
    eprintln!("Master key initialized!");
    eprintln!();

    Ok(manager)
}

pub fn require_interactive_terminal() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        return Err(PyxError::Validation(
            "This operation requires an interactive terminal. Use --yes to skip.".into(),
        ));
    }
    Ok(())
}

pub fn fetch_providers() -> Result<()> {
    fetch_providers_with(fetch_models_from_remote)
}

pub fn fetch_providers_with<F>(fetch_remote: F) -> Result<()>
where
    F: FnOnce() -> Result<ModelsCache>,
{
    match ModelsCache::load() {
        Ok(cache) if !cache.is_stale_default() => {
            eprintln!("Using cached providers.");
            eprintln!();
            Ok(())
        }
        Ok(cache) => refresh_provider_cache(Some(cache), fetch_remote),
        Err(PyxError::Config(_)) => refresh_provider_cache(None, fetch_remote),
        Err(e) => Err(e),
    }
}

fn refresh_provider_cache<F>(cached: Option<ModelsCache>, fetch_remote: F) -> Result<()>
where
    F: FnOnce() -> Result<ModelsCache>,
{
    eprint!("Fetching providers... ");
    let start = Instant::now();

    match fetch_remote() {
        Ok(cache) => {
            eprintln!("done ({}ms)", start.elapsed().as_millis());
            cache.save()?;
            eprintln!();
            Ok(())
        }
        Err(e) => {
            if cached.is_some() {
                eprintln!("using cache (offline mode)");
                eprintln!();
                return Ok(());
            }

            if has_custom_providers()? {
                eprintln!("using custom providers");
                eprintln!();
                return Ok(());
            }

            eprintln!("failed");
            Err(PyxError::Network(format!(
                "Failed to fetch providers and no cache available: {e}"
            )))
        }
    }
}

pub fn has_custom_providers() -> Result<bool> {
    Ok(ProvidersEnvConfig::load()?
        .map(|config| !config.provider_names().is_empty())
        .unwrap_or(false))
}

pub fn get_provider_list() -> Result<Vec<String>> {
    let mut providers: HashSet<String> = HashSet::new();

    if let Ok(cache) = ModelsCache::load() {
        for provider in cache.models.keys() {
            providers.insert(provider.clone());
        }
    }

    if let Ok(Some(config)) = ProvidersEnvConfig::load() {
        for name in config.provider_names() {
            providers.insert(name.to_string());
        }
    }

    let mut list: Vec<String> = providers.into_iter().collect();
    list.sort();
    Ok(list)
}

pub fn prompt_api_key(provider: &str) -> Result<SecretString> {
    let api_key = prompt::prompt_secret(prompt::SecretPromptOptions {
        prompt: "API key".to_string(),
        helper: Some(format!("Enter API key for {provider} (input is hidden):")),
        confirmation: None,
        empty_error: "API key cannot be empty".to_string(),
    })?;

    Ok(api_key)
}

pub fn store_provider_entry(
    manager: &KeyManager,
    db: &mut Database,
    provider: &str,
    api_key: &SecretString,
) -> Result<bool> {
    let master_key = manager.get_key_bytes()?;

    let cipher = encrypt_with_key(api_key.expose_secret().as_bytes(), master_key.as_slice())
        .map_err(|e| PyxError::Crypto(format!("Failed to encrypt API key: {e}")))?;

    let is_update = db.has_provider(provider);

    let entry = ProviderEntry::new(provider.to_string(), cipher);
    db.upsert(entry);

    db.save()?;

    Ok(is_update)
}

pub fn format_time_now() -> String {
    let now = time::OffsetDateTime::now_utc();
    now.format(&TIME_FORMAT)
        .expect("formatting UTC time should not fail")
}

pub fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() < 6 {
        return "****".to_string();
    }
    let start: String = chars[..2].iter().collect();
    let end: String = chars[chars.len() - 2..].iter().collect();
    format!("{start}...{end}")
}

static TIME_FORMAT: std::sync::LazyLock<Vec<time::format_description::FormatItem<'static>>> =
    std::sync::LazyLock::new(|| {
        time::format_description::parse("[hour]:[minute]:[second] UTC").expect("valid time format")
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::EnvGuard;
    use std::cell::Cell;
    use tempfile::tempdir;
    use time::{Duration, OffsetDateTime};

    #[test]
    fn fetch_providers_skips_remote_fetch_when_cache_is_fresh() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let mut cache = ModelsCache::new("v1.0.0");
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string()]);
        cache.updated_at = OffsetDateTime::now_utc();
        cache.save().unwrap();

        let called = Cell::new(false);
        fetch_providers_with(|| {
            called.set(true);
            Ok(ModelsCache::new("v2.0.0"))
        })
        .unwrap();

        assert!(!called.get());
    }

    #[test]
    fn fetch_providers_uses_stale_cache_when_refresh_fails() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let mut cache = ModelsCache::new("v1.0.0");
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string()]);
        cache.updated_at = OffsetDateTime::now_utc() - Duration::days(2);
        cache.save().unwrap();

        let called = Cell::new(false);
        fetch_providers_with(|| {
            called.set(true);
            Err(PyxError::Network("offline".to_string()))
        })
        .unwrap();

        assert!(called.get());
    }

    #[test]
    fn fetch_providers_errors_when_cache_missing_and_refresh_fails() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let err = fetch_providers_with(|| Err(PyxError::Network("offline".to_string())))
            .expect_err("missing cache should fail");

        assert!(matches!(err, PyxError::Network(_)));
    }

    #[test]
    fn get_provider_list_includes_custom_providers_without_models_cache() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let mut config = ProvidersEnvConfig::default();
        config
            .upsert(
                "custom-provider".to_string(),
                "CUSTOM_PROVIDER_API_KEY".to_string(),
            )
            .unwrap();
        config.save().unwrap();

        let providers = get_provider_list().unwrap();
        assert_eq!(providers, vec!["custom-provider".to_string()]);
    }

    #[test]
    fn test_has_custom_providers_returns_true_when_configured() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let mut config = ProvidersEnvConfig::default();
        config
            .upsert("test-provider".to_string(), "TEST_API_KEY".to_string())
            .unwrap();
        config.save().unwrap();

        assert!(has_custom_providers().unwrap());
    }

    #[test]
    fn test_has_custom_providers_returns_false_when_empty() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let config = ProvidersEnvConfig::default();
        config.save().unwrap();

        assert!(!has_custom_providers().unwrap());
    }

    #[test]
    fn test_has_custom_providers_returns_false_when_no_config() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        assert!(!has_custom_providers().unwrap());
    }

    #[test]
    fn test_format_time_now_returns_valid_format() {
        let formatted = format_time_now();

        assert!(formatted.contains("UTC"));
        assert!(formatted.contains(":"));
        assert!(formatted.len() >= 10);

        let parts: Vec<&str> = formatted.split_whitespace().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "UTC");

        let time_parts: Vec<&str> = parts[0].split(':').collect();
        assert_eq!(time_parts.len(), 3);

        let hours: u32 = time_parts[0].parse().unwrap();
        let mins: u32 = time_parts[1].parse().unwrap();
        let secs: u32 = time_parts[2].parse().unwrap();

        assert!(hours < 24);
        assert!(mins < 60);
        assert!(secs < 60);
    }

    #[test]
    fn test_format_time_now_is_reasonable() {
        let re = regex::Regex::new(r"^(\d{2}):(\d{2}):(\d{2}) UTC$").unwrap();
        let formatted = format_time_now();

        let caps = re.captures(&formatted).unwrap();

        let hours: u32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let mins: u32 = caps.get(2).unwrap().as_str().parse().unwrap();
        let secs: u32 = caps.get(3).unwrap().as_str().parse().unwrap();

        assert!(hours < 24);
        assert!(mins < 60);
        assert!(secs < 60);
    }

    #[test]
    fn test_get_provider_list_deduplicates() {
        let temp = tempdir().unwrap();
        let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

        let mut cache = ModelsCache::new("v1.0.0");
        cache.upsert_models("openai".to_string(), vec!["gpt-4".to_string()]);
        cache.upsert_models("anthropic".to_string(), vec!["claude-3".to_string()]);
        cache.save().unwrap();

        let mut config = ProvidersEnvConfig::default();
        config
            .upsert("openai".to_string(), "DUPLICATE_API_KEY".to_string())
            .unwrap();
        config.save().unwrap();

        let providers = get_provider_list().unwrap();
        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"anthropic".to_string()));
        let openai_count = providers.iter().filter(|p| *p == "openai").count();
        assert_eq!(openai_count, 1);
    }

    #[test]
    fn test_mask_key_standard() {
        assert_eq!(mask_key("sk-1234567890abcdef"), "sk...ef");
    }

    #[test]
    fn test_mask_key_short() {
        assert_eq!(mask_key("short"), "****");
        assert_eq!(mask_key("12345"), "****");
    }

    #[test]
    fn test_mask_key_exactly_6() {
        assert_eq!(mask_key("123456"), "12...56");
    }
}
