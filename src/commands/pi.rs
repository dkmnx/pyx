//! Pi command implementation

use crate::error::Result;
use crate::pi::exec;

/// Show pi status (installed/not installed, version)
pub fn execute_status() -> Result<()> {
    exec::show_pi_status()
}

/// Execute pi install command
pub fn execute_install(auto: bool) -> Result<()> {
    if auto {
        exec::install_pi_auto()
    } else {
        exec::install_pi_with_prompt()
    }
}
