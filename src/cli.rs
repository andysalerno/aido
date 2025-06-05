use clap::Parser;

#[derive(Parser)]
#[command(name = "aido")]
#[command(version = "1.0.0")]
#[command(about = "A sample AI assistant application")]
#[command(long_about = None)]
pub struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    config_file: Option<String>,

    /// Input file to process
    #[arg(short, long)]
    input: Option<String>,
}

impl Args {
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn config_file(&self) -> Option<&str> {
        self.config_file.as_deref()
    }
}
