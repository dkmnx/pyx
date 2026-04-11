//! CLI parsing and command definitions

use clap::{Parser, Subcommand};

pub use crate::pi::exec::ShellType;

/// Returns the complete list of subcommand names derived from the Commands enum.
/// Used by routing logic in `should_use_clap` to determine when to delegate to clap.
/// This is generated dynamically so adding a new subcommand variant automatically
/// includes it — no manual sync required.
pub fn get_subcommand_names() -> &'static [&'static str] {
    use clap::CommandFactory;
    use std::sync::LazyLock;

    static NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let cmd = Cli::command();
        let mut names: Vec<String> = cmd
            .get_subcommands()
            .map(|sub| sub.get_name().to_string())
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|s| &*Box::leak(s.into_boxed_str()))
            .collect()
    });

    &NAMES
}

#[derive(Parser)]
#[command(name = "pyx")]
#[command(author, version, about = "Secure API key management for pi")]
#[command(long_about = "pyx securely manages AI provider API keys for the pi coding agent.")]
pub struct Cli {
    /// Provider name to use
    #[arg(index = 1)]
    provider: Option<String>,

    /// Session ID to use
    #[arg(short = 's', long = "session")]
    session: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
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
        /// Provider name to delete (prompts interactively if omitted)
        provider: Option<String>,

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

    /// Generate or install shell completion script
    Completion {
        /// Shell type (bash, zsh, fish, powershell)
        #[arg(value_enum)]
        shell: ShellType,

        /// Install completion script to shell config directory
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

impl Cli {
    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    pub fn parsed_command(&self) -> Option<&Commands> {
        self.command.as_ref()
    }

    /// Returns a clap Command built from this Cli definition.
    /// Use this for shell completion generation.
    pub fn clap_command() -> clap::Command {
        use clap::CommandFactory;
        Self::command()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subcommand_names_includes_all_variants() {
        let names = get_subcommand_names();
        let expected = [
            "completion",
            "delete",
            "list",
            "models",
            "pi",
            "reset",
            "setup",
            "version",
        ];
        assert_eq!(names, expected);
    }
}
