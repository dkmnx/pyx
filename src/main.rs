//! Pyx Rust CLI - Secure API key management for pi

use clap::Parser;
use pyx_rs::cli::{Cli, Commands, ModelsCommands, PiCommands};
use pyx_rs::error::{PyxError, Result};
use pyx_rs::root_args::{parse_root_invocation, should_use_clap};

fn main() {
    if let Err(e) = run() {
        match e {
            PyxError::Cancelled => std::process::exit(0),
            _ => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn run() -> Result<()> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if should_use_clap(&raw_args) {
        return run_subcommand_mode();
    }

    let invocation = parse_root_invocation(&raw_args)?;
    let exit_code = pyx_rs::commands::root::execute(pyx_rs::commands::root::RootCommandArgs {
        provider: invocation.provider.as_deref(),
        session: invocation.session.as_deref(),
        continue_session: invocation.continue_session,
        resume_session: invocation.resume_session,
        pi_args: &invocation.pi_args,
    })?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn run_subcommand_mode() -> Result<()> {
    let cli = Cli::parse();

    match cli.parsed_command() {
        Some(Commands::Init) => {
            pyx_rs::commands::init::execute()?;
        }
        Some(Commands::Add { provider }) => {
            pyx_rs::commands::add::execute(provider.as_deref())?;
        }
        Some(Commands::Edit { provider, yes }) => {
            pyx_rs::commands::edit::execute(provider.as_deref(), *yes)?;
        }
        Some(Commands::List { json }) => {
            if *json {
                pyx_rs::commands::list::execute_json()?;
            } else {
                pyx_rs::commands::list::execute()?;
            }
        }
        Some(Commands::Delete { provider, yes }) => {
            pyx_rs::commands::delete::execute(provider.as_deref(), *yes)?;
        }
        Some(Commands::Models {
            provider,
            json,
            refresh,
            action,
        }) => match action {
            Some(ModelsCommands::Update) => {
                pyx_rs::commands::models::execute_update()?;
            }
            None => {
                pyx_rs::commands::models::execute(pyx_rs::commands::models::ModelsCommandArgs {
                    json: *json,
                    refresh: *refresh,
                    provider: provider.as_deref(),
                })?;
            }
        },
        Some(Commands::Pi { action }) => match action {
            Some(PiCommands::Install { force }) => {
                pyx_rs::commands::pi::execute_install(*force)?;
            }
            None => {
                pyx_rs::commands::pi::execute_status()?;
            }
        },
        Some(Commands::Reset { yes }) => {
            pyx_rs::commands::reset::execute(*yes)?;
        }
        Some(Commands::Completion { shell, install }) => {
            if *install {
                pyx_rs::commands::completion::install_completion(*shell)?;
            } else {
                pyx_rs::commands::completion::generate_completion(*shell)?;
            }
        }
        Some(Commands::Version { json }) => {
            if *json {
                pyx_rs::commands::version::execute_json()?;
            } else {
                pyx_rs::commands::version::execute()?;
            }
        }
        None => {
            let exit_code =
                pyx_rs::commands::root::execute(pyx_rs::commands::root::RootCommandArgs {
                    provider: cli.provider(),
                    session: cli.session(),
                    continue_session: cli.continue_session(),
                    resume_session: cli.resume_session(),
                    pi_args: &[],
                })?;

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
    }

    Ok(())
}
