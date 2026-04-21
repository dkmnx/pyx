//! Add command implementation - add a new provider credential

use crate::commands::helpers::{
    fetch_providers, format_time_now, get_provider_list, has_custom_providers,
    load_existing_master_key, load_or_create_database, store_provider_entry,
};
use crate::error::{PyxError, Result};
use crate::keys::manager::KeyManager;
use crate::prompt;
use crate::providers::validate_provider_name;
use std::io::IsTerminal;

pub fn execute(provider_name: Option<&str>) -> Result<()> {
    eprintln!("Pyx Add");
    eprintln!();

    if !KeyManager::master_key_exists() {
        return Err(PyxError::Config(
            "Pyx not initialized. Run 'pyx init' first.".to_string(),
        ));
    }

    let manager = load_existing_master_key()?;

    let mut db = load_or_create_database()?;

    let provider = match provider_name {
        Some(name) => {
            validate_provider_name(name)?;
            if db.has_provider(name) {
                return Err(PyxError::Config(format!(
                    "Provider '{name}' already configured. Use 'pyx edit' to update."
                )));
            }
            name.to_string()
        }
        None => {
            let stdin = std::io::stdin();
            if !stdin.is_terminal() {
                return Err(PyxError::Validation(
                    "Non-interactive: pass --provider and --key. Use 'pyx add --help' for details."
                        .to_string(),
                ));
            }
            fetch_providers()?;
            let providers = get_provider_list()?;
            if providers.is_empty() && !has_custom_providers()? {
                return Err(PyxError::Config(
                    "No providers available. Add a custom provider first.".to_string(),
                ));
            }
            let selected = select_provider_interactive(&providers)?;
            if db.has_provider(&selected) {
                return Err(PyxError::Config(format!(
                    "Provider '{selected}' already configured. Use 'pyx edit' to update."
                )));
            }
            selected
        }
    };

    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        return Err(PyxError::Validation(
            "Non-interactive: pass --provider. API key must be provided interactively.".to_string(),
        ));
    }
    let key = crate::commands::helpers::prompt_api_key(&provider)?;

    let is_update = store_provider_entry(&manager, &mut db, &provider, &key)?;

    eprintln!();
    let action = if is_update { "Updated" } else { "Created" };
    eprintln!("  {provider}: {action} ({})", format_time_now());

    Ok(())
}

fn select_provider_interactive(providers: &[String]) -> Result<String> {
    let provider = prompt::prompt_provider(providers).map_err(|e| {
        if matches!(e, PyxError::Cancelled) {
            eprintln!("\n\nAdd cancelled!");
            e
        } else {
            e
        }
    })?;
    Ok(provider)
}
