use assert_cmd::Command;
use secrecy::SecretString;
use std::fs;
use tempfile::tempdir;

/// Helper to set up a minimal pyx environment with master key and database
fn setup_pox_env(temp: &tempfile::TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    // XDG_DATA_HOME should be the parent of the "pyx" directory
    let xdg_data = temp.path();
    let data_dir = xdg_data.join("pyx");
    fs::create_dir_all(&data_dir).unwrap();

    // Create master.key
    let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
    let master_key = b"integration-master-key";
    let master_cipher =
        pyx_rs::crypto::age::encrypt_with_passphrase(master_key, &passphrase).unwrap();
    fs::write(data_dir.join("master.key"), master_cipher).unwrap();

    // Create database.json with a provider entry
    let provider_cipher =
        pyx_rs::crypto::age::encrypt_with_key(b"sk-openai-test123", master_key).unwrap();
    let db_content = format!(
        r#"[
  {{
    "provider": "openai",
    "cipher": "{}",
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z"
  }}
]"#,
        provider_cipher
    );
    fs::write(data_dir.join("database.json"), db_content).unwrap();

    (data_dir, xdg_data.to_path_buf())
}

#[test]
fn list_subcommand_shows_configured_providers() {
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("openai"));
}

#[test]
fn list_subcommand_json_output() {
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .arg("--json")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("\"providers\":"));
}

#[test]
fn version_subcommand_shows_version() {
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("version")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");

    cmd.assert().success();
}

#[test]
fn list_subcommand_empty_database() {
    let temp = tempdir().unwrap();
    let xdg_data = temp.path();
    let data_dir = xdg_data.join("pyx");
    fs::create_dir_all(&data_dir).unwrap();

    // Create master.key but empty database
    let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
    let master_key = b"integration-master-key";
    let master_cipher =
        pyx_rs::crypto::age::encrypt_with_passphrase(master_key, &passphrase).unwrap();
    fs::write(data_dir.join("master.key"), master_cipher).unwrap();

    // Empty database
    fs::write(data_dir.join("database.json"), "[]").unwrap();

    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("No providers configured"));
}

#[test]
fn help_flag_works() {
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

#[test]
fn delete_subcommand_removes_provider() {
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    // First verify provider exists
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("openai"));

    // Delete the provider
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("delete")
        .arg("openai")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Removed provider: openai"));

    // Verify provider is gone
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("No providers configured"));
}

#[test]
fn delete_subcommand_not_found() {
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("delete")
        .arg("nonexistent")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn models_subcommand_shows_cached_models() {
    let temp = tempdir().unwrap();
    let (data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    // Create a models cache
    let cache_content = r#"{
  "version": "v1.0.0",
  "updated_at": "2026-01-01T00:00:00Z",
  "models": {
    "openai": ["gpt-4", "gpt-3.5-turbo"],
    "anthropic": ["claude-3-opus", "claude-3-sonnet"]
  }
}"#;
    fs::write(data_dir.join("models.json"), cache_content).unwrap();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("models")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("openai"))
        .stdout(predicates::str::contains("gpt-4"));
}

#[test]
fn models_subcommand_json_output() {
    let temp = tempdir().unwrap();
    let (data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    // Create a models cache
    let cache_content = r#"{
  "version": "v1.0.0",
  "updated_at": "2026-01-01T00:00:00Z",
  "models": {
    "openai": ["gpt-4"]
  }
}"#;
    fs::write(data_dir.join("models.json"), cache_content).unwrap();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("models")
        .arg("--json")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("\"version\""))
        .stdout(predicates::str::contains("\"models\""));
}

#[test]
fn completion_subcommand_generates_bash() {
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("completion").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("bash"))
        .stdout(predicates::str::contains("pyx"));
}

#[test]
fn completion_subcommand_generates_zsh() {
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("completion").arg("zsh");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("zsh"))
        .stdout(predicates::str::contains("pyx"));
}

#[test]
fn completion_subcommand_generates_fish() {
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("completion").arg("fish");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("fish"))
        .stdout(predicates::str::contains("pyx"));
}

#[test]
fn reset_subcommand_requires_confirmation() {
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    // Reset without confirmation should fail or prompt
    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("reset")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase");
    // Reset requires interactive confirmation, so it will fail in test env
    cmd.assert().failure();
}

#[test]
fn pi_subcommand_shows_status_when_not_installed() {
    // This test verifies that pi command shows status when pi is not installed
    let temp = tempdir().unwrap();
    let (_data_dir, xdg_data) = setup_pox_env(&temp);
    let xdg_data_str = xdg_data.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("pi")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PYX_PASSPHRASE", "test-passphrase")
        // Remove pi from PATH to simulate not installed
        .env("PATH", "/nonexistent");
    // Should succeed and show status
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("pi is not installed"));
}
