mod support;

use assert_cmd::Command;
use std::fs;
use support::{
    create_test_env, prepend_path, write_executable, write_master_key, write_provider_database,
    write_provider_database_entries, TEST_PASSPHRASE,
};
use tempfile::tempdir;

#[test]
fn root_forwards_pi_args_and_injects_provider_env() {
    let temp = tempdir().unwrap();
    let env = create_test_env(&temp);
    write_master_key(&env.data_dir);
    write_provider_database(&env.data_dir, "openai", "sk-openai-integration");

    let pi_output = temp.path().join("pi-output.txt");
    let script = format!(
        "#!/usr/bin/env bash\nprintf '%s|%s\n' \"${{OPENAI_API_KEY:-}}\" \"$*\" > \"{}\"\n",
        pi_output.display()
    );
    write_executable(&env.bin_dir.join("pi"), &script);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("openai")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert_eq!(captured.trim(), "sk-openai-integration|--model gpt-4");
}

#[test]
fn root_forwards_session_flag_after_double_dash_separator() {
    let temp = tempdir().unwrap();
    let env = create_test_env(&temp);
    write_master_key(&env.data_dir);
    write_provider_database(&env.data_dir, "openai", "sk-openai-session");

    let pi_output = temp.path().join("pi-session-output.txt");
    let script = format!(
        "#!/usr/bin/env bash\nprintf '%s|%s\n' \"${{OPENAI_API_KEY:-}}\" \"$*\" > \"{}\"\n",
        pi_output.display()
    );
    write_executable(&env.bin_dir.join("pi"), &script);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("-s")
        .arg("session-123")
        .arg("openai")
        .arg("--")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert_eq!(
        captured.trim(),
        "sk-openai-session|--model gpt-4 --session session-123"
    );
}

#[test]
fn root_without_provider_injects_all_configured_provider_envs() {
    let temp = tempdir().unwrap();
    let env = create_test_env(&temp);
    write_master_key(&env.data_dir);
    write_provider_database_entries(
        &env.data_dir,
        &[
            ("openai", "sk-openai-all"),
            ("anthropic", "sk-anthropic-all"),
        ],
    );

    let pi_output = temp.path().join("pi-all-output.txt");
    let script = format!(
        "#!/usr/bin/env bash\nprintf '%s|%s|%s\n' \"${{ANTHROPIC_API_KEY:-}}\" \"${{OPENAI_API_KEY:-}}\" \"$*\" > \"{}\"\n",
        pi_output.display()
    );
    write_executable(&env.bin_dir.join("pi"), &script);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert_eq!(
        captured.trim(),
        "sk-anthropic-all|sk-openai-all|--model gpt-4"
    );
}

#[test]
fn root_forwards_continue_flag() {
    let temp = tempdir().unwrap();
    let env = create_test_env(&temp);
    write_master_key(&env.data_dir);
    write_provider_database(&env.data_dir, "openai", "sk-openai-continue");

    let pi_output = temp.path().join("pi-continue-output.txt");
    let script = format!(
        "#!/usr/bin/env bash\nprintf '%s|%s\n' \"${{OPENAI_API_KEY:-}}\" \"$*\" > \"{}\"\n",
        pi_output.display()
    );
    write_executable(&env.bin_dir.join("pi"), &script);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("-c")
        .arg("openai")
        .arg("--")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert_eq!(
        captured.trim(),
        "sk-openai-continue|--model gpt-4 --continue"
    );
}

#[test]
fn root_forwards_resume_flag() {
    let temp = tempdir().unwrap();
    let env = create_test_env(&temp);
    write_master_key(&env.data_dir);
    write_provider_database(&env.data_dir, "openai", "sk-openai-resume");

    let pi_output = temp.path().join("pi-resume-output.txt");
    let script = format!(
        "#!/usr/bin/env bash\nprintf '%s|%s\n' \"${{OPENAI_API_KEY:-}}\" \"$*\" > \"{}\"\n",
        pi_output.display()
    );
    write_executable(&env.bin_dir.join("pi"), &script);

    let mut cmd = Command::cargo_bin("pyx").unwrap();
    cmd.arg("-r")
        .arg("openai")
        .arg("--")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let captured = fs::read_to_string(pi_output).unwrap();
    assert_eq!(captured.trim(), "sk-openai-resume|--model gpt-4 --resume");
}
