use crate::cli::{Args, Commands, ConfigCommands, RecipeCommands};
use clap::Parser;
use log::info;

mod cli;
mod config;
mod llm;
mod recipe;
mod run;
mod tools;
mod tools2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    let config = if let Some(config_file) = args.config_file() {
        config::retrieve_from_path(config_file)?
    } else {
        config::retrieve()?
    };

    if let Some(command) = args.command() {
        match command {
            Commands::Config { command } => match command {
                ConfigCommands::Show => {
                    println!("{config:?}");
                    return Ok(());
                }
                ConfigCommands::ShowPath => {
                    let config_path = config::get_configuration_file_path()?;
                    println!("{config_path}");
                    return Ok(());
                }
                ConfigCommands::Edit => {
                    println!("...editing config...");
                    return Ok(());
                }
                ConfigCommands::Validate => {
                    println!("...validating config...");
                    return Ok(());
                }
            },
            Commands::Recipe { command } => {
                match command {
                    RecipeCommands::List => {
                        println!("...listing recipes...");
                    }
                    RecipeCommands::Show { name } => {
                        println!("...showing recipe: {name}...");
                    }
                    RecipeCommands::Create { name } => {
                        println!("...creating recipe: {name}...");
                    }
                }
                return Ok(());
            }
            Commands::Run { recipe } => {
                println!("...running recipe: {recipe}");
                return Ok(());
            }
        }
    }

    info!("Configuration loaded: {config:?}");

    if let Some(input) = args.input() {
        info!("Input: {:?}", args.input());
        run::run(config, input, args.usage())?;
    } else {
        info!("No input file provided; all done.");
    }

    Ok(())
}
