//! Auto-push hooks for GitHub sync.
//!
//! Functions prefixed `do_` are the bare action implementations used by
//! both the legacy `auto_*` wrappers (driven by `auto_push` config)
//! and the new automation rule dispatcher.
//!
//! Network failures print warnings but never block local operations.

use crate::context::CommandContext;
use crate::display::short_id;
use crate::models::{Problem, Solution};
use crate::storage::MetadataStore;
use crate::sync::github::GitHubProvider;
use crate::sync::SyncProvider;

// ── Bare implementations (no config guard) ─────────────────────────

/// Create a GitHub issue for a problem. Mutates problem to set github_issue.
pub fn do_create_issue(store: &MetadataStore, problem: &mut Problem) -> crate::error::Result<()> {
    let config = store.load_config()?;
    let repo_root = store.jj_client.repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let number = provider.create_issue(problem)?;

    problem.github_issue = Some(number);
    store.save_problem(problem)?;

    println!("  (auto-created GitHub issue #{})", number);
    Ok(())
}

/// Close a GitHub issue linked to a problem.
pub fn do_close_issue(store: &MetadataStore, problem: &Problem) -> crate::error::Result<()> {
    let issue_number = match problem.github_issue {
        Some(n) => n,
        None => return Ok(()),
    };

    let config = store.load_config()?;
    let repo_root = store.jj_client.repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.close_issue(issue_number)?;

    println!("  (auto-closed GitHub issue #{})", issue_number);
    Ok(())
}

/// Create or update a GitHub PR for a solution.
pub fn do_create_or_update_pr(
    store: &MetadataStore,
    solution: &mut Solution,
) -> crate::error::Result<()> {
    let config = store.load_config()?;
    let repo_root = store.jj_client.repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let problem = store.load_problem(&solution.problem_id)?;

    if let Some(pr_number) = solution.github_pr {
        println!("  (GitHub PR #{} will be updated on push)", pr_number);
        return Ok(());
    }

    if solution.change_ids.is_empty() {
        return Ok(());
    }

    let sid = short_id(&solution.id);
    // `jjj-s-`, not `jjj/s-`: refs/heads/jjj is a file, so a nested
    // refs/heads/jjj/s-<id> is a git D/F conflict and the push is rejected.
    let branch = format!("jjj-s-{}", sid);

    // Auto-created PRs target the default branch.
    let pr_number = provider.create_pr(solution, &problem, &branch, "main")?;
    solution.github_pr = Some(pr_number);
    solution.github_branch = Some(branch);
    store.with_metadata("Link GitHub PR to solution", || {
        store.save_solution(solution)
    })?;

    println!("  (auto-created GitHub PR #{})", pr_number);
    Ok(())
}

/// Merge a GitHub PR for a solution.
pub fn do_merge_pr(store: &MetadataStore, solution: &Solution) -> crate::error::Result<()> {
    let pr_number = match solution.github_pr {
        Some(n) => n,
        None => return Ok(()),
    };

    let config = store.load_config()?;
    let repo_root = store.jj_client.repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;

    // Idempotent: if the PR is already merged (e.g. `jjj github merge` merged
    // it and then approval fired a github_merge automation rule), do nothing
    // instead of erroring on a double-merge.
    if provider.pr_is_merged(pr_number).unwrap_or(false) {
        return Ok(());
    }
    provider.merge_pr(pr_number)?;

    println!("  (auto-merged GitHub PR #{})", pr_number);
    Ok(())
}

// ── Legacy wrappers (check auto_push, used by existing command handlers) ──

/// Auto-create a GitHub issue after a new problem is created.
///
/// Skipped if an explicit automation rule matches `problem_created` — the
/// `automation::run` path handles the action in that case, and firing both
/// would create two GitHub issues.
pub fn auto_create_issue(ctx: &CommandContext, problem: &mut Problem) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !config.github.auto_push {
        return;
    }
    if crate::automation::has_explicit_rule(
        &config.automation,
        &crate::models::EventType::ProblemCreated,
    ) {
        return;
    }
    if let Err(e) = do_create_issue(&ctx.store, problem) {
        eprintln!("Warning: auto-push to GitHub failed: {}", e);
    }
}

/// Auto-close a GitHub issue after a problem is solved or dissolved.
///
/// Triggers when any of these are true:
/// - `force` is set (caller passed `--github-close`)
/// - `github.auto_close_on_solve = true` in config
/// - `github.auto_push = true` in config (coarse-grained catch-all)
///
/// Skipped if an explicit automation rule matches `event_type` (the triggering
/// `problem_solved` / `problem_dissolved` event) — that rule fires the action;
/// firing both would close the issue twice.
///
/// Safe to call unconditionally: with `force = false` and no relevant config,
/// it is a no-op. (It must NOT be gated behind `--github-close` at the call
/// site, or `auto_close_on_solve`/`auto_push` would never take effect.)
pub fn auto_close_issue(
    ctx: &CommandContext,
    problem: &Problem,
    force: bool,
    event_type: &crate::models::EventType,
) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !force && !config.github.auto_push && !config.github.auto_close_on_solve {
        return;
    }
    if !force && crate::automation::has_explicit_rule(&config.automation, event_type) {
        return;
    }
    if let Err(e) = do_close_issue(&ctx.store, problem) {
        eprintln!("Warning: auto-close GitHub issue failed: {}", e);
    }
}
