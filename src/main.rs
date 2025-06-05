use clap::Parser;

#[derive(Parser)]
#[command(name = "aido")]
#[command(version = "1.0.0")]
#[command(about = "A sample AI assistant application")]
#[command(long_about = None)]
struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Input file to process
    #[arg(short, long)]
    input: Option<String>,
}

fn main() {
    let args = Args::parse();

    if args.verbose {
        println!("Verbose mode enabled");
    }

    match args.input {
        Some(file) => println!("Processing input file: {}", file),
        None => println!("Hello, world! Use --help for more options."),
    }
}
