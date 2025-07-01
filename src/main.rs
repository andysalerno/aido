use std::vec;

use crate::{
    cli::{Args, Commands, ConfigCommands, RecipeCommands},
    llm::Message,
};
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
                        recipe::list(&config_file_path)?;
                    }
                    RecipeCommands::Show { name } => {
                        println!("...showing recipe: {name}...");
                        let recipe_dir =
                            recipe::get_recipes_dir(&config_file_path);
                        let recipe = recipe::get_content(&recipe_dir, name)?;

                        println!("{recipe}");
                    }
                    RecipeCommands::Create { name } => {
                        println!("...creating recipe: {name}...");
                    }
                    RecipeCommands::ShowDir => {
                        // recipe dir is in the parent dir of the config file
                        let recipe_dir =
                            recipe::get_recipes_dir(&config_file_path);
                        let recipe_dir = recipe_dir.to_string_lossy();

                        println!("{recipe_dir}");
                    }
                }
                return Ok(());
            }
            Commands::Run { recipe, user_message } => {
                let recipes_dir = recipe::get_recipes_dir(&config_file_path);
                run::run_recipe(
                    config,
                    &recipes_dir,
                    recipe,
                    user_message.to_owned(),
                    args.usage(),
                )?;

                return Ok(());
            }
        }
    }

    info!("Configuration loaded: {config:?}");

    if let Some(input) = args.input() {
        info!("Input: {:?}", args.input());
        let messages = vec![Message::User(input.to_string())];
        run::run(config, messages, args.usage())?;
    } else {
        info!("No input file provided; all done.");
    }

    Ok(())
}
