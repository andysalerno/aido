use crate::cli::{Args, Commands};
use clap::Parser;
use log::info;

mod cli;
mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    // Handle subcommands first
    if let Some(command) = args.command() {
        match command {
            Commands::ShowConfigPath => {
                let config_path = config::get_configuration_file_path()?;
                println!("{}", config_path);
                std::process::exit(0);
            }
        }
    }

    let config = if let Some(config_file) = args.config_file() {
        config::retrieve_from_path(config_file)?
    } else {
        config::retrieve()?
    };

    info!("Configuration loaded: {config:?}");

    if args.verbose() {
        println!("Verbose mode enabled");
    }

    Ok(())
}
