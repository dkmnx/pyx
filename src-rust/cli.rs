//! CLI parsing and command definitions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pyx")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Provider name to use
    #[arg(index = 1)]
    pub provider: Option<String>,

    /// Session ID to use
    #[arg(short = 's', long = "session")]
    pub session: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Setup pyx with initial configuration
    Setup,

    /// List configured providers
    List,

    /// Delete a provider configuration
    Delete {
        /// Provider name to delete
        provider: String,
    },

    /// Manage AI models
    Models {
        #[command(subcommand)]
        action: Option<ModelsCommands>,
    },

    /// Install pi if not already installed
    #[command(name = "pi install")]
    PiInstall,

    /// Reset pyx to initial state
    Reset,

    /// Generate shell completion script
    Completion {
        /// Shell type
        shell: String,
    },

    /// Print version information
    Version,
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// Update models cache
    Update,

    /// List available models
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
