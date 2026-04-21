mod support;

use std::fs;
use support::{
    create_test_env, prepend_path, pyx_cmd, write_executable, write_master_key,
    write_provider_database, write_provider_database_entries, TEST_PASSPHRASE,
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

    let mut cmd = pyx_cmd();
    cmd.arg("openai")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let output = fs::read_to_string(&pi_output).unwrap();
    assert!(output.contains("sk-openai-integration"));
    assert!(output.contains("--model"));
    assert!(output.contains("gpt-4"));
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

    let mut cmd = pyx_cmd();
    cmd.arg("--session")
        .arg("session-123")
        .arg("openai")
        .arg("--")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let output = fs::read_to_string(&pi_output).unwrap();
    assert!(output.contains("sk-openai-session"));
    assert!(output.contains("--session"));
    assert!(output.contains("session-123"));
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
        "#!/usr/bin/env bash\nenv | grep -E 'OPENAI_API_KEY|ANTHROPIC_API_KEY' > \"{}\"\n",
        pi_output.display()
    );
    write_executable(&env.bin_dir.join("pi"), &script);

    let mut cmd = pyx_cmd();
    cmd.arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let output = fs::read_to_string(&pi_output).unwrap();
    assert!(output.contains("OPENAI_API_KEY=sk-openai-all"));
    assert!(output.contains("ANTHROPIC_API_KEY=sk-anthropic-all"));
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

    let mut cmd = pyx_cmd();
    cmd.arg("-c")
        .arg("openai")
        .arg("--")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let output = fs::read_to_string(&pi_output).unwrap();
    assert!(output.contains("sk-openai-continue"));
    assert!(output.contains("-c"));
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

    let mut cmd = pyx_cmd();
    cmd.arg("-r")
        .arg("openai")
        .arg("--")
        .arg("--model")
        .arg("gpt-4")
        .env("XDG_DATA_HOME", env.xdg_data_str())
        .env("PYX_PASSPHRASE", TEST_PASSPHRASE)
        .env("PATH", prepend_path(&env.bin_dir));

    cmd.assert().success();

    let output = fs::read_to_string(&pi_output).unwrap();
    assert!(output.contains("sk-openai-resume"));
    assert!(output.contains("-r"));
}
