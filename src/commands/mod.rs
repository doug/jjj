pub mod completion;
pub mod critique;
pub mod fetch;
pub mod init;
pub mod milestone;
pub mod problem;
pub mod push;
pub mod solution;
pub mod status;
pub mod ui;
pub mod workflow;

use crate::cli::{Cli, Commands};
use crate::context::CommandContext;
use crate::error::Result;

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        // Commands that don't need context (they create their own store or don't need one)
        Commands::Init => init::execute(),
        Commands::Ui => ui::execute(),
        Commands::Completion { shell } => completion::execute(shell),

        // All other commands need context - create it once here
        _ => {
            let ctx = CommandContext::new()?;
            execute_with_context(&ctx, cli.command)
        }
    }
}

fn execute_with_context(ctx: &CommandContext, command: Commands) -> Result<()> {
    match command {
        Commands::Problem { action } => problem::execute(ctx, action),
        Commands::Solution { action } => solution::execute(ctx, action),
        Commands::Critique { action } => critique::execute(ctx, action),
        Commands::Milestone { action } => milestone::execute(ctx, action),

        // Workflow Commands
        Commands::Submit { force } => workflow::submit(ctx, force),

        // Status (replaces dashboard + next)
        Commands::Status { all, mine, limit, json } => status::execute(ctx, all, mine, limit, json),

        // Sync commands
        Commands::Fetch { remote } => fetch::execute(ctx, &remote),
        Commands::Push { bookmarks, remote, no_prompt, dry_run } => {
            push::execute(ctx, bookmarks, &remote, no_prompt, dry_run)
        }

        // These are handled by execute() before calling this function
        Commands::Init | Commands::Ui | Commands::Completion { .. } => {
            unreachable!("These commands should be handled in execute()")
        }
    }
}
