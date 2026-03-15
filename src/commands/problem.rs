use crate::cli::ProblemAction;
use crate::commands::show_related_items;
use crate::context::CommandContext;
use crate::db::{search, Database};
use crate::display::truncated_prefixes;
use crate::embeddings::EmbeddingClient;
use crate::error::Result;
use crate::local_config::LocalConfig;
use crate::models::{Event, EventType, Priority, Problem, ProblemStatus};
use crate::storage::MetadataStore;

pub fn execute(ctx: &CommandContext, action: ProblemAction) -> Result<()> {
    match action {
        ProblemAction::New {
            title,
            priority,
            parent,
            milestone,
            force,
            context,
            tags,
        } => new_problem(
            ctx, title, priority, parent, milestone, force, context, tags,
        ),
        ProblemAction::List {
            status,
            tree,
            milestone,
            search,
            assignee,
            tag,
            sort,
            json,
        } => list_problems(
            ctx,
            status,
            tree,
            milestone,
            search.as_deref(),
            assignee,
            tag,
            &sort,
            json,
        ),
        ProblemAction::Show { problem_id, json } => show_problem(ctx, problem_id, json),
        ProblemAction::Edit {
            problem_id,
            title,
            status,
            priority,
            parent,
            add_tag,
            remove_tag,
            set_tags,
        } => edit_problem(
            ctx, problem_id, title, status, priority, parent, add_tag, remove_tag, set_tags,
        ),
        ProblemAction::Tree { problem_id } => show_tree(ctx, problem_id),
        ProblemAction::Solve {
            problem_id,
            github_close,
        } => solve_problem(ctx, problem_id, github_close),
        ProblemAction::Dissolve {
            problem_id,
            reason,
            github_close,
        } => dissolve_problem(ctx, problem_id, reason, github_close),
        ProblemAction::Assign { problem_id, to } => assign_problem(ctx, problem_id, to),
        ProblemAction::Reopen { problem_id } => reopen_problem(ctx, problem_id),
        ProblemAction::Duplicate { problem_id, of } => duplicate_problem(ctx, problem_id, of),
        ProblemAction::Graph { milestone, all } => graph_problems(ctx, milestone, all),
    }
}

#[allow(clippy::too_many_arguments)]
fn new_problem(
    ctx: &CommandContext,
    title: String,
    priority: String,
    parent: Option<String>,
    milestone: Option<String>,
    force: bool,
    context: Option<String>,
    tags: Vec<String>,
) -> Result<()> {
    let store = &ctx.store;

    // Validate title is not empty
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(crate::error::JjjError::Validation(
            "Title cannot be empty.".to_string(),
        ));
    }

    // If not forcing, check for exact title duplicates
    if !force {
        let existing = store.list_problems()?;
        let exact_match = existing
            .iter()
            .find(|p| p.title.to_lowercase() == title.to_lowercase());
        if let Some(dup) = exact_match {
            eprintln!(
                "Warning: A problem with a similar title already exists: '{}' ({})",
                dup.title,
                &dup.id[..8.min(dup.id.len())]
            );
            eprintln!("Use --force to create anyway.");
            return Err(crate::error::JjjError::Validation(
                "Duplicate title. Use --force to create anyway.".to_string(),
            ));
        }

        // Check for similar problems via FTS text search (best-effort, skip on error)
        let repo_root = ctx.jj().repo_root().to_path_buf();
        let db_path = repo_root.join(".jj").join("jjj.db");
        if db_path.exists() {
            if let Ok(db) = Database::open(&db_path) {
                if let Ok(results) = search::search(db.conn(), &title, Some("problem")) {
                    if !results.is_empty() {
                        eprintln!("Warning: similar problems already exist:");
                        for r in &results {
                            let short_id = &r.entity_id[..6.min(r.entity_id.len())];
                            eprintln!("  p/{} — \"{}\"", short_id, r.title);
                        }
                        eprintln!("\nUse --force to create anyway.");
                        return Err(crate::error::JjjError::Validation(
                            "Similar entities exist. Use --force to override.".to_string(),
                        ));
                    }
                }
            }
        }

        // Also check semantic duplicates via embeddings (if available)
        if let Some(similar) = check_for_duplicates(ctx, &title)? {
            if !prompt_create_anyway(&similar)? {
                println!("Cancelled.");
                return Ok(());
            }
        }
    }

    // Resolve and validate parent if provided
    let resolved_parent = if let Some(ref parent_input) = parent {
        Some(ctx.resolve_problem(parent_input)?)
    } else {
        None
    };

    // Resolve and validate milestone if provided
    let resolved_milestone = if let Some(ref milestone_input) = milestone {
        Some(ctx.resolve_milestone(milestone_input)?)
    } else {
        None
    };

    let user = store.get_current_user()?;

    let created_id = std::cell::RefCell::new(String::new());
    store.with_metadata(&format!("Create problem: {}", title), || {
        let problem_id = store.next_problem_id()?;
        let mut problem = Problem::new(problem_id.clone(), title.clone());

        // Set priority
        problem.priority = priority
            .parse::<Priority>()
            .map_err(|e: String| crate::error::JjjError::Validation(e))?;

        // Set context
        if let Some(ref ctx_text) = context {
            problem.context = ctx_text.clone();
        }

        // Set tags (trim, dedup, sort)
        if !tags.is_empty() {
            let mut t: Vec<String> = tags
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            t.sort();
            t.dedup();
            problem.tags = t;
        }

        // Set parent
        if let Some(ref parent_id) = resolved_parent {
            problem.set_parent(Some(parent_id.clone()));
        }

        // Set milestone
        if let Some(ref milestone_id) = resolved_milestone {
            problem.set_milestone(Some(milestone_id.clone()));

            // Update milestone's problem_ids
            let mut ms = store.load_milestone(milestone_id)?;
            ms.add_problem(problem_id.clone());
            store.save_milestone(&ms)?;
        }

        // Create event for decision log
        let event = Event::new(EventType::ProblemCreated, problem_id.clone(), user.clone());
        store.set_pending_event(event);

        store.save_problem(&problem)?;

        println!("Created problem {} ({})", problem.id, problem.title);
        if let Some(ref parent_id) = resolved_parent {
            println!("  Parent: {}", parent_id);
        }
        *created_id.borrow_mut() = problem_id;
        Ok(())
    })?;

    // Automation: fire rules for problem_created, with legacy auto_push fallback
    let pid = created_id.into_inner();
    if !pid.is_empty() {
        if let Ok(mut problem) = ctx.store.load_problem(&pid) {
            let has_rules = ctx
                .store
                .load_config()
                .ok()
                .map(|c| {
                    crate::automation::has_explicit_rule(&c.automation, &EventType::ProblemCreated)
                })
                .unwrap_or(false);
            if !has_rules {
                crate::sync::hooks::auto_create_issue(ctx, &mut problem);
            }

            let event = Event::new(EventType::ProblemCreated, pid.clone(), user.clone());
            crate::automation::run(ctx, &event, &pid);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn list_problems(
    ctx: &CommandContext,
    status_filter: Option<String>,
    tree: bool,
    milestone_filter: Option<String>,
    search_query: Option<&str>,
    assignee_filter: Option<String>,
    tag_filter: Option<String>,
    sort: &str,
    json: bool,
) -> Result<()> {
    let store = &ctx.store;

    let mut problems = store.list_problems()?;

    // Filter by status
    if let Some(status_str) = status_filter {
        let status: ProblemStatus = status_str
            .parse()
            .map_err(|e: String| crate::error::JjjError::Validation(e))?;
        problems.retain(|p| p.status == status);
    } else {
        // Default: show only active problems (open and in_progress), like a kanban board
        problems.retain(|p| matches!(p.status, ProblemStatus::Open | ProblemStatus::InProgress));
    }

    // Filter by milestone (entity resolution: UUID, prefix, or title)
    if let Some(ref ms_input) = milestone_filter {
        let resolved = ctx.resolve_milestone(ms_input)?;
        problems.retain(|p| p.milestone_id.as_deref() == Some(resolved.as_str()));
    }

    // Filter by assignee (substring match)
    if let Some(ref assignee_pattern) = assignee_filter {
        let pattern = assignee_pattern.to_lowercase();
        problems.retain(|p| {
            p.assignee
                .as_deref()
                .map(|a| a.to_lowercase().contains(&pattern))
                .unwrap_or(false)
        });
    }

    // Filter by tag (case-insensitive exact match)
    if let Some(ref tag_pattern) = tag_filter {
        let pattern = tag_pattern.to_lowercase();
        problems.retain(|p| p.tags.iter().any(|t| t.to_lowercase() == pattern));
    }

    // Filter by search query using FTS (auto-populate DB if needed)
    if let Some(query) = search_query {
        let jj_client = ctx.jj();
        let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
        let db = Database::open(&db_path)?;
        crate::db::load_from_markdown(&db, &ctx.store)?;
        let results = search::search(db.conn(), query, Some("problem"))?;
        let matching_ids: std::collections::HashSet<_> =
            results.iter().map(|r| r.entity_id.as_str()).collect();
        problems.retain(|p| matching_ids.contains(p.id.as_str()));
    }

    // Sort
    match sort {
        "priority" => problems.sort_by(|a, b| b.priority.cmp(&a.priority)),
        "status" => problems.sort_by(|a, b| a.status.cmp(&b.status)),
        "created" => problems.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "title" => problems.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        _ => {} // default: no additional sort (UUID7 order)
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&problems)?);
        return Ok(());
    }

    if tree {
        // Show as tree structure
        let root_problems: Vec<_> = problems.iter().filter(|p| p.parent_id.is_none()).collect();
        for problem in root_problems {
            print_problem_tree(store, problem, 0)?;
        }
    } else {
        // Show as flat list
        if problems.is_empty() {
            println!("No problems found.");
            return Ok(());
        }

        // Calculate truncated prefixes
        let uuids: Vec<&str> = problems.iter().map(|p| p.id.as_str()).collect();
        let prefixes = truncated_prefixes(&uuids);

        println!("{:<10} {:<12} {:<10} TITLE", "ID", "STATUS", "PRIORITY");
        println!("{}", "-".repeat(70));

        for (problem, (_, prefix)) in problems.iter().zip(prefixes.iter()) {
            let status_icon = match problem.status {
                ProblemStatus::Open => " ",
                ProblemStatus::InProgress => ">",
                ProblemStatus::Solved => "+",
                ProblemStatus::Dissolved => "~",
            };
            println!(
                "{:<10} {}{:<11} {:<10} {}",
                prefix, status_icon, problem.status, problem.priority, problem.title
            );
        }
    }

    Ok(())
}

fn print_problem_tree(store: &MetadataStore, problem: &Problem, depth: usize) -> Result<()> {
    let indent = "  ".repeat(depth);
    let status_icon = match problem.status {
        ProblemStatus::Open => "[ ]",
        ProblemStatus::InProgress => "[>]",
        ProblemStatus::Solved => "[+]",
        ProblemStatus::Dissolved => "[~]",
    };

    // Get solution count
    let solutions = store.list_solutions_for_problem(&problem.id)?;
    let solution_info = if solutions.is_empty() {
        String::new()
    } else {
        let accepted = solutions
            .iter()
            .filter(|s| s.status == crate::models::SolutionStatus::Approved)
            .count();
        format!(" ({} solutions, {} approved)", solutions.len(), accepted)
    };

    println!(
        "{}{} {} {}{}",
        indent, status_icon, problem.id, problem.title, solution_info
    );

    // Recursively print children
    let children = store.list_subproblems(&problem.id)?;
    for child in &children {
        print_problem_tree(store, child, depth + 1)?;
    }

    Ok(())
}

fn show_problem(ctx: &CommandContext, problem_input: String, json: bool) -> Result<()> {
    let store = &ctx.store;

    let problem_id = ctx.resolve_problem(&problem_input)?;
    let problem = store.load_problem(&problem_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&problem)?);
        return Ok(());
    }

    println!("Problem: {} - {}", problem.id, problem.title);
    println!("Status: {}", problem.status);
    println!("Priority: {}", problem.priority);

    if let Some(ref parent) = problem.parent_id {
        println!("Parent: {}", parent);
    }

    if let Some(ref milestone) = problem.milestone_id {
        println!("Milestone: {}", milestone);
    }

    if let Some(ref assignee) = problem.assignee {
        println!("Assignee: {}", assignee);
    }

    if !problem.tags.is_empty() {
        println!("Tags: {}", problem.tags.join(", "));
    }

    // Show description
    if !problem.description.is_empty() {
        println!("\n## Description\n{}", problem.description);
    }

    // Show context
    if !problem.context.is_empty() {
        println!("\n## Context\n{}", problem.context);
    }

    // Show dissolved reason
    if let Some(ref reason) = problem.dissolved_reason {
        println!("\n## Dissolved Reason\n{}", reason);
    }

    // Show solutions
    let solutions = store.list_solutions_for_problem(&problem_id)?;
    if !solutions.is_empty() {
        println!("\n## Solutions ({})", solutions.len());
        for solution in &solutions {
            let status_icon = match solution.status {
                crate::models::SolutionStatus::Proposed => " ",
                crate::models::SolutionStatus::Submitted => ">",
                crate::models::SolutionStatus::Approved => "+",
                crate::models::SolutionStatus::Withdrawn => "x",
            };
            println!(
                "  {} {} - {} [{}]",
                status_icon, solution.id, solution.title, solution.status
            );
        }
    }

    // Show subproblems
    let subproblems = store.list_subproblems(&problem_id)?;
    if !subproblems.is_empty() {
        println!("\n## Sub-problems ({})", subproblems.len());
        for sub in &subproblems {
            println!("  {} - {} [{}]", sub.id, sub.title, sub.status);
        }
    }

    println!("\nCreated: {}", problem.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", problem.updated_at.format("%Y-%m-%d %H:%M"));

    show_related_items(ctx, "problem", &problem.id)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn edit_problem(
    ctx: &CommandContext,
    problem_input: String,
    title: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    parent: Option<String>,
    add_tag: Option<String>,
    remove_tag: Option<String>,
    set_tags: Option<Vec<String>>,
) -> Result<()> {
    let store = &ctx.store;

    let problem_id = ctx.resolve_problem(&problem_input)?;

    // Resolve parent if provided and not empty
    let resolved_parent = if let Some(ref parent_input) = parent {
        if parent_input.is_empty() {
            Some(String::new()) // Empty means clear parent
        } else {
            Some(ctx.resolve_problem(parent_input)?)
        }
    } else {
        None
    };

    store.with_metadata(&format!("Edit problem {}", problem_id), || {
        let mut problem = store.load_problem(&problem_id)?;

        if let Some(new_title) = title {
            problem.title = new_title;
        }

        if let Some(status_str) = status {
            let new_status: ProblemStatus = status_str
                .parse()
                .map_err(|e: String| crate::error::JjjError::Validation(e))?;

            // Guard: solved requires at least one approved solution
            if new_status == ProblemStatus::Solved {
                let solutions = store.list_solutions_for_problem(&problem_id)?;
                let has_approved = solutions.iter().any(|s| {
                    s.status == crate::models::SolutionStatus::Approved
                });
                if !has_approved {
                    return Err(crate::error::JjjError::Validation(
                        "Cannot mark as solved: no approved solution. Use 'jjj solution approve' first, or 'jjj problem dissolve' if the problem is no longer relevant.".to_string(),
                    ));
                }
            }

            let valid = valid_transitions_for_problem(&problem.status);
            let cur_status_str = problem.status.to_string();
            problem.try_set_status(new_status)
                .map_err(|e| crate::error::JjjError::Validation(format!(
                    "{}. Valid transitions from {}: {}",
                    e, cur_status_str, valid
                )))?;
        }

        if let Some(p_str) = priority {
            problem.priority = p_str
                .parse::<Priority>()
                .map_err(|e: String| crate::error::JjjError::Validation(e))?;
        }

        if let Some(ref new_parent) = resolved_parent {
            problem.set_parent(if new_parent.is_empty() {
                None
            } else {
                Some(new_parent.clone())
            });
        }

        if let Some(ref tags) = set_tags {
            let mut t: Vec<String> = tags.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            t.sort();
            t.dedup();
            problem.tags = t;
        }

        if let Some(ref tag) = add_tag {
            let tag = tag.trim().to_string();
            if !tag.is_empty() && !problem.tags.iter().any(|t| t.to_lowercase() == tag.to_lowercase()) {
                problem.tags.push(tag);
                problem.tags.sort();
            }
        }

        if let Some(ref tag) = remove_tag {
            let tag_lower = tag.trim().to_lowercase();
            problem.tags.retain(|t| t.to_lowercase() != tag_lower);
        }

        store.save_problem(&problem)?;
        println!("Updated problem {}", problem_id);
        Ok(())
    })
}

fn show_tree(ctx: &CommandContext, problem_input: Option<String>) -> Result<()> {
    let store = &ctx.store;

    if let Some(ref input) = problem_input {
        let problem_id = ctx.resolve_problem(input)?;
        let problem = store.load_problem(&problem_id)?;
        print_problem_tree(store, &problem, 0)?;
    } else {
        let root_problems = store.list_root_problems()?;
        if root_problems.is_empty() {
            println!("No problems found.");
            return Ok(());
        }
        for problem in &root_problems {
            print_problem_tree(store, problem, 0)?;
        }
    }

    Ok(())
}

fn solve_problem(ctx: &CommandContext, problem_input: String, github_close: bool) -> Result<()> {
    let store = &ctx.store;

    let problem_id = ctx.resolve_problem(&problem_input)?;

    // Check if can be solved
    let (can_solve, message) = store.can_solve_problem(&problem_id)?;

    if !can_solve {
        // Already-solved is idempotent: still honour --github-close.
        if message == "Problem is already solved" {
            println!("Problem {} is already marked as solved.", problem_id);
            if let Ok(problem) = store.load_problem(&problem_id) {
                crate::sync::hooks::auto_close_issue(ctx, &problem, github_close);
            }
            return Ok(());
        }
        return Err(crate::error::JjjError::CannotSolveProblem(message));
    }

    if !message.is_empty() {
        println!("{}", message);
    }

    // Warn about active solutions still in progress
    let solutions = store.list_solutions()?;
    let active: Vec<_> = solutions
        .iter()
        .filter(|s| s.problem_id == problem_id && s.is_active())
        .collect();
    if !active.is_empty() {
        eprintln!(
            "Warning: {} active solution(s) still in progress:",
            active.len()
        );
        for s in &active {
            eprintln!("  {}: {} [{}]", s.id, s.title, s.status);
        }
        eprintln!("Proceeding with solve anyway.");
    }

    // Create event for decision log
    let user = store.get_current_user()?;
    let event = Event::new(EventType::ProblemSolved, problem_id.clone(), user.clone());

    store.with_metadata(&format!("Solve problem {}", problem_id), || {
        store.set_pending_event(event.clone());
        let mut problem = store.load_problem(&problem_id)?;
        problem.set_status(ProblemStatus::Solved);
        store.save_problem(&problem)?;
        println!("Problem {} marked as solved.", problem_id);
        Ok(())
    })?;

    // Auto-close GitHub issue if explicitly requested or configured
    if let Ok(problem) = ctx.store.load_problem(&problem_id) {
        let has_rules = ctx
            .store
            .load_config()
            .ok()
            .map(|c| crate::automation::has_explicit_rule(&c.automation, &EventType::ProblemSolved))
            .unwrap_or(false);
        if !has_rules {
            crate::sync::hooks::auto_close_issue(ctx, &problem, github_close);
        }

        let event = Event::new(EventType::ProblemSolved, problem_id.clone(), user);
        crate::automation::run(ctx, &event, &problem_id);
    }

    Ok(())
}

fn dissolve_problem(
    ctx: &CommandContext,
    problem_input: String,
    reason: Option<String>,
    github_close: bool,
) -> Result<()> {
    let store = &ctx.store;

    let problem_id = ctx.resolve_problem(&problem_input)?;

    // Create event for decision log
    let user = store.get_current_user()?;
    let mut event = Event::new(
        EventType::ProblemDissolved,
        problem_id.clone(),
        user.clone(),
    );
    if let Some(ref r) = reason {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Dissolve problem {}", problem_id), || {
        store.set_pending_event(event.clone());
        let mut problem = store.load_problem(&problem_id)?;
        if let Some(reason) = reason.clone() {
            problem.dissolve(reason);
        } else {
            problem.set_status(ProblemStatus::Dissolved);
        }
        store.save_problem(&problem)?;
        println!(
            "Problem {} marked as dissolved (based on false premises or became irrelevant).",
            problem_id
        );
        Ok(())
    })?;

    // Auto-close GitHub issue if explicitly requested or configured
    if let Ok(problem) = ctx.store.load_problem(&problem_id) {
        let has_rules = ctx
            .store
            .load_config()
            .ok()
            .map(|c| {
                crate::automation::has_explicit_rule(&c.automation, &EventType::ProblemDissolved)
            })
            .unwrap_or(false);
        if !has_rules {
            crate::sync::hooks::auto_close_issue(ctx, &problem, github_close);
        }

        let event = Event::new(
            EventType::ProblemDissolved,
            problem_id.clone(),
            user.clone(),
        );
        crate::automation::run(ctx, &event, &problem_id);
    }

    Ok(())
}

fn assign_problem(
    ctx: &CommandContext,
    problem_input: String,
    assignee: Option<String>,
) -> Result<()> {
    let store = &ctx.store;

    let problem_id = ctx.resolve_problem(&problem_input)?;

    let assignee_name = match assignee {
        Some(name) => name,
        None => store.jj_client.user_identity()?,
    };

    store.with_metadata(
        &format!("Assign problem {} to {}", problem_id, assignee_name),
        || {
            let mut problem = store.load_problem(&problem_id)?;
            problem.assignee = Some(assignee_name.clone());
            store.save_problem(&problem)?;
            println!("Problem {} assigned to {}", problem_id, assignee_name);
            Ok(())
        },
    )
}

fn reopen_problem(ctx: &CommandContext, problem_input: String) -> Result<()> {
    let store = &ctx.store;
    let problem_id = ctx.resolve_problem(&problem_input)?;

    let problem = store.load_problem(&problem_id)?;
    if !matches!(
        problem.status,
        ProblemStatus::Solved | ProblemStatus::Dissolved
    ) {
        return Err(crate::error::JjjError::Validation(format!(
            "Problem '{}' is {} — only solved or dissolved problems can be reopened.",
            problem.title, problem.status
        )));
    }

    let user = store.get_current_user()?;
    let event = Event::new(EventType::ProblemReopened, problem_id.clone(), user);

    store.with_metadata(&format!("Reopen problem {}", problem_id), || {
        store.set_pending_event(event.clone());
        let mut problem = store.load_problem(&problem_id)?;
        problem.set_status(ProblemStatus::Open);
        store.save_problem(&problem)?;
        println!("Problem '{}' reopened.", problem.title);
        Ok(())
    })
}

fn duplicate_problem(ctx: &CommandContext, input: String, canonical_input: String) -> Result<()> {
    let store = &ctx.store;
    let dup_id = ctx.resolve_problem(&input)?;
    let canonical_id = ctx.resolve_problem(&canonical_input)?;

    if dup_id == canonical_id {
        return Err(crate::error::JjjError::Validation(
            "A problem cannot be a duplicate of itself.".to_string(),
        ));
    }

    let canonical = store.load_problem(&canonical_id)?;
    let reason = format!("Duplicate of '{}'", canonical.title);

    let user = store.get_current_user()?;
    let event = Event::new(EventType::ProblemDissolved, dup_id.clone(), user)
        .with_rationale(reason.clone());

    store.with_metadata(
        &format!("Mark problem {} as duplicate of {}", dup_id, canonical_id),
        || {
            store.set_pending_event(event.clone());
            let mut problem = store.load_problem(&dup_id)?;
            if problem.is_resolved() {
                return Err(crate::error::JjjError::Validation(format!(
                    "Problem '{}' is already {} — cannot mark as duplicate.",
                    problem.title, problem.status
                )));
            }
            problem.dissolve(reason.clone());
            store.save_problem(&problem)?;
            println!(
                "Problem '{}' dissolved as duplicate of '{}'.",
                problem.title, canonical.title
            );
            Ok(())
        },
    )
}

fn check_for_duplicates(
    ctx: &CommandContext,
    title: &str,
) -> Result<Option<Vec<search::SimilarityResult>>> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    if !db_path.exists() {
        return Ok(None);
    }

    let local_config = LocalConfig::load(repo_root);
    if !local_config.duplicate_check_enabled() {
        return Ok(None);
    }

    let client = match EmbeddingClient::from_config(&local_config, false) {
        Some(c) => c,
        None => return Ok(None),
    };

    let db = Database::open(&db_path)?;
    let conn = db.conn();

    // Embed the title
    let embedding = match client.embed(title) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    // Find similar problems
    let threshold = local_config.duplicate_threshold();
    let results = search::similarity_search(conn, &embedding, Some("problem"), None, 5)?;
    let similar: Vec<_> = results
        .into_iter()
        .filter(|r| r.similarity >= threshold)
        .collect();

    if similar.is_empty() {
        Ok(None)
    } else {
        Ok(Some(similar))
    }
}

fn graph_problems(
    ctx: &CommandContext,
    milestone_filter: Option<String>,
    show_all: bool,
) -> Result<()> {
    let store = &ctx.store;

    // Resolve milestone ID if provided
    let milestone_id = if let Some(ref m) = milestone_filter {
        Some(ctx.resolve_milestone(m)?)
    } else {
        None
    };

    let mut problems = store.list_problems()?;

    // Filter by status unless --all
    if !show_all {
        problems
            .retain(|p| p.status == ProblemStatus::Open || p.status == ProblemStatus::InProgress);
    }

    // Filter by milestone
    if let Some(ref mid) = milestone_id {
        problems.retain(|p| p.milestone_id.as_deref() == Some(mid.as_str()));
    }

    if problems.is_empty() {
        println!("No problems found.");
        return Ok(());
    }

    // Build set of IDs in filtered set
    let id_set: std::collections::HashSet<&str> = problems.iter().map(|p| p.id.as_str()).collect();

    // Find roots: problems whose parent is None or not in the filtered set
    let roots: Vec<&Problem> = problems
        .iter()
        .filter(|p| {
            p.parent_id
                .as_deref()
                .map(|pid| !id_set.contains(pid))
                .unwrap_or(true)
        })
        .collect();

    // Build children map: parent_id -> children
    let mut children_map: std::collections::HashMap<&str, Vec<&Problem>> =
        std::collections::HashMap::new();
    for p in &problems {
        if let Some(ref parent_id) = p.parent_id {
            if id_set.contains(parent_id.as_str()) {
                children_map.entry(parent_id.as_str()).or_default().push(p);
            }
        }
    }

    for root in &roots {
        print_graph_node(root, &children_map, &[], true);
    }

    Ok(())
}

fn print_graph_node(
    problem: &Problem,
    children_map: &std::collections::HashMap<&str, Vec<&Problem>>,
    prefix_stack: &[bool], // true = last child at that level
    is_last: bool,
) {
    let icon = match problem.status {
        ProblemStatus::Open | ProblemStatus::InProgress => "○",
        ProblemStatus::Solved => "◉",
        ProblemStatus::Dissolved => "×",
    };

    // Build the line prefix from the stack
    let mut prefix = String::new();
    for &last in prefix_stack {
        if last {
            prefix.push_str("   ");
        } else {
            prefix.push_str("│  ");
        }
    }

    if prefix_stack.is_empty() {
        // Root node — no connector
        println!("{} {} [{}]", icon, problem.title, problem.status);
    } else if is_last {
        println!(
            "{}└─ {} {} [{}]",
            prefix, icon, problem.title, problem.status
        );
    } else {
        println!(
            "{}├─ {} {} [{}]",
            prefix, icon, problem.title, problem.status
        );
    }

    let children = children_map
        .get(problem.id.as_str())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let mut new_stack = prefix_stack.to_vec();
    new_stack.push(is_last);
    for (i, child) in children.iter().enumerate() {
        let child_is_last = i == children.len() - 1;
        print_graph_node(child, children_map, &new_stack, child_is_last);
    }
}

fn valid_transitions_for_problem(status: &ProblemStatus) -> &'static str {
    match status {
        ProblemStatus::Open => "in_progress, solved, dissolved",
        ProblemStatus::InProgress => "open, solved, dissolved",
        ProblemStatus::Solved => "open",
        ProblemStatus::Dissolved => "open",
    }
}

fn prompt_create_anyway(similar: &[search::SimilarityResult]) -> Result<bool> {
    use std::io::{self, Write};

    println!("\nSimilar existing problems found:\n");
    for result in similar {
        let short_id = &result.entity_id[..6.min(result.entity_id.len())];
        println!(
            "  p/{}  [{:.2}]  \"{}\"",
            short_id, result.similarity, result.title
        );
    }
    println!();

    print!("Create anyway? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}
