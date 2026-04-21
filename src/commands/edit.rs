//! Edit command implementation - edit an existing provider credential

use crate::commands::helpers::{
    format_time_now, load_existing_master_key, load_or_create_database, mask_key,
    store_provider_entry,
};
use crate::crypto::age::decrypt_with_key;
use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::prompt;
use std::io::IsTerminal;

pub fn execute(provider_name: Option<&str>, skip_confirm: bool) -> Result<()> {
    eprintln!("Pyx Edit");
    eprintln!();

    if !KeyManager::master_key_exists() {
        return Err(PyxError::Config(
            "Pyx not initialized. Run 'pyx init' first.".to_string(),
        ));
    }

    let manager = load_existing_master_key()?;

    let mut db = load_or_create_database()?;

    if db.is_empty() {
        return Err(PyxError::Config(
            "No providers configured. Use 'pyx add' to add one.".to_string(),
        ));
    }

    let provider = match provider_name {
        Some(name) => {
            if !db.has_provider(name) {
                return Err(PyxError::ProviderNotFound(format!(
                    "Provider '{name}' not found. Use 'pyx add' to add."
                )));
            }
            name.to_string()
        }
        None => {
            let stdin = std::io::stdin();
            if !stdin.is_terminal() {
                return Err(PyxError::Validation(
                    "Non-interactive: pass --provider and --key. Use 'pyx edit --help' for details."
                        .to_string(),
                ));
            }
            select_provider_interactive(&db)?
        }
    };

    let masked = {
        let current_entry = db.get(&provider).ok_or_else(|| {
            PyxError::ProviderNotFound(format!("Provider '{provider}' not found."))
        })?;
        let master_key_bytes = manager.get_key_bytes()?;
        let plaintext = decrypt_with_key(&current_entry.cipher, master_key_bytes.as_slice())
            .map_err(|e| PyxError::Crypto(format!("Failed to decrypt current key: {e}")))?;
        let plain_str = String::from_utf8(plaintext)
            .map_err(|e| PyxError::Crypto(format!("Current key is not valid UTF-8: {e}")))?;
        mask_key(&plain_str)
    };
    eprintln!("Current key: {masked}");
    eprintln!();

    let stdin = std::io::stdin();
    if stdin.is_terminal() && !skip_confirm {
        crate::commands::helpers::require_interactive_terminal()?;
        let confirm = prompt::prompt_confirm("Overwrite with new key?")?;
        if !confirm {
            eprintln!("\nEdit cancelled.");
            return Ok(());
        }
    }

    if !stdin.is_terminal() {
        return Err(PyxError::Validation(
            "Non-interactive: API key must be provided interactively.".to_string(),
        ));
    }
    let key = crate::commands::helpers::prompt_api_key(&provider)?;

    store_provider_entry(&manager, &mut db, &provider, &key)?;

    eprintln!();
    eprintln!("  {provider}: Updated ({})", format_time_now());

    Ok(())
}

fn select_provider_interactive(db: &crate::storage::database::Database) -> Result<String> {
    let providers: Vec<String> = db
        .get_provider_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    prompt::prompt_provider(&providers).map_err(|e| {
        if matches!(e, PyxError::Cancelled) {
            eprintln!("\n\nEdit cancelled.");
            e
        } else {
            e
        }
    })
}
