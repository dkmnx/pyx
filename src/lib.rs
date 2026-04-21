//! Pyx Rust - Secure API key management for pi
//!
//! This is a Rust rewrite of the Go-based pyx CLI tool.

pub mod cli;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod keys;
pub mod models;
pub mod passphrase;
pub mod pi;
pub mod prompt;
pub mod providers;
pub mod root_args;
pub mod session;
pub mod storage;
#[cfg(test)]
pub mod test_helpers;

// Re-export commonly used types
pub use keys::manager::KeyManager;
pub use storage::database::{Database, ProviderEntry};
pub use storage::models_cache::ModelsCache;

pub mod env_vars {
    use std::env;

    #[cfg(test)]
    thread_local! {
        static SHADOW: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }

    /// Read an environment variable.
    ///
    /// In test mode, checks a thread-local shadow map first so each test thread
    /// gets its own isolated view. Falls back to `std::env::var` for un-shadowed vars.
    ///
    /// In production mode, this is just a thin wrapper around `std::env::var`.
    pub fn var(key: &str) -> Result<String, env::VarError> {
        #[cfg(test)]
        {
            if let Some(value) = SHADOW.with(|s| s.borrow().get(key).cloned()) {
                match value {
                    Some(v) => return Ok(v),
                    None => return Err(env::VarError::NotPresent),
                }
            }
        }
        env::var(key)
    }

    /// Set a variable — test-only, updates thread-local shadow.
    #[cfg(test)]
    pub fn set_var(key: &str, value: &str) {
        SHADOW.with(|s| {
            s.borrow_mut()
                .insert(key.to_string(), Some(value.to_string()));
        });
    }

    /// Remove a variable — test-only, marks shadow as unavailable.
    #[cfg(test)]
    pub fn remove_var(key: &str) {
        SHADOW.with(|s| {
            s.borrow_mut().insert(key.to_string(), None);
        });
    }

    /// Pop shadow entry — test-only. Returns true if key was present.
    #[cfg(test)]
    pub fn pop_var(key: &str) -> bool {
        SHADOW.with(|s| s.borrow_mut().remove(key).is_some())
    }
}

/// No-op mutex for backward compatibility. Tests no longer need to acquire this —
/// thread-local storage in `env_vars` provides per-thread isolation.
/// Kept here so existing `use crate::ENV_MUTEX; let _guard = ENV_MUTEX.lock().unwrap();`
/// patterns continue to compile without modification.
#[cfg(test)]
pub static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_rust_files(&path, files)?;
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(path);
            }
        }
        Ok(())
    }

    fn is_function_declaration(line: &str) -> bool {
        let line = line.trim_start();
        line.starts_with("fn ")
            || line.starts_with("pub fn ")
            || line.starts_with("async fn ")
            || line.starts_with("pub async fn ")
    }

    fn is_env_mutation(line: &str) -> bool {
        (line.contains("std::env::set_var(")
            || line.contains("std::env::remove_var(")
            || line.contains("env::set_var(")
            || line.contains("env::remove_var("))
            && !line.contains("EnvGuard")
            && !line.contains("env_vars::")
    }

    fn find_block_end(lines: &[String], start: usize) -> usize {
        let mut depth = 0usize;
        let mut started = false;

        for (index, line) in lines.iter().enumerate().skip(start) {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    started = true;
                } else if ch == '}' && started {
                    depth = depth.saturating_sub(1);
                }
            }

            if started && depth == 0 {
                return index;
            }
        }

        lines.len().saturating_sub(1)
    }

    fn env_mutations_without_guard(path: &Path) -> std::io::Result<Vec<usize>> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();

        let mut issues = Vec::new();
        let mut index = 0usize;
        let mut pending_test = false;

        while index < lines.len() {
            let line = lines[index].trim();

            if line.contains("#[test]") {
                pending_test = true;
                index += 1;
                continue;
            }

            if pending_test && is_function_declaration(&lines[index]) {
                let end = find_block_end(&lines, index);
                let block = &lines[index..=end];

                for (offset, block_line) in block.iter().enumerate() {
                    if !is_env_mutation(block_line) {
                        continue;
                    }

                    issues.push(index + offset + 1);
                }

                pending_test = false;
                index = end + 1;
                continue;
            }

            index += 1;
        }

        Ok(issues)
    }

    #[test]
    fn env_var_mutations_in_tests_require_guard() {
        let mut files = Vec::new();
        collect_rust_files(Path::new("src"), &mut files).unwrap();
        files.sort();

        let mut offenders = Vec::new();

        for file in files {
            // test_helpers.rs and env_vars (this file) use raw env:: calls for setup/teardown — skip
            let file_str = file.to_string_lossy();
            if file_str.contains("test_helpers.rs") || file_str.contains("env_vars") {
                continue;
            }

            for line in env_mutations_without_guard(&file).unwrap() {
                offenders.push(format!("{}:{line}", file.display()));
            }
        }

        assert!(
            offenders.is_empty(),
            "All std::env::set_var/remove_var calls in #[test] functions must use EnvGuard or env_vars (not raw std::env calls):\n{}",
            offenders.join("\n")
        );
    }
}
