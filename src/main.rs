use clap::Parser;
use jjj::cli;
use jjj::commands;
use jjj::error::Result;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    commands::execute(cli)
}
