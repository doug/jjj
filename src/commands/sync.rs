//! Sync command handler for bidirectional sync with external systems.

use crate::cli::GitHubSyncAction;
use crate::context::CommandContext;
use crate::display::short_id;
use crate::error::{JjjError, Result};
use crate::models::{Event, EventExtra, EventType};
use crate::sync::github::GitHubProvider;
use crate::sync::{IssueStatus, PrStatus, ReviewState, SyncProvider};

pub fn execute(
    ctx: &CommandContext,
    action: Option<GitHubSyncAction>,
    dry_run: bool,
) -> Result<()> {
    execute_github(ctx, action, dry_run)
}

fn execute_github(
    ctx: &CommandContext,
    action: Option<GitHubSyncAction>,
    dry_run: bool,
) -> Result<()> {
    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();

    // Check if GitHub integration is explicitly disabled
    if config.github.enabled == Some(false) {
        return Err(JjjError::Config(
            "GitHub integration is disabled. Set github.enabled = true in config.toml to enable."
                .to_string(),
        ));
    }

    let provider = GitHubProvider::from_config(repo_root, &config.github)?;

    match action {
        None => sync_pull(ctx, &provider, dry_run),
        Some(GitHubSyncAction::Import { issue, all, label }) => {
            sync_import(ctx, &provider, issue, all, label, dry_run)
        }
        Some(GitHubSyncAction::Pr { solution_id, base }) => {
            sync_pr(ctx, &provider, solution_id, &base, dry_run)
        }
        Some(GitHubSyncAction::Status) => sync_status(ctx, &provider),
        Some(GitHubSyncAction::Merge { solution_id }) => {
            sync_merge(ctx, &provider, &solution_id, dry_run)
        }
        Some(GitHubSyncAction::Close { problem_id }) => {
            sync_close(ctx, &provider, &problem_id, dry_run)
        }
        Some(GitHubSyncAction::Reopen { problem_id }) => {
            sync_reopen(ctx, &provider, &problem_id, dry_run)
        }
        Some(GitHubSyncAction::Push) => sync_push(ctx, &provider, dry_run),
    }
}

/// Pull reviews and check status for all linked entities.
fn sync_pull(ctx: &CommandContext, provider: &GitHubProvider, dry_run: bool) -> Result<()> {
    let (owner, repo) = provider.detect_repo()?;
    println!("Syncing with {}/{}...\n", owner, repo);

    let solutions = ctx.store.list_solutions()?;
    let config = ctx.store.load_config()?;
    let mut imported_reviews = 0;

    for solution in &solutions {
        if let Some(pr_number) = solution.github_pr {
            // Check PR status
            match provider.pr_status(pr_number) {
                Ok(status) => {
                    let status_str = match status {
                        PrStatus::Open => "open",
                        PrStatus::Merged => "merged",
                        PrStatus::Closed => "closed",
                    };
                    println!("  PR #{} ({}) — {}", pr_number, solution.title, status_str);
                }
                Err(e) => {
                    eprintln!("  Warning: could not check PR #{}: {}", pr_number, e);
                    continue;
                }
            }

            // Pull reviews if enabled
            if config.github.sync_critiques {
                // Collect already-imported IDs once (covers both top-level reviews
                // and inline threads since both use github_review_id for dedup).
                let existing_review_ids: std::collections::HashSet<u64> = ctx
                    .store
                    .list_critiques_for_solution(&solution.id)?
                    .iter()
                    .filter_map(|c| c.github_review_id)
                    .collect();

                // Collect all new critiques (reviews + threads) before saving any.
                // Each entry is (critique, event, display_message).
                let mut new_critiques: Vec<(crate::models::Critique, Event, String)> = Vec::new();

                // --- Top-level PR reviews ---
                match provider.pull_reviews(pr_number) {
                    Ok(reviews) => {
                        for review in &reviews {
                            if existing_review_ids.contains(&review.id) {
                                continue;
                            }
                            if review.state == ReviewState::Approved && !config.github.sync_lgtm {
                                continue;
                            }
                            if review.state == ReviewState::Commented && review.body.is_empty() {
                                continue;
                            }

                            if dry_run {
                                println!(
                                    "    Would import review from @{} ({:?})",
                                    review.author, review.state
                                );
                            } else {
                                let critique_id = crate::id::generate_id();
                                let critique = crate::sync::review_to_critique(
                                    review,
                                    &solution.id,
                                    critique_id,
                                );
                                let event = Event::new(
                                    EventType::GithubReviewImported,
                                    critique.id.clone(),
                                    review.author.clone(),
                                )
                                .with_extra(EventExtra {
                                    github_number: Some(pr_number),
                                    target: Some(solution.id.clone()),
                                    ..Default::default()
                                });
                                let msg = format!(
                                    "    Imported review from @{} as critique {}",
                                    review.author,
                                    short_id(&critique.id)
                                );
                                new_critiques.push((critique, event, msg));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "  Warning: could not pull reviews for PR #{}: {}",
                            pr_number, e
                        );
                    }
                }

                // --- Inline review threads (file-level comments) ---
                match provider.pull_review_threads(pr_number) {
                    Ok(threads) => {
                        for thread in &threads {
                            if thread.is_resolved {
                                continue;
                            }
                            if existing_review_ids.contains(&thread.comment_id) {
                                continue;
                            }

                            let loc = match thread.line {
                                Some(l) => format!("{}:{}", thread.path, l),
                                None => thread.path.clone(),
                            };

                            if dry_run {
                                println!(
                                    "    Would import inline comment from @{} at {}",
                                    thread.author, loc
                                );
                            } else {
                                let critique_id = crate::id::generate_id();
                                let critique = crate::sync::thread_to_critique(
                                    thread,
                                    &solution.id,
                                    critique_id,
                                );
                                let event = Event::new(
                                    EventType::GithubReviewImported,
                                    critique.id.clone(),
                                    thread.author.clone(),
                                )
                                .with_extra(EventExtra {
                                    github_number: Some(pr_number),
                                    target: Some(solution.id.clone()),
                                    ..Default::default()
                                });
                                let msg = format!(
                                    "    Imported inline comment from @{} at {} as critique {}",
                                    thread.author,
                                    loc,
                                    short_id(&critique.id)
                                );
                                new_critiques.push((critique, event, msg));
                            }
                        }
                    }
                    Err(e) => {
                        // Not all gh versions support reviewThreads — warn but don't fail
                        eprintln!(
                            "  Warning: could not pull inline comments for PR #{}: {}",
                            pr_number, e
                        );
                    }
                }

                // Save all new critiques in a single metadata commit.
                if !new_critiques.is_empty() {
                    let n = new_critiques.len();
                    let commit_msg = format!("GitHub sync: {} review(s) for PR #{}", n, pr_number);
                    ctx.store.with_metadata(&commit_msg, || {
                        for (critique, event, _) in &new_critiques {
                            ctx.store.set_pending_event(event.clone());
                            ctx.store.save_critique(critique)?;
                        }
                        Ok(())
                    })?;
                    for (_, _, msg) in &new_critiques {
                        println!("{}", msg);
                    }
                    imported_reviews += n;
                }
            }
        }
    }

    // Check linked issues
    let problems = ctx.store.list_problems()?;
    for problem in &problems {
        if let Some(issue_number) = problem.github_issue {
            match provider.issue_status(issue_number) {
                Ok(status) => {
                    let status_str = match status {
                        IssueStatus::Open => "open",
                        IssueStatus::Closed => "closed",
                    };
                    println!(
                        "  Issue #{} ({}) — {}",
                        issue_number, problem.title, status_str
                    );
                }
                Err(e) => {
                    eprintln!("  Warning: could not check issue #{}: {}", issue_number, e);
                }
            }
        }
    }

    // List unlinked issues
    let existing: Vec<(String, u64)> = problems
        .iter()
        .filter_map(|p| p.github_issue.map(|n| (p.id.clone(), n)))
        .collect();

    match provider.list_unlinked_issues(&existing, None) {
        Ok(unlinked) if !unlinked.is_empty() => {
            println!("\nUnlinked GitHub issues:");
            for (number, title) in &unlinked {
                println!("  #{} — {}", number, title);
            }
            println!("\nUse 'jjj github import <number>' to import.");
        }
        _ => {}
    }

    if imported_reviews > 0 {
        println!("\nImported {} review(s) as critiques.", imported_reviews);
    }

    println!("\nSync complete.");
    Ok(())
}

/// Import a GitHub issue as a jjj problem.
fn sync_import(
    ctx: &CommandContext,
    provider: &GitHubProvider,
    issue: Option<String>,
    all: bool,
    label: Option<String>,
    dry_run: bool,
) -> Result<()> {
    if all {
        // Import all unlinked issues (optionally filtered by label)
        let problems = ctx.store.list_problems()?;
        let existing: Vec<(String, u64)> = problems
            .iter()
            .filter_map(|p| p.github_issue.map(|n| (p.id.clone(), n)))
            .collect();

        let unlinked = provider.list_unlinked_issues(&existing, label.as_deref())?;

        if unlinked.is_empty() {
            println!("No unlinked GitHub issues found.");
            return Ok(());
        }

        for (number, title) in &unlinked {
            if dry_run {
                println!("Would import issue #{} — {}", number, title);
            } else {
                import_single_issue(ctx, provider, *number)?;
            }
        }
        return Ok(());
    }

    let issue_str = issue.ok_or_else(|| {
        JjjError::Validation(
            "Please specify an issue number (e.g., '#123' or '123'), or use --all".to_string(),
        )
    })?;

    let number = parse_issue_number(&issue_str)?;

    if dry_run {
        let problem = provider.import_issue(number)?;
        println!(
            "Would import issue #{} as problem: {}",
            number, problem.title
        );
        return Ok(());
    }

    import_single_issue(ctx, provider, number)
}

fn import_single_issue(ctx: &CommandContext, provider: &GitHubProvider, number: u64) -> Result<()> {
    // Check if already linked
    let problems = ctx.store.list_problems()?;
    if let Some(existing) = problems.iter().find(|p| p.github_issue == Some(number)) {
        println!(
            "Issue #{} is already linked to problem {} ({})",
            number,
            short_id(&existing.id),
            existing.title
        );
        return Ok(());
    }

    let problem = provider.import_issue(number)?;
    let sid = short_id(&problem.id).to_string();

    ctx.store.with_metadata("GitHub sync", || {
        let event = Event::new(
            EventType::GithubIssueImported,
            problem.id.clone(),
            "sync".to_string(),
        )
        .with_extra(EventExtra {
            github_number: Some(number),
            ..Default::default()
        });
        ctx.store.set_pending_event(event);
        ctx.store.save_problem(&problem)?;
        Ok(())
    })?;

    println!(
        "Imported issue #{} as problem {} ({})",
        number, sid, problem.title
    );
    Ok(())
}

/// Create or update a GitHub PR for a solution.
fn sync_pr(
    ctx: &CommandContext,
    provider: &GitHubProvider,
    solution_id: Option<String>,
    _base: &str,
    dry_run: bool,
) -> Result<()> {
    let sol_id = match solution_id {
        Some(ref id) => ctx.resolve_solution(id)?,
        None => {
            // Try to find solution for current change
            let change_id =
                ctx.jj()
                    .execute(&["log", "-r", "@", "-T", "change_id", "--no-graph"])?;
            let change_id = change_id.trim();

            let solutions = ctx.store.list_solutions()?;
            let found = solutions
                .iter()
                .find(|s| s.change_ids.iter().any(|c| c == change_id));

            match found {
                Some(s) => s.id.clone(),
                None => return Err(JjjError::Validation(
                    "No solution specified and current change is not attached to any solution.\n\
                         Use 'jjj sync github pr <solution>' to specify."
                        .to_string(),
                )),
            }
        }
    };

    let solution = ctx.store.load_solution(&sol_id)?;
    let problem = ctx.store.load_problem(&solution.problem_id)?;

    // Generate branch name
    let sid = short_id(&sol_id);
    let branch = format!("jjj/s-{}", sid);

    if let Some(pr_number) = solution.github_pr {
        // PR already exists — update by pushing branch
        if dry_run {
            println!(
                "Would update PR #{} for solution {} by pushing branch {}",
                pr_number, solution.title, branch
            );
            return Ok(());
        }

        println!(
            "PR #{} already exists for solution {}. Push branch '{}' to update.",
            pr_number, solution.title, branch
        );
        return Ok(());
    }

    if dry_run {
        println!(
            "Would create PR for solution '{}' on branch '{}'",
            solution.title, branch
        );
        return Ok(());
    }

    // Create git branch from solution's changes
    // First, push the branch using jj git push
    if solution.change_ids.is_empty() {
        return Err(JjjError::Validation(
            "Solution has no attached changes. Attach changes first with 'jjj solution attach'."
                .to_string(),
        ));
    }

    let change_id = &solution.change_ids[0];

    // Set the bookmark on the solution's change.
    // Bookmarks are a core jj concept — no backend config needed.
    ctx.jj().execute(&[
        "--ignore-working-copy",
        "bookmark",
        "set",
        &branch,
        "-r",
        change_id,
    ])?;

    // Push the branch using configured or default sync command
    let sync_config = ctx.store.load_config().unwrap_or_default().sync;
    let has_git = ctx.jj().has_git_backend();
    let push_cmd = match sync_config.resolve_push(has_git) {
        Some(cmd) => cmd,
        None => {
            return Err(JjjError::Validation(
                "Creating PRs requires a sync backend for pushing branches.\n\
                 Configure [sync] push in config.toml."
                    .to_string(),
            ));
        }
    };
    let vars = [("bookmark", branch.as_str()), ("remote", "origin")];
    ctx.jj().execute_sync_command(&push_cmd, &vars)?;

    // Create the PR
    let pr_number = provider.create_pr(&solution, &problem, &branch)?;

    // Save the PR number and branch back to the solution
    let mut updated_solution = solution.clone();
    updated_solution.github_pr = Some(pr_number);
    updated_solution.github_branch = Some(branch.clone());

    ctx.store.with_metadata("GitHub sync", || {
        let event = Event::new(
            EventType::GithubPrCreated,
            updated_solution.id.clone(),
            "sync".to_string(),
        )
        .with_extra(EventExtra {
            github_number: Some(pr_number),
            problem: Some(problem.id.clone()),
            ..Default::default()
        });
        ctx.store.set_pending_event(event);
        ctx.store.save_solution(&updated_solution)?;
        Ok(())
    })?;

    println!(
        "Created PR #{} for solution '{}' on branch '{}'",
        pr_number, updated_solution.title, branch
    );
    Ok(())
}

/// Show sync status for all linked entities.
fn sync_status(ctx: &CommandContext, provider: &GitHubProvider) -> Result<()> {
    match provider.detect_repo() {
        Ok((owner, repo)) => println!("Repository: {}/{}", owner, repo),
        Err(_) => println!("Repository: not detected (set github.repo in config)"),
    }

    match provider.check_auth() {
        Ok(user) => println!("Authenticated as: {}", user),
        Err(_) => println!("Authentication: not configured (run 'gh auth login')"),
    }

    let config = ctx.store.load_config()?;
    println!(
        "Auto-push: {}",
        if config.github.auto_push {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("Sync critiques: {}", config.github.sync_critiques);
    println!("Sync LGTM: {}", config.github.sync_lgtm);
    println!("Auto-close on solve: {}", config.github.auto_close_on_solve);
    println!("Problem label: {}", config.github.problem_label);

    // Show linked problems
    let problems = ctx.store.list_problems()?;
    let linked_problems: Vec<_> = problems
        .iter()
        .filter(|p| p.github_issue.is_some())
        .collect();

    if !linked_problems.is_empty() {
        println!("\nLinked problems:");
        for p in &linked_problems {
            if let Some(issue_num) = p.github_issue {
                println!(
                    "  p/{} — #{} — {} [{}]",
                    short_id(&p.id),
                    issue_num,
                    p.title,
                    p.status
                );
            }
        }
    }

    // Show linked solutions
    let solutions = ctx.store.list_solutions()?;
    let linked_solutions: Vec<_> = solutions.iter().filter(|s| s.github_pr.is_some()).collect();

    if !linked_solutions.is_empty() {
        println!("\nLinked solutions:");
        for s in &linked_solutions {
            if let Some(pr_num) = s.github_pr {
                println!(
                    "  s/{} — PR #{} — {} [{}]",
                    short_id(&s.id),
                    pr_num,
                    s.title,
                    s.status
                );
            }
        }
    }

    if linked_problems.is_empty() && linked_solutions.is_empty() {
        println!("\nNo linked entities. Use 'jjj sync github import' to import issues.");
    }

    Ok(())
}

/// Merge a linked GitHub PR.
fn sync_merge(
    ctx: &CommandContext,
    provider: &GitHubProvider,
    solution_input: &str,
    dry_run: bool,
) -> Result<()> {
    let sol_id = ctx.resolve_solution(solution_input)?;
    let solution = ctx.store.load_solution(&sol_id)?;

    let pr_number = solution.github_pr.ok_or_else(|| {
        JjjError::Validation(format!(
            "Solution '{}' is not linked to a GitHub PR",
            solution.title
        ))
    })?;

    if dry_run {
        println!(
            "Would merge PR #{} for solution '{}'",
            pr_number, solution.title
        );
        return Ok(());
    }

    provider.merge_pr(pr_number)?;

    // Record GithubPrMerged event
    ctx.store.with_metadata("GitHub sync: PR merged", || {
        let event = Event::new(
            EventType::GithubPrMerged,
            solution.id.clone(),
            "sync".to_string(),
        )
        .with_extra(EventExtra {
            github_number: Some(pr_number),
            ..Default::default()
        });
        ctx.store.set_pending_event(event);
        Ok(())
    })?;

    // Approve solution, emit SolutionApproved event, auto-solve problem
    crate::domain::approve_solution(&ctx.store, &sol_id, false, None)?;

    println!(
        "Merged PR #{} and approved solution '{}'",
        pr_number, solution.title
    );
    Ok(())
}

/// Close the linked GitHub issue for a problem.
fn sync_close(
    ctx: &CommandContext,
    provider: &GitHubProvider,
    problem_input: &str,
    dry_run: bool,
) -> Result<()> {
    let prob_id = ctx.resolve_problem(problem_input)?;
    let problem = ctx.store.load_problem(&prob_id)?;

    let issue_number = problem.github_issue.ok_or_else(|| {
        JjjError::Validation(format!(
            "Problem '{}' is not linked to a GitHub issue",
            problem.title
        ))
    })?;

    if dry_run {
        println!(
            "Would close issue #{} for problem '{}'",
            issue_number, problem.title
        );
        return Ok(());
    }

    provider.close_issue(issue_number)?;

    ctx.store.with_metadata("GitHub sync", || {
        let event = Event::new(
            EventType::GithubIssueClosed,
            problem.id.clone(),
            "sync".to_string(),
        )
        .with_extra(EventExtra {
            github_number: Some(issue_number),
            ..Default::default()
        });
        ctx.store.set_pending_event(event);
        Ok(())
    })?;

    println!(
        "Closed issue #{} for problem '{}'",
        issue_number, problem.title
    );
    Ok(())
}

/// Reopen the linked GitHub issue for a problem.
fn sync_reopen(
    ctx: &CommandContext,
    provider: &GitHubProvider,
    problem_input: &str,
    dry_run: bool,
) -> Result<()> {
    let prob_id = ctx.resolve_problem(problem_input)?;
    let problem = ctx.store.load_problem(&prob_id)?;

    let issue_number = problem.github_issue.ok_or_else(|| {
        JjjError::Validation(format!(
            "Problem '{}' is not linked to a GitHub issue",
            problem.title
        ))
    })?;

    if dry_run {
        println!(
            "Would reopen issue #{} for problem '{}'",
            issue_number, problem.title
        );
        return Ok(());
    }

    provider.reopen_issue(issue_number)?;

    // Reopen the problem locally if it's solved/dissolved
    if problem.is_resolved() {
        crate::domain::reopen_problem(&ctx.store, &prob_id)?;
    }

    println!(
        "Reopened issue #{} for problem '{}'",
        issue_number, problem.title
    );
    Ok(())
}

/// Push local state to GitHub: refresh PR bodies and reconcile issue open/closed state.
///
/// Each GitHub API call is attempted independently so that a single failure does not
/// prevent subsequent operations. On completion, a summary of succeeded/failed
/// operations is printed so the user can manually reconcile any divergence.
fn sync_push(ctx: &CommandContext, provider: &GitHubProvider, dry_run: bool) -> Result<()> {
    use crate::models::ProblemStatus;
    let solutions = ctx.store.list_solutions()?;
    let problems = ctx.store.list_problems()?;

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    // 1. Refresh PR bodies for solutions with a linked PR
    for solution in &solutions {
        let pr_number = match solution.github_pr {
            Some(n) => n,
            None => continue,
        };

        let problem = match problems.iter().find(|p| p.id == solution.problem_id) {
            Some(p) => p,
            None => continue,
        };

        let critiques = ctx.store.list_critiques_for_solution(&solution.id)?;
        let new_body = crate::sync::github::mapping::format_pr_body(solution, problem, &critiques);

        if dry_run {
            println!("Would update PR #{}: {}", pr_number, solution.title);
            succeeded.push(format!("PR #{} (dry run)", pr_number));
            continue;
        }

        // Skip write if the body is already up to date
        match provider.get_pr_body(pr_number) {
            Ok(current) if current.trim() == new_body.trim() => {
                continue;
            }
            _ => {}
        }

        match provider.update_pr_body(pr_number, solution, problem, &critiques) {
            Ok(()) => {
                println!("Updated PR #{}: {}", pr_number, solution.title);
                succeeded.push(format!("update PR #{}", pr_number));
            }
            Err(e) => {
                eprintln!("  Error: failed to update PR #{}: {}", pr_number, e);
                failed.push(format!("update PR #{}: {}", pr_number, e));
            }
        }
    }

    // 2. Reconcile issue open/closed state for problems with a linked issue
    for problem in &problems {
        let issue_number = match problem.github_issue {
            Some(n) => n,
            None => continue,
        };

        let live_status = match provider.issue_status(issue_number) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  Warning: could not check issue #{}: {}", issue_number, e);
                failed.push(format!("check issue #{}: {}", issue_number, e));
                continue;
            }
        };

        let should_be_closed = matches!(
            problem.status,
            ProblemStatus::Solved | ProblemStatus::Dissolved
        );

        match (should_be_closed, &live_status) {
            (true, IssueStatus::Open) => {
                if dry_run {
                    println!(
                        "Would close issue #{} (problem {} is {})",
                        issue_number, problem.title, problem.status
                    );
                    succeeded.push(format!("issue #{} (dry run)", issue_number));
                } else {
                    match provider.close_issue(issue_number) {
                        Ok(()) => {
                            println!(
                                "Closed issue #{}: {} (problem is {})",
                                issue_number, problem.title, problem.status
                            );
                            succeeded.push(format!("close issue #{}", issue_number));
                        }
                        Err(e) => {
                            eprintln!("  Error: failed to close issue #{}: {}", issue_number, e);
                            failed.push(format!("close issue #{}: {}", issue_number, e));
                        }
                    }
                }
            }
            (false, IssueStatus::Closed) => {
                if dry_run {
                    println!(
                        "Would reopen issue #{} (problem {} is {})",
                        issue_number, problem.title, problem.status
                    );
                    succeeded.push(format!("issue #{} (dry run)", issue_number));
                } else {
                    match provider.reopen_issue(issue_number) {
                        Ok(()) => {
                            println!(
                                "Reopened issue #{}: {} (problem is {})",
                                issue_number, problem.title, problem.status
                            );
                            succeeded.push(format!("reopen issue #{}", issue_number));
                        }
                        Err(e) => {
                            eprintln!("  Error: failed to reopen issue #{}: {}", issue_number, e);
                            failed.push(format!("reopen issue #{}: {}", issue_number, e));
                        }
                    }
                }
            }
            _ => {} // Already in sync
        }
    }

    if succeeded.is_empty() && failed.is_empty() {
        println!("Nothing to push — GitHub is already up to date.");
    } else if !failed.is_empty() {
        eprintln!(
            "\nSync completed with errors ({} succeeded, {} failed):",
            succeeded.len(),
            failed.len()
        );
        for f in &failed {
            eprintln!("  FAILED: {}", f);
        }
        eprintln!("\nManually reconcile the failed operations above.");
        return Err(JjjError::Validation(format!(
            "{} of {} operation(s) failed during push — see errors above",
            failed.len(),
            succeeded.len() + failed.len()
        )));
    }

    Ok(())
}

/// Parse an issue number from user input like "#123" or "123".
fn parse_issue_number(input: &str) -> Result<u64> {
    let s = input.trim().trim_start_matches('#');
    s.parse().map_err(|_| {
        JjjError::Validation(format!(
            "Invalid issue number: '{}'. Use a number like '#123' or '123'.",
            input
        ))
    })
}
