pub mod board;
pub mod completion;
pub mod critique;
pub mod dashboard;
pub mod init;
pub mod milestone;
pub mod problem;
pub mod resolve;
pub mod solution;
pub mod tag;
pub mod workflow;

use crate::cli::{Cli, Commands, SolutionAction};
use crate::error::{JjjError, Result};

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::execute(),
        Commands::Board { json } => board::execute(json),
        Commands::Problem { action } => problem::execute(action),
        Commands::Solution { action } => solution::execute(action),
        Commands::Critique { action } => critique::execute(action),
        Commands::Dashboard { json } => dashboard::execute(json),
        Commands::Resolve { id, pick } => resolve::execute(id, pick),
        Commands::Milestone { action } => milestone::execute(action),
        Commands::Tag { action } => tag::execute(action),

        // Workflow Commands
        Commands::Start { arg, problem } => workflow::start(arg, problem),
        Commands::Submit { force } => workflow::submit(force),

        // Shell completion
        Commands::Completion { shell } => completion::execute(shell),

        // Shorthand commands
        Commands::ReviewShorthand { reviewers } => {
            let jj_client = crate::jj::JjClient::new()?;
            let store = crate::storage::MetadataStore::new(jj_client.clone())?;
            let change_id = jj_client.current_change_id()?;

            let solutions = store.list_solutions()?;
            let sol = solutions.iter()
                .find(|s| s.change_ids.contains(&change_id))
                .ok_or_else(|| JjjError::Other(
                    format!("No solution found for current change {}. Use 'jjj solution review <id>' instead.", change_id)
                ))?;

            solution::execute(SolutionAction::Review {
                solution_id: sol.id.clone(),
                reviewers,
            })
        }
        Commands::LgtmShorthand => {
            let jj_client = crate::jj::JjClient::new()?;
            let store = crate::storage::MetadataStore::new(jj_client.clone())?;
            let change_id = jj_client.current_change_id()?;

            let solutions = store.list_solutions()?;
            let sol = solutions.iter()
                .find(|s| s.change_ids.contains(&change_id))
                .ok_or_else(|| JjjError::Other(
                    format!("No solution found for current change {}. Use 'jjj solution lgtm <id>' instead.", change_id)
                ))?;

            solution::execute(SolutionAction::Lgtm {
                solution_id: sol.id.clone(),
            })
        }
    }
}
