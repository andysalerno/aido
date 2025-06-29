use crate::cli::{Args, Commands, ConfigCommands, RecipeCommands};
use clap::Parser;
use log::info;

mod cli;
mod config;
mod llm;
mod recipe;
mod run;
mod tools;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    let config_file_path = if let Some(config_file) = args.config_file() {
        config_file.to_string()
    } else {
        config::get_configuration_file_path()?
    };

    let config = config::retrieve_from_path(&config_file_path)?;

    if let Some(command) = args.command() {
        match command {
            Commands::Config { command } => match command {
                ConfigCommands::Show => {
                    println!("{config:?}");
                    return Ok(());
                }
                ConfigCommands::ShowPath => {
                    println!("{config_file_path}");
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
                        // recipe dir is in the parent dir of the config file
                        let recipe_dir =
                            std::path::Path::new(&config_file_path)
                                .parent()
                                .unwrap()
                                .join("recipes");

                        // List all recipes in the directory:
                        let entries = std::fs::read_dir(recipe_dir)?;
                        for entry in entries.flatten() {
                            // Get the file extension of the entry:
                            if entry.file_type()?.is_dir() {
                                // Only print directories (recipes)
                                // If you want to include files, remove this check
                                continue;
                            }

                            if let Some(name) = entry.file_name().to_str()
                                && name.ends_with(".recipe")
                            {
                                println!("- {name}");
                            }
                        }
                    }
                    RecipeCommands::Show { name } => {
                        println!("...showing recipe: {name}...");
                    }
                    RecipeCommands::Create { name } => {
                        println!("...creating recipe: {name}...");
                    }
                    RecipeCommands::ShowDir => {
                        // recipe dir is in the parent dir of the config file
                        let recipe_dir =
                            std::path::Path::new(&config_file_path)
                                .parent()
                                .unwrap()
                                .join("recipes");
                        let recipe_dir = recipe_dir.to_string_lossy();

                        println!("{recipe_dir}");
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
