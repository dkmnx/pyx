//! Setup command implementation - deprecated, use init + add instead

use crate::commands::helpers::{
    fetch_providers, format_time_now, get_provider_list, load_or_create_database,
    load_or_create_master_key, store_provider_entry,
};
use crate::error::{PyxError, Result};
use crate::prompt;
use crate::providers::validate_provider_name;

pub fn execute() -> Result<()> {
    eprintln!("Warning: 'pyx setup' is deprecated. Use 'pyx init' followed by 'pyx add' instead.");
    println!();
    println!("Pyx Setup");
    println!();

    let data_dir = crate::storage::paths::ensure_data_dir()?;
    println!("Data directory: {}", data_dir.display());
    println!();

    let manager = load_or_create_master_key()?;

    let mut db = load_or_create_database()?;

    fetch_providers()?;

    let providers = get_provider_list()?;

    let provider = prompt_provider_selection(&providers, &db)?;

    let api_key = crate::commands::helpers::prompt_api_key(&provider)?;

    let is_update = store_provider_entry(&manager, &mut db, &provider, &api_key)?;

    let action = if is_update { "Updated" } else { "Created" };
    println!();
    println!("  {}: {} ({})", provider, action, format_time_now());
    println!();
    println!("Setup complete!");

    Ok(())
}

fn prompt_provider_selection(
    providers: &[String],
    db: &crate::storage::database::Database,
) -> Result<String> {
    let provider = match prompt::prompt_provider(providers) {
        Ok(p) => p,
        Err(PyxError::Cancelled) => {
            println!("\n\nSetup cancelled!");
            return Err(PyxError::Cancelled);
        }
        Err(e) => return Err(e),
    };

    validate_provider_name(&provider)?;

    if db.has_provider(&provider) {
        let confirm = prompt::prompt_confirm(&format!(
            "Provider '{provider}' already configured. Override?"
        ))?;
        if !confirm {
            println!("\nProvider already configured!\n");
            return Err(PyxError::Cancelled);
        }
    }

    Ok(provider)
}
