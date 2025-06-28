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

    /// Print token usage after responding
    #[arg(short, long, global = true)]
    usage: bool,

    #[arg(short, long, global = true)]
    config_file: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Configuration-related commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Recipe-related commands
    Recipe {
        #[command(subcommand)]
        command: RecipeCommands,
    },
    /// Run a recipe
    Run { recipe: String },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show the configuration file path
    Show,
    /// Edit the configuration file
    Edit,
    /// Validate the configuration file
    Validate,
}

#[derive(Subcommand)]
pub enum RecipeCommands {
    /// List available recipes
    List,
    /// Show recipe details
    Show { name: String },
    /// Create a new recipe
    Create { name: String },
}

impl Args {
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn config_file(&self) -> Option<&str> {
        self.config_file.as_deref()
    }

    pub fn command(&self) -> Option<&Commands> {
        self.command.as_ref()
    }

    pub fn input(&self) -> Option<&str> {
        self.input.as_deref()
    }

    pub fn usage(&self) -> bool {
        self.usage
    }
}
