//! Pyx Rust CLI - Secure API key management for pi

use clap::Parser;
use pyx_rust::cli::{Cli, Commands};
use pyx_rust::error::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Setup) => {
            pyx_rust::commands::setup::execute()?;
        }
        Some(Commands::List) => {
            pyx_rust::commands::list::execute()?;
        }
        Some(Commands::Delete { provider }) => {
            println!("Delete command - provider: {}", provider);
            // TODO: Implement delete command
        }
        Some(Commands::Models { action }) => {
            match action {
                Some(pyx_rust::cli::ModelsCommands::Update) => {
                    println!("Models update command");
                    // TODO: Implement models update
                }
                Some(pyx_rust::cli::ModelsCommands::List { json }) => {
                    pyx_rust::commands::models::execute(json)?;
                }
                None => {
                    println!("Models command - use 'models update' or 'models list'");
                }
            }
        }
        Some(Commands::PiInstall) => {
            println!("PI install command");
            // TODO: Implement pi install
        }
        Some(Commands::Reset) => {
            pyx_rust::commands::reset::execute()?;
        }
        Some(Commands::Completion { shell }) => {
            println!("Completion command - shell: {}", shell);
            // TODO: Implement completion generation
        }
        Some(Commands::Version) => {
            pyx_rust::commands::version::execute()?;
        }
        None => {
            // Root command execution
            if let Some(provider) = cli.provider {
                println!("Running pi with provider: {}", provider);
                // TODO: Implement root execution with provider
            } else {
                println!("Running pi with all configured providers");
                // TODO: Implement root execution with all providers
            }
            
            if let Some(session) = cli.session {
                println!("Session ID: {}", session);
                // TODO: Forward session to pi
            }
        }
    }

    Ok(())
}
