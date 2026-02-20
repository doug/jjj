//! Auto-push hooks for GitHub sync.
//!
//! These functions are called from existing commands when `auto_push` is enabled.
//! Network failures print warnings but never block local operations.

use crate::context::CommandContext;
use crate::models::{Problem, Solution};
use crate::sync::github::GitHubProvider;
use crate::sync::SyncProvider;

/// Auto-create a GitHub issue after a new problem is created.
pub fn auto_create_issue(ctx: &CommandContext, problem: &mut Problem) {
    if let Err(e) = try_auto_create_issue(ctx, problem) {
        eprintln!("Warning: auto-push to GitHub failed: {}", e);
    }
}

fn try_auto_create_issue(ctx: &CommandContext, problem: &mut Problem) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    if !config.github.auto_push {
        return Ok(());
    }

    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let number = provider.create_issue(problem)?;

    problem.github_issue = Some(number);
    ctx.store.save_problem(problem)?;

    println!("  (auto-created GitHub issue #{})", number);
    Ok(())
}

/// Auto-close a GitHub issue after a problem is solved.
pub fn auto_close_issue(ctx: &CommandContext, problem: &Problem) {
    if let Err(e) = try_auto_close_issue(ctx, problem) {
        eprintln!("Warning: auto-close GitHub issue failed: {}", e);
    }
}

fn try_auto_close_issue(ctx: &CommandContext, problem: &Problem) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    if !config.github.auto_push {
        return Ok(());
    }

    let issue_number = match problem.github_issue {
        Some(n) => n,
        None => return Ok(()),
    };

    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.close_issue(issue_number)?;

    println!("  (auto-closed GitHub issue #{})", issue_number);
    Ok(())
}

/// Auto-create or update a GitHub PR after submit.
pub fn auto_create_or_update_pr(ctx: &CommandContext, solution: &mut Solution) {
    if let Err(e) = try_auto_create_or_update_pr(ctx, solution) {
        eprintln!("Warning: auto-push PR to GitHub failed: {}", e);
    }
}

fn try_auto_create_or_update_pr(
    ctx: &CommandContext,
    solution: &mut Solution,
) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    if !config.github.auto_push {
        return Ok(());
    }

    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let problem = ctx.store.load_problem(&solution.problem_id)?;

    if solution.github_pr.is_some() {
        // PR exists, just push the branch (done by submit already)
        println!(
            "  (GitHub PR #{} will be updated on push)",
            solution.github_pr.unwrap()
        );
        return Ok(());
    }

    // Create new PR
    if solution.change_ids.is_empty() {
        return Ok(());
    }

    let short_id = &solution.id[..8.min(solution.id.len())];
    let branch = format!("jjj/s-{}", short_id);

    let pr_number = provider.create_pr(solution, &problem, &branch)?;
    solution.github_pr = Some(pr_number);
    solution.github_branch = Some(branch);
    ctx.store.save_solution(solution)?;

    println!("  (auto-created GitHub PR #{})", pr_number);
    Ok(())
}

/// Auto-merge a GitHub PR after a solution is accepted.
pub fn auto_merge_pr(ctx: &CommandContext, solution: &Solution) {
    if let Err(e) = try_auto_merge_pr(ctx, solution) {
        eprintln!("Warning: auto-merge GitHub PR failed: {}", e);
    }
}

fn try_auto_merge_pr(ctx: &CommandContext, solution: &Solution) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    if !config.github.auto_push {
        return Ok(());
    }

    let pr_number = match solution.github_pr {
        Some(n) => n,
        None => return Ok(()),
    };

    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.merge_pr(pr_number)?;

    println!("  (auto-merged GitHub PR #{})", pr_number);
    Ok(())
}
