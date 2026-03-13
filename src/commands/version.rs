//! Version command implementation

use crate::error::Result;

/// Execute the version command
pub fn execute() -> Result<()> {
    println!("pyx version {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

/// Execute version command with JSON output
pub fn execute_json() -> Result<()> {
    let version_info = serde_json::json!({
        "name": "pyx",
        "version": env!("CARGO_PKG_VERSION"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
    });
    println!("{}", version_info);
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
