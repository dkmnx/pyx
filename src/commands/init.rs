//! Init command implementation - initialize pyx data directory and master key

use crate::commands::helpers::create_new_master_key;
use crate::error::Result;
use crate::keys::manager::KeyManager;
use crate::storage::paths::ensure_data_dir;

pub fn execute() -> Result<()> {
    println!("Pyx Init");
    println!();

    let data_dir = ensure_data_dir()?;
    println!("Data directory: {}", data_dir.display());
    println!();

    if KeyManager::master_key_exists() {
        println!("Pyx is already initialized.");
        return Ok(());
    }

    create_new_master_key()?;

    println!("Init complete!");

    Ok(())
}
