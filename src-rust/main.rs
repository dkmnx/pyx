//! Pyx Rust CLI - Secure API key management for pi

use clap::Parser;
use pyx_rust::cli::{Cli, Commands, ModelsCommands};
use pyx_rust::error::Result;
use pyx_rust::root_args::{parse_root_invocation, should_use_clap};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if should_use_clap(&raw_args) {
        return run_subcommand_mode();
    }

    let invocation = parse_root_invocation(&raw_args)?;
    let exit_code = pyx_rust::commands::root::execute(
        invocation.provider.as_deref(),
        invocation.session.as_deref(),
        &invocation.pi_args,
    )?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn run_subcommand_mode() -> Result<()> {
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
        Some(Commands::Models { action }) => match action {
            ModelsCommands::Update => {
                pyx_rust::commands::models::execute_update()?;
            }
            ModelsCommands::List { json, refresh } => {
                pyx_rust::commands::models::execute(json, refresh)?;
            }
        },
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
            let exit_code = pyx_rust::commands::root::execute(
                cli.provider.as_deref(),
                cli.session.as_deref(),
                &[],
            )?;

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
    }

    Ok(())
}
