use clap::Parser;
use jjj::cli;
use jjj::commands;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = commands::execute(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
