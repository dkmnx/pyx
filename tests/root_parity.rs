mod support;

use assert_cmd::Command;
use std::fs;
use support::{
    create_test_env, prepend_path, write_executable, write_master_key, write_provider_database,
    TEST_PASSPHRASE,
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
