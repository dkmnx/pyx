//! Version command implementation

use crate::error::Result;

pub fn execute() -> Result<()> {
    println!("pyx version {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
