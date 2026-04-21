//! Fetch models from remote source

use crate::error::{PyxError, Result};
use crate::models::parse::parse_models;
use crate::storage::models_cache::ModelsCache;
use crate::storage::settings::Settings;
use serde::Deserialize;
use std::time::Duration;

const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
const DEFAULT_GITHUB_RAW_URL: &str = "https://raw.githubusercontent.com";
const DEFAULT_OWNER: &str = "badlogic";
const DEFAULT_REPO: &str = "pi-mono";
const DEFAULT_MODELS_PATH: &str = "packages/ai/src/models.generated.ts";
const REQUEST_TIMEOUT_SECONDS: u64 = 30;

#[derive(Debug, Clone)]
struct SourceConfig {
    api_url: String,
    raw_url: String,
    owner: String,
    repo: String,
    branch: Option<String>,
    models_path: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_GITHUB_API_URL.to_string(),
            raw_url: DEFAULT_GITHUB_RAW_URL.to_string(),
            owner: DEFAULT_OWNER.to_string(),
            repo: DEFAULT_REPO.to_string(),
            branch: None,
            models_path: DEFAULT_MODELS_PATH.to_string(),
        }
    }
}

/// Fetch models from pi-mono remote source.
pub fn fetch_models_from_remote() -> Result<ModelsCache> {
    let config = load_source_config()?;
    fetch_models_with_config(&config)
}

/// Get the latest pyx/pi models version reference (release tag or configured branch).
pub fn get_latest_version() -> Result<String> {
    let config = load_source_config()?;
    resolve_ref(&config)
}

fn fetch_models_with_config(config: &SourceConfig) -> Result<ModelsCache> {
    let version = resolve_ref(config)?;
    let content = fetch_models_file(config, &version)?;
    let parsed_models = parse_models(&content)?;

    let mut cache = ModelsCache::new(&version);
    for (provider, models) in parsed_models {
        cache.upsert_models(provider, models);
    }

    Ok(cache)
}

fn resolve_ref(config: &SourceConfig) -> Result<String> {
    if let Some(branch) = &config.branch {
        return Ok(branch.clone());
    }

    fetch_latest_release_tag(config)
}

fn fetch_latest_release_tag(config: &SourceConfig) -> Result<String> {
    let base_api = config.api_url.trim_end_matches('/');
    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        base_api, config.owner, config.repo
    );

    let response = request_get(&url)
        .map_err(|err| PyxError::Network(format!("Failed to fetch latest release tag: {err}")))?;

    let release: ReleaseResponse = serde_json::from_str(&response).map_err(|err| {
        PyxError::Network(format!("Failed to decode latest release response: {err}"))
    })?;

    if release.tag_name.trim().is_empty() {
        return Err(PyxError::Network(
            "Failed to decode latest release response: missing tag_name".to_string(),
        ));
    }

    Ok(release.tag_name)
}

fn fetch_models_file(config: &SourceConfig, git_ref: &str) -> Result<String> {
    let base_raw = config.raw_url.trim_end_matches('/');
    let models_path = config.models_path.trim_start_matches('/');

    let url = format!(
        "{}/{}/{}/{}/{}",
        base_raw, config.owner, config.repo, git_ref, models_path
    );

    request_get(&url)
        .map_err(|err| PyxError::Network(format!("Failed to fetch models file: {err}")))
}

fn request_get(url: &str) -> std::result::Result<String, String> {
    const MAX_RESPONSE_BODY_SIZE: u64 = 5 * 1024 * 1024; // 5MB
    const MAX_ERROR_BODY_SIZE: u64 = 1024; // 1KB for error responses
    const MAX_RETRIES: u32 = 3;
    const INITIAL_BACKOFF_MS: u64 = 500;
    let mut attempt = 0;
    let mut backoff_ms = INITIAL_BACKOFF_MS;

    loop {
        attempt += 1;
        let mut response = {
            let config = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(REQUEST_TIMEOUT_SECONDS)))
                .http_status_as_error(false)
                .build();
            let agent: ureq::Agent = config.into();

            match agent
                .get(url)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "pyx-cli")
                .call()
            {
                Ok(r) => r,
                Err(e) => {
                    let err_str = e.to_string();
                    if is_retryable_error(&err_str) && attempt < MAX_RETRIES {
                        eprintln!("Request failed (attempt {attempt}/{MAX_RETRIES}), retrying in {backoff_ms}ms...");
                        std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                        backoff_ms *= 2;
                        continue;
                    }
                    return Err(err_str);
                }
            }
        };

        let status = response.status().as_u16();
        if status >= 400 {
            if is_retryable_status(status) && attempt < MAX_RETRIES {
                let wait_ms = backoff_ms;
                eprintln!(
                    "HTTP {status} (attempt {attempt}/{MAX_RETRIES}), retrying in {wait_ms}ms..."
                );
                std::thread::sleep(std::time::Duration::from_millis(wait_ms));
                backoff_ms *= 2;
                continue;
            }
            let body = response
                .body_mut()
                .with_config()
                .limit(MAX_ERROR_BODY_SIZE)
                .read_to_string()
                .unwrap_or_else(|e| format!("<read error: {e}>"));
            return Err(format!("HTTP {status}: {}", body.trim()));
        }

        return response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BODY_SIZE)
            .read_to_string()
            .map_err(|err| format!("failed to read response body: {err}"));
    }
}

fn is_retryable_error(err: &str) -> bool {
    err.contains("timeout")
        || err.contains("connection")
        || err.contains("broken pipe")
        || err.contains("connection refused")
        || err.contains("connection reset")
}

fn is_retryable_status(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

fn load_source_config() -> Result<SourceConfig> {
    let settings = Settings::load()?;
    let mut config = SourceConfig::default();

    if let Some(github_source) = settings.github_source {
        config.owner = github_source.owner;
        config.repo = github_source.repo;
        config.branch = github_source
            .branch
            .and_then(|branch| normalize_optional_string(&branch));
    }

    apply_env_override("PYX_GITHUB_API_URL", &mut config.api_url);
    apply_env_override("PYX_GITHUB_RAW_URL", &mut config.raw_url);
    apply_env_override("PYX_PI_MONO_OWNER", &mut config.owner);
    apply_env_override("PYX_PI_MONO_REPO", &mut config.repo);
    apply_env_override("PYX_MODELS_FILE_PATH", &mut config.models_path);

    if config.api_url.trim().is_empty()
        || config.raw_url.trim().is_empty()
        || config.owner.trim().is_empty()
        || config.repo.trim().is_empty()
        || config.models_path.trim().is_empty()
    {
        return Err(PyxError::Config(
            "Invalid GitHub source configuration for models fetch".to_string(),
        ));
    }

    Ok(config)
}

fn apply_env_override(env_key: &str, target: &mut String) {
    if let Ok(value) = crate::env_vars::var(env_key) {
        if let Some(normalized) = normalize_optional_string(&value) {
            *target = normalized;
        }
    }
}

fn normalize_optional_string(value: &str) -> Option<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn default_test_config(server_url: &str) -> SourceConfig {
        SourceConfig {
            api_url: server_url.to_string(),
            raw_url: server_url.to_string(),
            owner: "badlogic".to_string(),
            repo: "pi-mono".to_string(),
            branch: None,
            models_path: "packages/ai/src/models.generated.ts".to_string(),
        }
    }

    #[test]
    fn fetch_models_uses_latest_release_and_models_file() {
        let mut server = Server::new();
        let config = default_test_config(&server.url());

        let release_mock = server
            .mock("GET", "/repos/badlogic/pi-mono/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v9.9.9"}"#)
            .expect(1)
            .create();

        let models_mock = server
            .mock(
                "GET",
                "/badlogic/pi-mono/v9.9.9/packages/ai/src/models.generated.ts",
            )
            .with_status(200)
            .with_body(
                r#"
model "one" {
  provider: "openai"
  id: "openai/gpt-4.1"
}
model "two" {
  provider: "anthropic"
  id: "anthropic/claude-3-7-sonnet"
}
"#,
            )
            .expect(1)
            .create();

        let cache = fetch_models_with_config(&config).expect("fetch should succeed");

        release_mock.assert();
        models_mock.assert();
        assert_eq!(cache.version, "v9.9.9");
        assert!(cache.models.contains_key("openai"));
        assert!(cache.models.contains_key("anthropic"));
    }

    #[test]
    fn fetch_models_uses_branch_when_configured() {
        let mut server = Server::new();
        let mut config = default_test_config(&server.url());
        config.branch = Some("main".to_string());

        let models_mock = server
            .mock(
                "GET",
                "/badlogic/pi-mono/main/packages/ai/src/models.generated.ts",
            )
            .with_status(200)
            .with_body(
                r#"
model "one" {
  provider: "openai"
  id: "openai/gpt-4.1"
}
"#,
            )
            .expect(1)
            .create();

        let cache = fetch_models_with_config(&config).expect("branch fetch should succeed");
        models_mock.assert();
        assert_eq!(cache.version, "main");
    }

    #[test]
    fn fetch_models_returns_error_when_release_lookup_fails() {
        let mut server = Server::new();
        let config = default_test_config(&server.url());

        let _release_mock = server
            .mock("GET", "/repos/badlogic/pi-mono/releases/latest")
            .with_status(500)
            .expect(1)
            .create();

        let err = fetch_models_with_config(&config).expect_err("release lookup should fail");
        assert!(matches!(err, PyxError::Network(_)));
    }

    #[test]
    fn fetch_models_returns_error_when_models_file_fails() {
        let mut server = Server::new();
        let config = default_test_config(&server.url());

        let _release_mock = server
            .mock("GET", "/repos/badlogic/pi-mono/releases/latest")
            .with_status(200)
            .with_body(r#"{"tag_name":"v1.2.3"}"#)
            .expect(1)
            .create();

        let _models_mock = server
            .mock(
                "GET",
                "/badlogic/pi-mono/v1.2.3/packages/ai/src/models.generated.ts",
            )
            .with_status(404)
            .expect(1)
            .create();

        let err = fetch_models_with_config(&config).expect_err("models fetch should fail");
        assert!(matches!(err, PyxError::Network(_)));
    }

    #[test]
    fn fetch_models_returns_error_for_unparseable_content() {
        let mut server = Server::new();
        let config = default_test_config(&server.url());

        let _release_mock = server
            .mock("GET", "/repos/badlogic/pi-mono/releases/latest")
            .with_status(200)
            .with_body(r#"{"tag_name":"v1.2.3"}"#)
            .expect(1)
            .create();

        let _models_mock = server
            .mock(
                "GET",
                "/badlogic/pi-mono/v1.2.3/packages/ai/src/models.generated.ts",
            )
            .with_status(200)
            .with_body("definitely not parseable model content")
            .expect(1)
            .create();

        let err = fetch_models_with_config(&config).expect_err("parser should fail");
        assert!(matches!(err, PyxError::Validation(_)));
    }
}
