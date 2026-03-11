//! Pyx Rust CLI - Secure API key management for pi

use clap::Parser;
use pyx_rust::cli::{Cli, Commands, ModelsCommands};
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
        Some(Commands::List { json }) => {
            if json {
                pyx_rust::commands::list::execute_json()?;
            } else {
                pyx_rust::commands::list::execute()?;
            }
        }
        Some(Commands::Delete { provider }) => {
            pyx_rust::commands::delete::execute(&provider)?;
        }
        Some(Commands::Models { action }) => {
            match action {
                ModelsCommands::Update => {
                    pyx_rust::commands::models::execute_update()?;
                }
                ModelsCommands::List { json, refresh } => {
                    pyx_rust::commands::models::execute(json, refresh)?;
                }
            }
        }
        Some(Commands::PiInstall) => {
            pyx_rust::pi::exec::install_pi()?;
        }
        Some(Commands::Reset) => {
            pyx_rust::commands::reset::execute()?;
        }
        Some(Commands::Completion { shell, install }) => {
            if install {
                pyx_rust::commands::completion::print_install_instructions(&shell);
            } else {
                pyx_rust::commands::completion::generate_completion(&shell)?;
            }
        }
        Some(Commands::Version { json }) => {
            if json {
                pyx_rust::commands::version::execute_json()?;
            } else {
                pyx_rust::commands::version::execute()?;
            }
        }
        None => {
            // Root command execution - run pi with configured providers
            pyx_rust::commands::root::execute(cli.provider.as_deref(), cli.session.as_deref())?;
        }
    }

    Ok(())
}
