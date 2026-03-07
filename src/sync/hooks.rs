//! Auto-push hooks for GitHub sync.
//!
//! Functions prefixed `do_` are the bare action implementations used by
//! both the legacy `auto_*` wrappers (driven by `auto_push` config)
//! and the new automation rule dispatcher.
//!
//! Network failures print warnings but never block local operations.

use crate::context::CommandContext;
use crate::models::{Problem, Solution};
use crate::sync::github::GitHubProvider;
use crate::sync::SyncProvider;

// ── Bare implementations (no config guard) ─────────────────────────

/// Create a GitHub issue for a problem. Mutates problem to set github_issue.
pub fn do_create_issue(ctx: &CommandContext, problem: &mut Problem) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let number = provider.create_issue(problem)?;

    problem.github_issue = Some(number);
    ctx.store.save_problem(problem)?;

    println!("  (auto-created GitHub issue #{})", number);
    Ok(())
}

/// Close a GitHub issue linked to a problem.
pub fn do_close_issue(ctx: &CommandContext, problem: &Problem) -> crate::error::Result<()> {
    let issue_number = match problem.github_issue {
        Some(n) => n,
        None => return Ok(()),
    };

    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.close_issue(issue_number)?;

    println!("  (auto-closed GitHub issue #{})", issue_number);
    Ok(())
}

/// Create or update a GitHub PR for a solution.
pub fn do_create_or_update_pr(
    ctx: &CommandContext,
    solution: &mut Solution,
) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let problem = ctx.store.load_problem(&solution.problem_id)?;

    if let Some(pr_number) = solution.github_pr {
        println!("  (GitHub PR #{} will be updated on push)", pr_number);
        return Ok(());
    }

    if solution.change_ids.is_empty() {
        return Ok(());
    }

    let short_id = &solution.id[..8.min(solution.id.len())];
    let branch = format!("jjj/s-{}", short_id);

    let pr_number = provider.create_pr(solution, &problem, &branch)?;
    solution.github_pr = Some(pr_number);
    solution.github_branch = Some(branch);
    ctx.store.with_metadata("Link GitHub PR to solution", || {
        ctx.store.save_solution(solution)
    })?;

    println!("  (auto-created GitHub PR #{})", pr_number);
    Ok(())
}

/// Merge a GitHub PR for a solution.
pub fn do_merge_pr(ctx: &CommandContext, solution: &Solution) -> crate::error::Result<()> {
    let pr_number = match solution.github_pr {
        Some(n) => n,
        None => return Ok(()),
    };

    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.merge_pr(pr_number)?;

    println!("  (auto-merged GitHub PR #{})", pr_number);
    Ok(())
}

// ── Legacy wrappers (check auto_push, used by existing command handlers) ──

/// Auto-create a GitHub issue after a new problem is created.
pub fn auto_create_issue(ctx: &CommandContext, problem: &mut Problem) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !config.github.auto_push {
        return;
    }
    if let Err(e) = do_create_issue(ctx, problem) {
        eprintln!("Warning: auto-push to GitHub failed: {}", e);
    }
}

/// Auto-close a GitHub issue after a problem is solved.
///
/// Triggers when any of these are true:
/// - `force` is set (caller passed `--github-close`)
/// - `github.auto_close_on_solve = true` in config
/// - `github.auto_push = true` in config (coarse-grained catch-all)
pub fn auto_close_issue(ctx: &CommandContext, problem: &Problem, force: bool) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !force && !config.github.auto_push && !config.github.auto_close_on_solve {
        return;
    }
    if let Err(e) = do_close_issue(ctx, problem) {
        eprintln!("Warning: auto-close GitHub issue failed: {}", e);
    }
}

