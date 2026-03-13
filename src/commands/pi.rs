//! Pi command implementation

use crate::error::Result;
use crate::pi::exec;

/// Execute pi install command
pub fn execute_install(auto: bool) -> Result<()> {
    if auto {
        exec::install_pi_auto()
    } else {
        exec::install_pi_with_prompt()
    }
}
