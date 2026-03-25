//! Version command implementation

use crate::error::Result;

/// Execute the version command (text output)
pub fn execute() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let git_describe = env!("GIT_DESCRIBE");
    let git_dirty = env!("GIT_DIRTY");

    if git_describe.is_empty() {
        // No git info - just show cargo version
        println!("pyx version {version}");
    } else if git_describe.starts_with('v') && !git_describe.contains('-') {
        // On an exact tag (e.g., "v0.1.0") - show just the version
        println!("pyx version {version}");
    } else {
        // Show version with git info (e.g., branch or v0.1.0-5-gabc123)
        println!("pyx version {version} ({git_hash}@{git_describe}{git_dirty})");
    }

    Ok(())
}

/// Execute the version command (JSON output)
pub fn execute_json() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let git_describe = env!("GIT_DESCRIBE");
    let git_dirty = env!("GIT_DIRTY");

    let output = serde_json::json!({
        "version": version,
        "git_hash": git_hash,
        "git_describe": git_describe,
        "git_dirty": git_dirty,
        "is_exact_tag": git_describe.starts_with('v') && !git_describe.contains('-'),
    });
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_execute() {
        // Just verify it doesn't error
        assert!(execute().is_ok());
    }

    #[test]
    fn test_version_execute_json() {
        // Just verify it doesn't error
        assert!(execute_json().is_ok());
    }
}
