use crate::{
    cli::{Args, Commands},
    config::Config,
    llm::LlmRequest,
};
use clap::Parser;
use log::info;

mod cli;
mod config;
mod llm;
mod recipe;

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
        run(config, input)?;
    } else {
        info!("No input file provided; all done.");
    }

    Ok(())
}

fn run(config: Config, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let llm = llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    let response = llm.get_chat_completion(&LlmRequest { text: input.into() })?;

    info!("LLM Response: {}", response.text);

    Ok(())
}
