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
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));

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
    env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));
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
    env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));

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
    env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("orphan-passphrase".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    let file_path = passphrase_path().unwrap();
    assert!(
        file_path.exists(),
        "passphrase file should exist before clear"
    );

    // Now disable fallback to verify cleanup still happens.
    let _disable_guard = EnvGuard::set_var("PYX_DISABLE_FILE_FALLBACK", "1");
    assert!(!file_fallback_enabled());

    clear_passphrase().unwrap();
    assert!(
        !file_path.exists(),
        "passphrase file should be removed even when fallback disabled"
    );

    reset_backend();
}

#[test]
fn test_set_passphrase_does_not_write_file_when_opt_out() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_DISABLE_FILE_FALLBACK", "1"));

    set_backend(Box::new(MockKeyring::new()));

    let passphrase = SecretString::new("no-fallback".to_string().into_boxed_str());
    set_passphrase(&passphrase).unwrap();

    let file_exists = passphrase_path().map(|p| p.exists()).unwrap_or(false);
    assert!(
        !file_exists,
        "file should not exist when fallback is opted out"
    );

    clear_passphrase().unwrap();
    reset_backend();
}

#[test]
fn test_get_passphrase_uses_keyring_when_fallback_opted_out() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp = tempdir().unwrap();
    let mut env = EnvGuard::set_var("XDG_DATA_HOME", temp.path().to_string_lossy().to_string());
    env.extend(EnvGuard::set_var("PYX_DISABLE_FILE_FALLBACK", "1"));

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

    // Default is now enabled
    assert!(file_fallback_enabled());

    // Opt-out
    let _g0 = EnvGuard::set_var("PYX_DISABLE_FILE_FALLBACK", "1");
    assert!(!file_fallback_enabled());
    drop(_g0);

    // Legacy opt-in still works
    let _g1 = EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "1");
    assert!(file_fallback_enabled());

    let _g2 = EnvGuard::set_var("PYX_ALLOW_FILE_FALLBACK", "true");
    assert!(file_fallback_enabled());
    drop(_g2);
    drop(_g1);

    // Back to default: enabled
    assert!(file_fallback_enabled());
}

#[test]
fn test_get_password_returns_none_for_missing_entry() {
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
    env.extend(EnvGuard::set_var("PYX_SCRYPT_WORK_FACTOR", "15"));
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
    let _env = EnvGuard::set_var("PYX_DISABLE_FILE_FALLBACK", "1");

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
    env.extend(EnvGuard::set_var("PYX_DISABLE_FILE_FALLBACK", "1"));
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

/// Test that NativeKeyring::is_available() returns a valid result
/// Test that the NativeKeyring backend can perform get/set/delete operations
/// or gracefully report that it's unavailable.
///
/// Skipped when `CI=true` is set in the environment because macOS and Windows
/// CI runners expose a native keyring that responds to availability probes but
/// silently fails to persist credentials across operations.
#[test]
fn test_native_keyring_roundtrip_when_available() {
    use backend::NativeKeyring;

    if std::env::var("CI").is_ok() {
        eprintln!("Skipping native keyring roundtrip test in CI environment");
        return;
    }

    // Try keyring operations directly — if the keyring daemon isn't available,
    // the operations will error and we skip the test.
    let backend = NativeKeyring;
    let service = "pyx-test-roundtrip";
    let username = "test-user";
    let password = "test-password-123";

    // Set
    let set_result = backend.set_password(service, username, password);
    if set_result.is_err() {
        eprintln!("Skipping native keyring test: no keyring backend available");
        return;
    }

    // Get
    let result = backend
        .get_password(service, username)
        .expect("get_password should succeed with available backend");
    assert_eq!(result, Some(password.to_string()));

    // Delete (idempotent)
    backend
        .delete_password(service, username)
        .expect("delete_password should succeed with available backend");

    // Verify deleted
    let result = backend
        .get_password(service, username)
        .expect("get_password should succeed with available backend");
    assert_eq!(result, None, "password should be deleted");

    // Delete again (idempotent)
    backend
        .delete_password(service, username)
        .expect("delete_password should be idempotent");
}
