//! CLI parsing and command definitions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pyx")]
#[command(author, version, about = "Secure API key management for pi")]
#[command(long_about = "pyx securely manages AI provider API keys for the pi coding agent.")]
pub struct Cli {
    /// Provider name to use
    #[arg(index = 1)]
    pub provider: Option<String>,

    /// Session ID to use
    #[arg(short = 's', long = "session")]
    pub session: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "text", value_parser = ["text", "json"])]
    pub format: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Setup pyx with initial configuration
    Setup,

    /// Add a new provider with API key
    Add {
        /// Provider name (optional - will prompt if not provided)
        provider: Option<String>,
    },

    /// List configured providers
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Delete a provider configuration
    Delete {
        /// Provider name to delete
        provider: String,
    },

    /// Manage AI models
    Models {
        #[command(subcommand)]
        action: ModelsCommands,
    },

    /// Install pi if not already installed
    #[command(name = "pi-install")]
    PiInstall,

    /// Reset pyx to initial state
    Reset,

    /// Generate shell completion script
    Completion {
        /// Shell type (bash, zsh, fish, powershell)
        shell: String,

        /// Show installation instructions
        #[arg(long)]
        install: bool,
    },

    /// Print version information
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// Update models cache from remote
    Update,

    /// List available models
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Force refresh from remote
        #[arg(long)]
        refresh: bool,
    },
}
