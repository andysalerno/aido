use crate::cli::Args;
use clap::Parser;
use log::info;

mod cli;
mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

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
