use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aido")]
#[command(version = "1.0.0")]
#[command(about = "A sample AI assistant application")]
#[command(long_about = None)]
pub struct Args {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(short, long, global = true)]
    config_file: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Input file to process
    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show the configuration file path
    ShowConfigPath,
}

impl Args {
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn config_file(&self) -> Option<&str> {
        self.config_file.as_deref()
    }

    pub fn command(&self) -> &Option<Commands> {
        &self.command
    }
}
