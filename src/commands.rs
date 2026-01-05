mod board;
mod bug;
mod dashboard;
mod feature;
mod init;
mod milestone;
mod resolve;
mod review;
mod task;

use crate::cli::{Cli, Commands};
use crate::error::Result;

/// Execute the CLI command
pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::execute(),
        Commands::Board { json } => board::execute(json),
        Commands::Task { action } => task::execute(action),
        Commands::Review { action } => review::execute(action),
        Commands::Dashboard { json } => dashboard::execute(json),
        Commands::Resolve { id, pick } => resolve::execute(id, pick),
        Commands::Milestone { action } => milestone::execute(action),
        Commands::Feature { action } => feature::execute(action),
        Commands::Bug { action } => bug::execute(action),
    }
}
