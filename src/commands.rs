mod board;
mod dashboard;
mod init;
mod resolve;
mod review;
mod task;

use crate::cli::{Cli, Commands};
use crate::error::Result;

/// Execute the CLI command
pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::execute(),
        Commands::Board => board::execute(),
        Commands::Task { action } => task::execute(action),
        Commands::Review { action } => review::execute(action),
        Commands::Dashboard => dashboard::execute(),
        Commands::Resolve { id, pick } => resolve::execute(id, pick),
    }
}
