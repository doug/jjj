use crate::context::CommandContext;
use crate::error::Result;
use crate::models::SolutionStatus;
use crate::sync::SyncProvider as _;

/// Find the trunk branch name for this repo (`main`, `master`, or `trunk`).
/// Returns `None` if none of the common names exist.
fn detect_trunk(jj_client: &crate::jj::JjClient) -> Option<&'static str> {
    for name in &["main", "master", "trunk"] {
        if jj_client.bookmark_exists(name).unwrap_or(false) {
            return Some(name);
        }
    }
    None
}

pub fn submit(ctx: &CommandContext, solution_input: Option<String>, force: bool) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    // Resolve the solution: explicit arg > current change > error
    let solution = if let Some(input) = solution_input {
        let id = ctx.resolve_solution(&input)?;
        store.load_solution(&id)?
    } else {
        let change_id = jj_client.current_change_id()?;
        let solutions = store.list_solutions()?;
        match solutions.into_iter().find(|s| s.change_ids.contains(&change_id)) {
            Some(s) => s,
            None => {
                // No solution attached — just squash onto trunk
                println!("No solution found for current change. Squashing only.");
                let desc = jj_client.change_description("@").unwrap_or_default();
                if let Some(trunk) = detect_trunk(jj_client) {
                    jj_client.execute(&["rebase", "-d", trunk])?;
                }
                let msg = if desc.is_empty() { None } else { Some(desc.as_str()) };
                jj_client.squash(msg)?;
                println!("Submitted successfully.");
                return Ok(());
            }
        }
    };

    println!("Submitting {}: {}", solution.id, solution.title);

    // Solution must be in Review (or already Accepted) — Proposed means not ready
    if solution.status == SolutionStatus::Proposed {
        return Err(crate::error::JjjError::Validation(format!(
            "Solution '{}' is proposed but hasn't been opened for review yet.\n\
             Move it to review first:\n  jjj solution review {}",
            solution.title, solution.id,
        )));
    }

    // Accept solution first (critique check, emit events, auto-solve).
    // This must run before any irreversible VCS operations so a failed
    // critique check aborts cleanly without squashing anything.
    super::solution::finalize_solution(ctx, &solution.id, force)?;
    println!("  Solution '{}' accepted.", solution.title);

    // Integrate code: PR merge if linked, otherwise rebase + squash onto trunk
    if let Some(pr_number) = solution.github_pr {
        let config = store.load_config()?;
        let repo_root = jj_client.repo_root();
        let provider = crate::sync::github::GitHubProvider::from_config(repo_root, &config.github)?;
        provider.merge_pr(pr_number)?;
        println!("  Merged PR #{}", pr_number);
    } else {
        let desc = jj_client.change_description("@").unwrap_or_default();
        if let Some(trunk) = detect_trunk(jj_client) {
            jj_client.execute(&["rebase", "-d", trunk])?;
        }
        let msg = if desc.is_empty() { None } else { Some(desc.as_str()) };
        jj_client.squash(msg)?;
    }

    println!("\nSquashed changes into trunk.");
    Ok(())
}
