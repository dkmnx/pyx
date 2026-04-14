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
pub mod test_helpers;

// Re-export commonly used types
pub use keys::manager::KeyManager;
pub use storage::database::{Database, ProviderEntry};
pub use storage::models_cache::ModelsCache;

/// Global mutex for tests that modify environment variables.
/// `std::env::set_var`/`std::env::remove_var` are NOT thread-safe on glibc even
/// for different keys (concurrent modifications to the process environ array can
/// corrupt reads), so all env mutations must be serialized through this lock.
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

    fn env_mutations_without_mutex(path: &Path) -> std::io::Result<Vec<usize>> {
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
                let first_lock = block
                    .iter()
                    .position(|block_line| block_line.contains("ENV_MUTEX.lock().unwrap()"));

                for (offset, block_line) in block.iter().enumerate() {
                    if !is_env_mutation(block_line) {
                        continue;
                    }

                    if first_lock.is_none_or(|lock| offset < lock) {
                        issues.push(index + offset + 1);
                    }
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
    fn env_var_mutations_in_tests_require_env_mutex() {
        let mut files = Vec::new();
        collect_rust_files(Path::new("src"), &mut files).unwrap();
        files.sort();

        let mut offenders = Vec::new();

        for file in files {
            for line in env_mutations_without_mutex(&file).unwrap() {
                offenders.push(format!("{}:{line}", file.display()));
            }
        }

        assert!(
            offenders.is_empty(),
            "All std::env::set_var/remove_var calls in #[test] functions must occur after ENV_MUTEX.lock().unwrap():\n{}",
            offenders.join("\n")
        );
    }
}
