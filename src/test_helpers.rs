//! Test helpers for environment variable management
//!
//! Provides RAII guards that automatically restore environment variables on drop.

use std::env;
use std::ffi::OsStr;

/// RAII guard that manages environment variable changes during a test.
///
/// When dropped, restores all modified environment variables to their original state.
/// This eliminates the need for manual cleanup and prevents env var leaks between tests.
pub struct EnvGuard {
    vars: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    /// Set an environment variable, restoring it to its original value (or removing it) on drop.
    pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) -> Self {
        let key_str = key.as_ref().to_string_lossy().into_owned();
        let original = env::var(&key_str).ok();
        env::set_var(&key_str, value);
        Self {
            vars: vec![(key_str, original)],
        }
    }

    /// Remove an environment variable, restoring it to its original value on drop.
    pub fn remove_var<K: AsRef<OsStr>>(key: K) -> Self {
        let key_str = key.as_ref().to_string_lossy().into_owned();
        let original = env::var(&key_str).ok();
        env::remove_var(&key_str);
        Self {
            vars: vec![(key_str, original)],
        }
    }

    /// Extend the guard to manage additional variable changes.
    pub fn extend(&mut self, mut other: EnvGuard) {
        self.vars.append(&mut other.vars);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, original) in self.vars.drain(..) {
            match original {
                Some(value) => env::set_var(&key, &value),
                None => env::remove_var(&key),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_var_restores_original_on_drop() {
        // Start clean
        env::remove_var("TEST_GUARD_VAR");

        {
            let _guard = EnvGuard::set_var("TEST_GUARD_VAR", "new_value");
            assert_eq!(env::var("TEST_GUARD_VAR").unwrap(), "new_value");
        }

        // Should be removed (was not set before)
        assert!(env::var("TEST_GUARD_VAR").is_err());
    }

    #[test]
    fn set_var_restores_existing_value_on_drop() {
        env::set_var("TEST_GUARD_VAR", "original");

        {
            let _guard = EnvGuard::set_var("TEST_GUARD_VAR", "modified");
            assert_eq!(env::var("TEST_GUARD_VAR").unwrap(), "modified");
        }

        assert_eq!(env::var("TEST_GUARD_VAR").unwrap(), "original");

        env::remove_var("TEST_GUARD_VAR");
    }

    #[test]
    fn remove_var_restores_original_on_drop() {
        env::set_var("TEST_GUARD_VAR", "will_be_removed");

        {
            let _guard = EnvGuard::remove_var("TEST_GUARD_VAR");
            assert!(env::var("TEST_GUARD_VAR").is_err());
        }

        assert_eq!(env::var("TEST_GUARD_VAR").unwrap(), "will_be_removed");

        env::remove_var("TEST_GUARD_VAR");
    }

    #[test]
    fn remove_var_restores_nonexistent_on_drop() {
        env::remove_var("TEST_GUARD_VAR_NONEXISTENT");

        {
            let _guard = EnvGuard::remove_var("TEST_GUARD_VAR_NONEXISTENT");
            assert!(env::var("TEST_GUARD_VAR_NONEXISTENT").is_err());
        }

        // Should still not exist
        assert!(env::var("TEST_GUARD_VAR_NONEXISTENT").is_err());
    }

    #[test]
    fn extend_combines_guards() {
        env::remove_var("TEST_VAR_A");
        env::remove_var("TEST_VAR_B");

        {
            let mut guard = EnvGuard::set_var("TEST_VAR_A", "a");
            guard.extend(EnvGuard::set_var("TEST_VAR_B", "b"));

            assert_eq!(env::var("TEST_VAR_A").unwrap(), "a");
            assert_eq!(env::var("TEST_VAR_B").unwrap(), "b");
        }

        assert!(env::var("TEST_VAR_A").is_err());
        assert!(env::var("TEST_VAR_B").is_err());
    }
}
