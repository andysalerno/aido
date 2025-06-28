use crate::cli::{Args, Commands};
use clap::Parser;
use log::info;

mod cli;
mod config;
mod llm;
mod recipe;
mod run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    if let Some(command) = args.command() {
        match command {
            Commands::ShowConfigPath => {
                let config_path = config::get_configuration_file_path()?;
                println!("{config_path}");

                return Ok(());
            }
            Commands::Recipe => {
                println!("...recipes...");
            }
            Commands::Run { recipe } => {
                println!("...running recipe: {recipe}");
            }
        }
    }

    let config = if let Some(config_file) = args.config_file() {
        config::retrieve_from_path(config_file)?
    } else {
        config::retrieve()?
    };

    info!("Configuration loaded: {config:?}");

    if let Some(input) = args.input() {
        info!("Input: {:?}", args.input());
        run::run(config, input)?;
    } else {
        info!("No input file provided; all done.");
    }

    Ok(())
}
