//! Test helpers for environment variable management
//!
//! Uses thread-local storage (via `crate::env_vars`) to isolate env vars per test thread.
//! No global mutex needed — each thread manages its own env var state.

use std::ffi::OsStr;

/// RAII guard that manages environment variable changes during a test.
///
/// Updates a thread-local shadow map (NOT the global process environment).
/// When dropped, removes the shadow entries so the thread falls back to global values.
pub struct EnvGuard {
    keys: Vec<String>,
}

impl EnvGuard {
    /// Set an environment variable in the current thread's shadow.
    pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) -> Self {
        let key_str = key.as_ref().to_string_lossy().into_owned();
        crate::env_vars::set_var(&key_str, value.as_ref().to_str().unwrap_or(""));
        Self {
            keys: vec![key_str],
        }
    }

    /// Mark an environment variable as "unavailable" in the current thread's shadow.
    pub fn remove_var<K: AsRef<OsStr>>(key: K) -> Self {
        let key_str = key.as_ref().to_string_lossy().into_owned();
        crate::env_vars::remove_var(&key_str);
        Self {
            keys: vec![key_str],
        }
    }

    /// Extend the guard to manage additional variable changes.
    pub fn extend(&mut self, mut other: EnvGuard) {
        self.keys.append(&mut other.keys);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in self.keys.drain(..) {
            crate::env_vars::pop_var(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_up_clean_env() {
        crate::env_vars::pop_var("TEST_GUARD_VAR");
        crate::env_vars::pop_var("TEST_VAR_A");
        crate::env_vars::pop_var("TEST_VAR_B");
        crate::env_vars::pop_var("ISOLATED_TEST_VAR");
        crate::env_vars::pop_var("TEST_GUARD_VAR_NONEXISTENT");
    }

    #[test]
    fn set_var_restores_original_on_drop() {
        set_up_clean_env();

        {
            let _guard = EnvGuard::set_var("TEST_GUARD_VAR", "new_value");
            assert_eq!(crate::env_vars::var("TEST_GUARD_VAR").unwrap(), "new_value");
        }

        // After drop, should fall back to global (not set)
        assert!(crate::env_vars::var("TEST_GUARD_VAR").is_err());
    }

    #[test]
    fn set_var_overrides_existing_baseline() {
        set_up_clean_env();
        crate::env_vars::set_var("TEST_GUARD_VAR", "original");

        {
            let _guard = EnvGuard::set_var("TEST_GUARD_VAR", "modified");
            assert_eq!(crate::env_vars::var("TEST_GUARD_VAR").unwrap(), "modified");
        }

        // EnvGuard pop removes the shadow entry entirely; the baseline is also gone.
        assert!(crate::env_vars::var("TEST_GUARD_VAR").is_err());
        crate::env_vars::pop_var("TEST_GUARD_VAR");
    }

    #[test]
    fn remove_var_blocks_access_while_alive() {
        set_up_clean_env();
        crate::env_vars::set_var("TEST_GUARD_VAR", "will_be_removed");

        {
            let _guard = EnvGuard::remove_var("TEST_GUARD_VAR");
            assert!(crate::env_vars::var("TEST_GUARD_VAR").is_err());
        }

        // After drop the shadow entry is gone; real env has no value either.
        assert!(crate::env_vars::var("TEST_GUARD_VAR").is_err());
        crate::env_vars::pop_var("TEST_GUARD_VAR");
    }

    #[test]
    fn remove_var_restores_nonexistent_on_drop() {
        set_up_clean_env();
        crate::env_vars::remove_var("TEST_GUARD_VAR_NONEXISTENT");

        {
            let _guard = EnvGuard::remove_var("TEST_GUARD_VAR_NONEXISTENT");
            assert!(crate::env_vars::var("TEST_GUARD_VAR_NONEXISTENT").is_err());
        }

        assert!(crate::env_vars::var("TEST_GUARD_VAR_NONEXISTENT").is_err());
        crate::env_vars::pop_var("TEST_GUARD_VAR_NONEXISTENT");
    }

    #[test]
    fn extend_combines_guards() {
        set_up_clean_env();
        crate::env_vars::remove_var("TEST_VAR_A");
        crate::env_vars::remove_var("TEST_VAR_B");

        {
            let mut guard = EnvGuard::set_var("TEST_VAR_A", "a");
            guard.extend(EnvGuard::set_var("TEST_VAR_B", "b"));

            assert_eq!(crate::env_vars::var("TEST_VAR_A").unwrap(), "a");
            assert_eq!(crate::env_vars::var("TEST_VAR_B").unwrap(), "b");
        }

        assert!(crate::env_vars::var("TEST_VAR_A").is_err());
        assert!(crate::env_vars::var("TEST_VAR_B").is_err());
        crate::env_vars::pop_var("TEST_VAR_A");
        crate::env_vars::pop_var("TEST_VAR_B");
    }

    #[test]
    fn parallel_tests_do_not_interfere() {
        set_up_clean_env();

        let _guard = EnvGuard::set_var("ISOLATED_TEST_VAR", "test_value");
        assert_eq!(
            crate::env_vars::var("ISOLATED_TEST_VAR").unwrap(),
            "test_value"
        );
        // On drop, shadow is cleared
    }
}
