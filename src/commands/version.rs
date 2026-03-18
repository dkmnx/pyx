//! Version command implementation

use crate::error::Result;

/// Execute the version command
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_execute() {
        // Just verify it doesn't error
        assert!(execute().is_ok());
    }
}
