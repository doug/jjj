mod cli;
mod commands;
mod error;
mod jj;
mod models;
mod storage;
mod tui;
mod utils;

use clap::Parser;
use error::Result;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    commands::execute(cli)
}
