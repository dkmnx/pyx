use secrecy::SecretString;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub const TEST_PASSPHRASE: &str = "test-passphrase";
const TEST_MASTER_KEY: &[u8; 32] =
    b"integration-master-key\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

pub struct TestEnv {
    pub xdg_data: PathBuf,
    pub data_dir: PathBuf,
    pub bin_dir: PathBuf,
}

impl TestEnv {
    pub fn xdg_data_str(&self) -> String {
        self.xdg_data.to_string_lossy().to_string()
    }
}

pub fn create_test_env(temp: &TempDir) -> TestEnv {
    let xdg_data = temp.path().join("xdg");
    let data_dir = xdg_data.join("pyx");
    let bin_dir = temp.path().join("bin");

    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&bin_dir).unwrap();

    TestEnv {
        xdg_data,
        data_dir,
        bin_dir,
    }
}

pub fn write_master_key(data_dir: &Path) {
    std::env::set_var("PYX_SCRYPT_WORK_FACTOR", "15");
    let passphrase = SecretString::new(TEST_PASSPHRASE.to_string().into_boxed_str());
    let master_cipher =
        pyx_rs::crypto::age::encrypt_with_passphrase(TEST_MASTER_KEY, &passphrase).unwrap();
    fs::write(data_dir.join("master.key"), master_cipher).unwrap();
}

/// Create a `Command` for the pyx binary with test env vars pre-configured.
pub fn pyx_cmd() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("pyx").unwrap();
    cmd.env("PYX_SCRYPT_WORK_FACTOR", "15");
    cmd
}

pub fn write_provider_database(data_dir: &Path, provider: &str, api_key: &str) {
    write_provider_database_entries(data_dir, &[(provider, api_key)]);
}

pub fn write_provider_database_entries(data_dir: &Path, entries: &[(&str, &str)]) {
    let providers = entries
        .iter()
        .map(|(provider, api_key)| {
            let provider_cipher =
                pyx_rs::crypto::age::encrypt_with_key(api_key.as_bytes(), TEST_MASTER_KEY).unwrap();
            format!(
                r#"  {{
    "provider": "{}",
    "cipher": "{}",
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z"
  }}"#,
                provider, provider_cipher
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let db_content = format!("[\n{}\n]", providers);
    fs::write(data_dir.join("database.json"), db_content).unwrap();
}

#[allow(dead_code)]
pub fn write_models_cache(data_dir: &Path) {
    let content = r#"{
  "version": "v0.1.0-test",
  "updated_at": "2099-01-01T00:00:00Z",
  "models": {
    "openai": ["gpt-4", "gpt-4o", "gpt-3.5-turbo"],
    "anthropic": ["claude-3", "claude-instant"],
    "mistral": ["mistral-7b"],
    "groq": ["llama3-70b"],
    "deepseek": ["deepseek-chat"],
    "gemini": ["gemini-pro"]
  },
  "cache_format_version": "1"
}"#;
    fs::write(data_dir.join("models.json"), content).unwrap();
}

pub fn write_executable(path: &Path, script: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    fs::write(path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

pub fn prepend_path(bin_dir: &Path) -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    if current.is_empty() {
        bin_dir.display().to_string()
    } else {
        format!("{}:{}", bin_dir.display(), current)
    }
}
