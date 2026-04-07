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

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Setup pyx with a provider (initializes or adds provider)
    Setup,

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

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Manage AI models
    Models {
        /// Filter models by provider
        #[arg(short = 'p', long = "provider")]
        provider: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Force refresh from remote
        #[arg(long)]
        refresh: bool,

        #[command(subcommand)]
        action: Option<ModelsCommands>,
    },

    /// Manage pi installation
    Pi {
        #[command(subcommand)]
        action: Option<PiCommands>,
    },

    /// Reset pyx to initial state
    Reset {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Install shell completion script
    Completion {
        /// Shell type (bash, zsh, fish, powershell)
        shell: String,

        /// Install completion script to shell config
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
}

#[derive(Subcommand)]
pub enum PiCommands {
    /// Install pi if not already installed
    Install,
}
