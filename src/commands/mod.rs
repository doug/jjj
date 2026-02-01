pub mod board;
pub mod completion;
pub mod critique;
pub mod fetch;
pub mod init;
pub mod milestone;
pub mod problem;
pub mod resolve;
pub mod solution;
pub mod status;
pub mod workflow;

use crate::cli::{Cli, Commands};
use crate::error::Result;

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::execute(),
        Commands::Board { json } => board::execute(json),
        Commands::Problem { action } => problem::execute(action),
        Commands::Solution { action } => solution::execute(action),
        Commands::Critique { action } => critique::execute(action),
        Commands::Resolve { id, pick } => resolve::execute(id, pick),
        Commands::Milestone { action } => milestone::execute(action),

        // Workflow Commands
        Commands::Submit { force } => workflow::submit(force),

        // Shell completion
        Commands::Completion { shell } => completion::execute(shell),

        // Status (replaces dashboard + next)
        Commands::Status { all, mine, limit, json } => status::execute(all, mine, limit, json),

        // Sync commands
        Commands::Fetch { remote } => fetch::execute(&remote),
    }
}
