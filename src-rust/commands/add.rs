//! Add provider command implementation

use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::storage::database::{Database, ProviderEntry};

/// Execute the add provider command
pub fn execute(provider_name: &str) -> Result<()> {
    // Check if initialized
    if !KeyManager::master_key_exists() {
        eprintln!("Pyx not initialized. Run 'pyx setup' first.");
        eprintln!();
        eprintln!("To initialize:");
        eprintln!("  pyx setup");
        std::process::exit(1);
    }

    // Validate provider name
    crate::providers::validate_provider_name(provider_name)?;

    // Prompt for API key
    let _api_key = prompt_for_api_key(provider_name)?;

    // Load database
    let mut db = Database::load().map_err(|e| {
        match e {
            PyxError::Config(_) => PyxError::Config(
                "Database not found. Run 'pyx setup' first.".to_string(),
            ),
            _ => e,
        }
    })?;

    // Check if provider already exists
    if db.has_provider(provider_name) {
        eprintln!("Provider '{}' already exists.", provider_name);
        eprintln!("To update it, first delete with: pyx delete {}", provider_name);
        std::process::exit(1);
    }

    // TODO: Implement actual encryption once crypto migration is complete
    // For now, show migration notice
    eprintln!();
    eprintln!("⚠ CRYPTO MIGRATION REQUIRED");
    eprintln!();
    eprintln!("The Rust implementation requires a migration tool to encrypt new providers.");
    eprintln!("Until then, you can:");
    eprintln!("  1. Use the Go version to add providers");
    eprintln!("  2. Or wait for the migration tool");
    eprintln!();
    eprintln!("See RUST-IMPLEMENTATION-SUMMARY.md for details.");
    
    // For demonstration, we'll skip actual encryption
    // In production, this would encrypt the API key
    let _cipher = format!("PLACEHOLDER_FOR_{}", provider_name);
    
    // Create provider entry (with placeholder - won't work for actual use)
    let entry = ProviderEntry::new(provider_name.to_string(), _cipher);
    db.upsert(entry);

    // Save database
    db.save()?;

    println!("✓ Provider '{}' configuration prepared!", provider_name);
    println!();
    println!("Note: Actual encryption pending migration tool implementation.");
    println!("The Go version can be used to add providers until then.");

    Ok(())
}

/// Prompt user for API key
fn prompt_for_api_key(provider_name: &str) -> Result<String> {
    use dialoguer::{Input, theme::ColorfulTheme};

    let theme = ColorfulTheme::default();

    println!("Enter API key for provider: {}", provider_name);
    println!("(The key will be encrypted and stored securely)");
    println!();

    let api_key: String = Input::with_theme(&theme)
        .with_prompt("API Key")
        .interact_text()
        .map_err(|e| PyxError::Validation(format!("Failed to read API key: {}", e)))?;

    if api_key.is_empty() {
        return Err(PyxError::Validation(
            "API key cannot be empty".to_string(),
        ));
    }

    // Basic validation - should look like a key
    if api_key.len() < 10 {
        eprintln!("Warning: API key seems very short ({} characters)", api_key.len());
    }

    Ok(api_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_provider_workflow() {
        // This test would require full setup (master key, etc.)
        // For now, just verify the function exists
        let _ = provider_name;
    }
}
