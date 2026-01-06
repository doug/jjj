pub mod board;
pub mod bug;
pub mod completion;
pub mod dashboard;
pub mod feature;
pub mod init;
pub mod milestone;
pub mod resolve;
pub mod review;
pub mod tag;
pub mod task;
pub mod workflow;

use crate::cli::{Cli, Commands};
use crate::error::Result;

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
        Commands::Tag { action } => tag::execute(action),

        // New Workflow Commands
        Commands::Start { arg, feature } => workflow::start(arg, feature),
        Commands::Submit { force } => workflow::submit(force),

        // Shell completion
        Commands::Completion { shell } => completion::execute(shell),
    }
}
