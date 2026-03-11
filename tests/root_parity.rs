use assert_cmd::Command;
use secrecy::SecretString;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_fake_pi_script(bin_dir: &Path, output_path: &Path) {
    let script = format!(
        "#!/usr/bin/env bash\nprintf '%s|%s\n' \"${{OPENAI_API_KEY:-}}\" \"$*\" > \"{}\"\n",
        output_path.display()
    );

    let script_path = bin_dir.join("pi");
    fs::write(&script_path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
    }
}

#[test]
fn root_forwards_pi_args_and_injects_provider_env() {
    let temp = tempdir().unwrap();
    let xdg_data = temp.path().join("xdg");
    let data_dir = xdg_data.join("ply");
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&bin_dir).unwrap();

    // Prepare cryptographic fixtures for this test.
    let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());
    let master_key = b"integration-master-key";

    let master_cipher =
        pyx_rust::crypto::age::encrypt_with_passphrase(master_key, &passphrase).unwrap();
    fs::write(data_dir.join("master.key"), master_cipher).unwrap();

    let provider_cipher =
        pyx_rust::crypto::age::encrypt_with_key(b"sk-openai-integration", master_key).unwrap();
    let db_content = format!(
        "[{{\n  \"provider\": \"openai\",\n  \"cipher\": \"{}\",\n  \"created_at\": \"2026-01-01T00:00:00Z\",\n  \"updated_at\": \"2026-01-01T00:00:00Z\"\n}}]",
        provider_cipher
    );
    fs::write(data_dir.join("database.json"), db_content).unwrap();

    let pi_output = temp.path().join("pi-output.txt");
    write_fake_pi_script(&bin_dir, &pi_output);

    let path = format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap());

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("openai")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", &xdg_data)
        .env("PLY_PASSPHRASE", "test-passphrase")
        .env("PLY_PASSPHRASE_FORCE", "1")
        .env("PATH", path);

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert_eq!(captured.trim(), "sk-openai-integration|--model gpt-4");
}

#[test]
#[ignore = "Go random-byte passphrase compatibility requires dedicated migration path"]
fn root_decrypts_go_fixture_data() {
    let temp = tempdir().unwrap();
    let xdg_data = temp.path().join("xdg");
    let data_dir = xdg_data.join("ply");
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&bin_dir).unwrap();

    fs::copy("tests/fixtures/master.key", data_dir.join("master.key")).unwrap();
    fs::copy(
        "tests/fixtures/database.json",
        data_dir.join("database.json"),
    )
    .unwrap();

    let pi_output = temp.path().join("pi-output.txt");
    write_fake_pi_script(&bin_dir, &pi_output);

    let path = format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap());

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("openai")
        .env("XDG_DATA_HOME", &xdg_data)
        .env("PLY_PASSPHRASE", "test-passphrase")
        .env("PLY_PASSPHRASE_FORCE", "1")
        .env("PATH", path);

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert!(captured.starts_with("sk-openai-test-key-12345|"));
}
