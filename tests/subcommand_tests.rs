mod support;

use assert_cmd::Command;
use std::fs;
use support::{
    create_test_env, prepend_path, write_executable, write_master_key, write_provider_database,
    TestEnv, TEST_PASSPHRASE,
};
use tempfile::{tempdir, TempDir};

fn setup_pyx_env(temp: &TempDir) -> TestEnv {
    let env = create_test_env(temp);
    write_master_key(&env.data_dir);
    write_provider_database(&env.data_dir, "openai", "sk-openai-test123");
    env
}

fn setup_empty_pyx_env(temp: &TempDir) -> TestEnv {
    let env = create_test_env(temp);
    write_master_key(&env.data_dir);
    fs::write(env.data_dir.join("database.json"), "[]").unwrap();
    env
}

#[test]
fn list_subcommand_shows_configured_providers() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("openai"));
}

#[test]
fn list_subcommand_json_output() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .arg("--json")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("\"providers\":"));
}

#[test]
fn version_subcommand_shows_version() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("version")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);

    cmd.assert().success();
}

#[test]
fn list_subcommand_empty_database() {
    let temp = tempdir().unwrap();
    let env = setup_empty_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);

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
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("openai"));

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("delete")
        .arg("openai")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Removed provider: openai"));

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("list")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("No providers configured"));
}

#[test]
fn delete_subcommand_not_found() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("delete")
        .arg("nonexistent")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn models_subcommand_shows_cached_models() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let cache_content = r#"{
  "version": "v1.0.0",
  "updated_at": "2026-01-01T00:00:00Z",
  "models": {
    "openai": ["gpt-4", "gpt-3.5-turbo"],
    "anthropic": ["claude-3-opus", "claude-3-sonnet"]
  }
}"#;
    fs::write(env.data_dir.join("models.json"), cache_content).unwrap();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("models")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("openai"))
        .stdout(predicates::str::contains("gpt-4"));
}

#[test]
fn models_subcommand_json_output() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let cache_content = r#"{
  "version": "v1.0.0",
  "updated_at": "2026-01-01T00:00:00Z",
  "models": {
    "openai": ["gpt-4"]
  }
}"#;
    fs::write(env.data_dir.join("models.json"), cache_content).unwrap();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("models")
        .arg("--json")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
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
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("reset")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE);
    cmd.assert().failure();
}

#[test]
fn pi_subcommand_shows_status_when_not_installed() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("pi")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", "/nonexistent");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("pi is not installed"));
}

#[test]
fn pi_install_subcommand_surfaces_package_manager_failures() {
    let temp = tempdir().unwrap();
    let env = setup_pyx_env(&temp);

    let npm = env.bin_dir.join("npm");
    write_executable(&npm, "#!/bin/sh\nexit 12\n");

    let path = prepend_path(&env.bin_dir);
    assert!(path.starts_with(&env.bin_dir.display().to_string()));

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("pi")
        .arg("install")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", env.bin_dir.display().to_string());

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("Failed to install pi"));
}

#[test]
fn root_command_errors_when_not_initialized() {
    let temp = tempdir().unwrap();
    let xdg_data_str = temp.path().to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("openai")
        .env("XDG_DATA_HOME", &xdg_data_str)
        .env("PATH", "/nonexistent");

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("Pyx not initialized"));
}
