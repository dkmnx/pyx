use super::*;
use crate::test_helpers::EnvGuard;
use crate::ENV_MUTEX;
use tempfile::tempdir;

#[test]
fn test_env_passphrase_reads_non_empty() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let _g1 = EnvGuard::set_var(ENV_PASSPHRASE, "from-env");
    let passphrase = env_passphrase().unwrap();
    assert_eq!(passphrase.expose_secret(), "from-env");

    // Verify empty env var returns None
    let _g2 = EnvGuard::set_var(ENV_PASSPHRASE, "");
    assert!(env_passphrase().is_none());
}

#[test]
fn test_file_passphrase_roundtrip() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

    let passphrase = SecretString::new("test-passphrase".to_string().into_boxed_str());

    set_passphrase_file(&passphrase).unwrap();

    let retrieved = get_passphrase_file().unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().expose_secret(), "test-passphrase");

    delete_passphrase_file().unwrap();
    let retrieved = get_passphrase_file().unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_set_passphrase_writes_to_file() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1"));

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("file-test".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    let retrieved = get_passphrase().unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().expose_secret(), "file-test");

    clear_passphrase().unwrap();
    reset_backend();
}

#[test]
fn test_has_entry_checks_file() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1"));

    set_backend(Box::new(MockKeyring::new()));

    assert!(!has_entry());

    let passphrase = SecretString::new("test".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    assert!(has_entry());

    clear_passphrase().unwrap();
    assert!(!has_entry());
    reset_backend();
}

#[test]
fn test_clear_passphrase_removes_file_even_when_fallback_disabled() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1"));

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("orphan-passphrase".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    let file_path = passphrase_path().unwrap();
    assert!(
        file_path.exists(),
        "passphrase file should exist before clear"
    );

    // Disable fallback mid-test to verify cleanup still happens.
    // Cannot use EnvGuard here — the test requires the env var to be set,
    // then removed while the guard is still alive to test mid-flight behavior.
    #[allow(unsafe_code)]
    unsafe {
        std::env::remove_var("PYX_ALLOW_FILE_FALLBACK");
    }
    assert!(!file_fallback_enabled());

    clear_passphrase().unwrap();
    assert!(
        !file_path.exists(),
        "passphrase file should be removed even when fallback disabled"
    );

    reset_backend();
}

#[test]
fn test_set_passphrase_does_not_write_file_by_default() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("no-fallback".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    let file_exists = passphrase_path().map(|p| p.exists()).unwrap_or(false);
    assert!(
        !file_exists,
        "file should not exist when fallback is disabled"
    );

    clear_passphrase().unwrap();
    reset_backend();
}

#[test]
fn test_get_passphrase_uses_keyring_when_fallback_disabled() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let _env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("keyring-only".to_string().into_boxed_str());
    with_backend(|b| b.set_password(SERVICE_NAME, USER_NAME, passphrase.expose_secret())).unwrap();

    let retrieved = get_passphrase().unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().expose_secret(), "keyring-only");

    with_backend(|b| b.delete_password(SERVICE_NAME, USER_NAME)).unwrap();
    reset_backend();
}

#[test]
fn test_file_fallback_enabled_env_var() {
    let _guard = ENV_MUTEX.lock().unwrap();

    assert!(!file_fallback_enabled());

    let _g1 = EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
    assert!(file_fallback_enabled());

    let _g2 = EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "true");
    assert!(file_fallback_enabled());
    drop(_g2);

    drop(_g1);
    // After all guards drop, env var is unset — should return false
    assert!(!file_fallback_enabled());
}

#[cfg(target_os = "linux")]
#[test]
fn test_system_keyring_uses_secret_tool_when_available() {
    use std::fs;

    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    let store_dir = temp.path().join("store");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&store_dir).unwrap();

    let script_path = bin_dir.join("secret-tool");
    let script = r##"#!/usr/bin/env python3
import os
import pathlib
import sys

store_dir = pathlib.Path(os.environ["PYX_SECRET_TOOL_STORE_DIR"])
log_path = store_dir / "log.txt"
log_path.parent.mkdir(parents=True, exist_ok=True)

args = sys.argv[1:]
cmd = args[0]
attrs = args[1:]
if cmd == "store" and attrs[:2] == ["--label", "pyx passphrase"]:
    attrs = attrs[2:]
key = "__".join(attrs).replace("/", "_")
entry = store_dir / key

with log_path.open("a", encoding="utf-8") as fh:
    fh.write(cmd + "\n")

if cmd == "store":
    value = sys.stdin.read()
    entry.write_text(value, encoding="utf-8")
    sys.exit(0)
elif cmd == "lookup":
    if entry.exists():
        sys.stdout.write(entry.read_text(encoding="utf-8"))
        sys.exit(0)
    sys.exit(1)
elif cmd == "clear":
    if entry.exists():
        entry.unlink()
    sys.exit(0)
else:
    sys.exit(2)
"##;
    fs::write(&script_path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let original_path = std::env::var("PATH").unwrap_or_default();
    let mut env = EnvGuard::set_var("PATH", format!("{}:{}", bin_dir.display(), original_path));
    env.extend(EnvGuard::set_var("PYX_SECRET_TOOL_STORE_DIR", &store_dir));
    env.extend(EnvGuard::remove_var("PYX_PASSPHRASE"));
    env.extend(EnvGuard::remove_var("PYX_ALLOW_FILE_FALLBACK"));
    reset_backend();

    let passphrase = SecretString::new("secret-tool-pass".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    let log = fs::read_to_string(store_dir.join("log.txt")).unwrap();
    assert!(log.contains("store"));

    let retrieved = get_passphrase().unwrap().unwrap();
    assert_eq!(retrieved.expose_secret(), "secret-tool-pass");

    let log = fs::read_to_string(store_dir.join("log.txt")).unwrap();
    assert!(log.contains("lookup"));

    clear_passphrase().unwrap();

    let log = fs::read_to_string(store_dir.join("log.txt")).unwrap();
    assert!(log.contains("clear"));
}

#[test]
fn test_get_password_returns_none_for_missing_entry() {
    let _guard = ENV_MUTEX.lock().unwrap();

    set_backend(Box::new(MockKeyring::new()));

    let result = with_backend(|b| b.get_password("nonexistent-service", "nonexistent-user"));
    assert!(
        result.is_ok(),
        "get_password should not error on missing entry"
    );
    assert!(
        result.unwrap().is_none(),
        "get_password should return None for missing entry"
    );

    reset_backend();
}

#[test]
fn test_set_password_overwrites_existing() {
    let _guard = ENV_MUTEX.lock().unwrap();

    set_backend(Box::new(MockKeyring::new()));

    let service = "test-service";
    let user = "test-user";

    with_backend(|b| b.set_password(service, user, "first-value")).unwrap();

    let first = with_backend(|b| b.get_password(service, user))
        .unwrap()
        .unwrap();
    assert_eq!(first, "first-value");

    with_backend(|b| b.set_password(service, user, "second-value")).unwrap();

    let second = with_backend(|b| b.get_password(service, user))
        .unwrap()
        .unwrap();
    assert_eq!(second, "second-value");

    reset_backend();
}

#[test]
fn test_delete_password_is_idempotent() {
    let _guard = ENV_MUTEX.lock().unwrap();

    set_backend(Box::new(MockKeyring::new()));

    let service = "test-service";
    let user = "test-user";

    with_backend(|b| b.set_password(service, user, "value")).unwrap();

    with_backend(|b| b.delete_password(service, user)).unwrap();

    with_backend(|b| b.delete_password(service, user)).unwrap();

    let result = with_backend(|b| b.get_password(service, user)).unwrap();
    assert!(result.is_none());

    reset_backend();
}

#[test]
fn test_get_passphrase_uses_file_fallback_when_backend_errors() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1"));
    env.extend(EnvGuard::remove_var("PYX_PASSPHRASE"));

    struct ErrorBackend;
    impl crate::keys::keyring::backend::KeyringBackend for ErrorBackend {
        fn get_password(&self, _: &str, _: &str) -> crate::error::Result<Option<String>> {
            Err(crate::error::PyxError::Keyring(
                "Backend unavailable".to_string(),
            ))
        }
        fn set_password(&self, _: &str, _: &str, _: &str) -> crate::error::Result<()> {
            Err(crate::error::PyxError::Keyring(
                "Backend unavailable".to_string(),
            ))
        }
        fn delete_password(&self, _: &str, _: &str) -> crate::error::Result<()> {
            Ok(())
        }
    }

    set_backend(Box::new(ErrorBackend));

    let passphrase = SecretString::new("file-fallback-pass".to_string().into_boxed_str());
    set_passphrase_file(&passphrase).unwrap();

    let result = get_passphrase().unwrap();
    assert!(
        result.is_some(),
        "get_passphrase should fall back to file when backend errors"
    );
    assert_eq!(result.unwrap().expose_secret(), "file-fallback-pass");

    clear_passphrase().unwrap();
    reset_backend();
}

#[test]
fn test_has_entry_returns_false_when_backend_unavailable() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let _env = EnvGuard::remove_var("PYX_ALLOW_FILE_FALLBACK");

    struct EmptyBackend;
    impl crate::keys::keyring::backend::KeyringBackend for EmptyBackend {
        fn get_password(&self, _: &str, _: &str) -> crate::error::Result<Option<String>> {
            Ok(None)
        }
        fn set_password(&self, _: &str, _: &str, _: &str) -> crate::error::Result<()> {
            Ok(())
        }
        fn delete_password(&self, _: &str, _: &str) -> crate::error::Result<()> {
            Ok(())
        }
    }

    set_backend(Box::new(EmptyBackend));

    assert!(
        !has_entry(),
        "has_entry should return false when no entry exists"
    );

    reset_backend();
}

#[test]
fn test_set_passphrase_succeeds_when_backend_available() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::remove_var("PYX_ALLOW_FILE_FALLBACK"));
    env.extend(EnvGuard::remove_var("PYX_PASSPHRASE"));

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("backend-test".to_string().into_boxed_str());
    let result = set_passphrase(&passphrase);

    assert!(
        result.is_ok(),
        "set_passphrase should succeed when backend available: {:?}",
        result
    );

    let retrieved = get_passphrase().unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().expose_secret(), "backend-test");

    clear_passphrase().unwrap();
    reset_backend();
}
