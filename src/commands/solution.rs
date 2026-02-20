use crate::cli::SolutionAction;
use crate::context::CommandContext;
use crate::db::{search, Database};
use crate::display::truncated_prefixes;
use crate::embeddings::EmbeddingClient;
use crate::error::Result;
use crate::local_config::LocalConfig;
use crate::models::{
    Critique, CritiqueSeverity, CritiqueStatus, Event, EventExtra, EventType, ProblemStatus,
    Solution, SolutionStatus,
};
use std::io::{self, Write};

pub fn execute(ctx: &CommandContext, action: SolutionAction) -> Result<()> {
    match action {
        SolutionAction::New {
            title,
            problem,
            supersedes,
            reviewer,
            force,
        } => new_solution(ctx, title, problem, supersedes, reviewer, force),

        SolutionAction::List {
            problem,
            status,
            search,
            sort,
            json,
        } => list_solutions(ctx, problem, status, search.as_deref(), &sort, json),
        SolutionAction::Show { solution_id, json } => show_solution(ctx, solution_id, json),
        SolutionAction::Edit {
            solution_id,
            title,
            status,
        } => edit_solution(ctx, solution_id, title, status),
        SolutionAction::Attach { solution_id, force } => attach_change(ctx, solution_id, force),
        SolutionAction::Detach {
            solution_id,
            change_id,
            force,
        } => detach_change(ctx, solution_id, change_id, force),
        SolutionAction::Test { solution_id } => test_solution(ctx, solution_id),
        SolutionAction::Accept {
            solution_id,
            force,
            rationale,
            no_rationale,
        } => accept_solution(ctx, solution_id, force, rationale, no_rationale),
        SolutionAction::Refute {
            solution_id,
            rationale,
            no_rationale,
        } => refute_solution(ctx, solution_id, rationale, no_rationale),
        SolutionAction::Assign { solution_id, to } => assign_solution(ctx, solution_id, to),
        SolutionAction::Resume { solution_id } => resume_solution(ctx, solution_id),
    }
}

fn new_solution(
    ctx: &CommandContext,
    title: String,
    problem_input: Option<String>,
    supersedes_input: Option<String>,
    reviewer_critiques: Vec<String>,
    force: bool,
) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    // Validate title is not empty
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(crate::error::JjjError::Validation(
            "Title cannot be empty.".to_string(),
        ));
    }

    // If not forcing, check for duplicates
    if !force {
        // Check for similar solutions via FTS text search (best-effort, skip on error)
        let repo_root = jj_client.repo_root().to_path_buf();
        let db_path = repo_root.join(".jj").join("jjj.db");
        if db_path.exists() {
            if let Ok(db) = Database::open(&db_path) {
                if let Ok(results) = search::search(db.conn(), &title, Some("solution")) {
                    if !results.is_empty() {
                        eprintln!("Warning: similar solutions already exist:");
                        for r in &results {
                            let short_id = &r.entity_id[..6.min(r.entity_id.len())];
                            eprintln!("  s/{} — \"{}\"", short_id, r.title);
                        }
                        eprintln!("\nUse --force to create anyway.");
                        return Err(crate::error::JjjError::Other(
                            "Similar entities exist. Use --force to override.".to_string(),
                        ));
                    }
                }
            }
        }

        // Also check semantic duplicates via embeddings (if available)
        if let Some(similar) = check_for_similar_solutions(ctx, &title)? {
            if !prompt_create_solution_anyway(&similar)? {
                println!("Cancelled.");
                return Ok(());
            }
        }
    }

    // Resolve problem ID: use provided value or prompt interactively
    let problem_id = match problem_input {
        Some(ref input) => ctx.resolve_problem(input)?,
        None => {
            // List open problems for interactive selection
            let problems = store.list_problems()?;
            let open_problems: Vec<_> = problems.into_iter().filter(|p| p.is_open()).collect();

            if open_problems.is_empty() {
                return Err(crate::error::JjjError::Validation("No open problems found. Create a problem first with: jjj problem new \"title\"".to_string()));
            }

            println!("Select a problem to address:\n");
            for (i, p) in open_problems.iter().enumerate() {
                println!("  {}. {} - {} [{}]", i + 1, p.id, p.title, p.priority);
            }
            print!("\nChoice [1-{}]: ", open_problems.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice: usize = input
                .trim()
                .parse()
                .map_err(|_| crate::error::JjjError::Validation("Invalid choice".to_string()))?;

            if choice < 1 || choice > open_problems.len() {
                return Err(crate::error::JjjError::Validation(
                    "Invalid selection".to_string(),
                ));
            }

            open_problems[choice - 1].id.clone()
        }
    };

    // Validate problem exists
    let _problem = store.load_problem(&problem_id)?;

    // Resolve supersedes if provided
    let supersedes = match supersedes_input {
        Some(ref input) => Some(ctx.resolve_solution(input)?),
        None => None,
    };

    // Get user for event
    let user = store.get_current_user()?;

    store.with_metadata(&format!("Start solution: {}", title), || {
        let solution_id = store.next_solution_id()?;
        let mut solution = Solution::new(solution_id.clone(), title.clone(), problem_id.clone());

        // Set supersedes
        solution.supersedes = supersedes.clone();

        // Create event for decision log
        let extra = EventExtra {
            problem: Some(problem_id.clone()),
            supersedes: supersedes.clone(),
            ..Default::default()
        };
        let event = Event::new(
            EventType::SolutionCreated,
            solution_id.clone(),
            user.clone(),
        )
        .with_extra(extra);
        store.set_pending_event(event);

        // Auto-attach: create jj change and link to solution
        jj_client.new_empty_change(&title)?;
        let change_id = jj_client.current_change_id()?;
        solution.attach_change(change_id);
        solution.start_testing();

        store.save_solution(&solution)?;

        // Auto-set jj change description
        let problem = store.load_problem(&solution.problem_id)?;
        let description = format!(
            "{}: {}\n\nProblem: {} - {}",
            solution.id, solution.title, problem.id, problem.title
        );
        if let Err(e) = store.jj_client.describe(&description) {
            eprintln!("Warning: Could not set jj description: {}", e);
        }

        // Create awaiting review critiques for each reviewer (from --reviewer flag)
        for reviewer_spec in &reviewer_critiques {
            let (reviewer_name, severity) = parse_reviewer_spec(reviewer_spec);
            let critique_id = store.next_critique_id()?;
            let mut critique = Critique::new(
                critique_id.clone(),
                format!("Awaiting review from @{}", reviewer_name),
                solution.id.clone(),
            );
            critique.reviewer = Some(reviewer_name.clone());
            critique.severity = severity;
            critique.author = solution.assignee.clone();
            store.save_critique(&critique)?;
            solution.critique_ids.push(critique_id);
        }

        // Re-save solution with critique IDs if we added any
        if !reviewer_critiques.is_empty() {
            store.save_solution(&solution)?;
        }

        // Update problem
        let mut problem = store.load_problem(&problem_id)?;
        problem.add_solution(solution_id.clone());
        if problem.status == ProblemStatus::Open {
            problem.set_status(ProblemStatus::InProgress);
        }
        store.save_problem(&problem)?;

        println!("Created solution {} ({})", solution.id, solution.title);
        println!("  Addresses: {} - {}", problem.id, problem.title);
        if let Some(ref sup) = solution.supersedes {
            println!("  Supersedes: {}", sup);
        }
        if !reviewer_critiques.is_empty() {
            let names: Vec<_> = reviewer_critiques
                .iter()
                .map(|s| format!("@{}", parse_reviewer_spec(s).0))
                .collect();
            println!("  Awaiting review: {}", names.join(", "));
        }

        Ok(())
    })
}

/// Parse a reviewer specification like "@bob" or "bob:high" into (name, severity)
fn parse_reviewer_spec(spec: &str) -> (String, CritiqueSeverity) {
    let spec = spec.trim_start_matches('@');
    if let Some((name, severity_str)) = spec.split_once(':') {
        let severity = severity_str.parse().unwrap_or(CritiqueSeverity::Low);
        (name.to_string(), severity)
    } else {
        (spec.to_string(), CritiqueSeverity::Low)
    }
}

fn list_solutions(
    ctx: &CommandContext,
    problem_filter: Option<String>,
    status_filter: Option<String>,
    search_query: Option<&str>,
    sort: &str,
    json: bool,
) -> Result<()> {
    let store = &ctx.store;

    let mut solutions = store.list_solutions()?;

    // Filter by problem (resolve the input first)
    if let Some(ref problem_input) = problem_filter {
        let problem_id = ctx.resolve_problem(problem_input)?;
        solutions.retain(|s| s.problem_id == problem_id);
    }

    // Filter by status
    if let Some(status_str) = status_filter {
        let status: SolutionStatus = status_str
            .parse()
            .map_err(|e: String| crate::error::JjjError::Validation(e))?;
        solutions.retain(|s| s.status == status);
    }

    // Filter by search query using FTS
    if let Some(query) = search_query {
        let jj_client = ctx.jj();
        let db_path = jj_client.repo_root().join(".jj").join("jjj.db");

        if db_path.exists() {
            let db = Database::open(&db_path)?;
            let results = search::search(db.conn(), query, Some("solution"))?;
            let matching_ids: std::collections::HashSet<_> =
                results.iter().map(|r| r.entity_id.as_str()).collect();
            solutions.retain(|s| matching_ids.contains(s.id.as_str()));
        }
    }

    // Sort
    match sort {
        "status" => solutions.sort_by(|a, b| a.status.cmp(&b.status)),
        "created" => solutions.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "title" => solutions.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        _ => {} // default: no additional sort (UUID7 order)
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&solutions)?);
        return Ok(());
    }

    if solutions.is_empty() {
        println!("No solutions found.");
        return Ok(());
    }

    // Calculate truncated prefixes for solutions
    let solution_uuids: Vec<&str> = solutions.iter().map(|s| s.id.as_str()).collect();
    let solution_prefixes = truncated_prefixes(&solution_uuids);

    // Calculate truncated prefixes for problems (for display)
    let problem_uuids: Vec<&str> = solutions.iter().map(|s| s.problem_id.as_str()).collect();
    let problem_prefixes = truncated_prefixes(&problem_uuids);

    println!("{:<10} {:<12} {:<10} TITLE", "ID", "STATUS", "PROBLEM");
    println!("{}", "-".repeat(70));

    for ((solution, (_, sol_prefix)), (_, prob_prefix)) in solutions
        .iter()
        .zip(solution_prefixes.iter())
        .zip(problem_prefixes.iter())
    {
        let status_icon = match solution.status {
            SolutionStatus::Proposed => " ",
            SolutionStatus::Testing => ">",
            SolutionStatus::Accepted => "+",
            SolutionStatus::Refuted => "x",
        };

        println!(
            "{:<10} {}{:<11} {:<10} {}",
            sol_prefix, status_icon, solution.status, prob_prefix, solution.title
        );
    }

    Ok(())
}

fn show_solution(ctx: &CommandContext, solution_input: String, json: bool) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    let solution = store.load_solution(&solution_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&solution)?);
        return Ok(());
    }

    println!("Solution: {} - {}", solution.id, solution.title);
    println!("Status: {}", solution.status);
    if solution.force_accepted {
        println!("Force accepted: yes");
    }
    println!("Addresses: {}", solution.problem_id);
    if let Some(ref sup) = solution.supersedes {
        println!("Supersedes: {}", sup);
    }

    if let Some(ref assignee) = solution.assignee {
        println!("Assignee: {}", assignee);
    }

    // Show attached changes
    if !solution.change_ids.is_empty() {
        println!("\n## Changes ({})", solution.change_ids.len());
        for change_id in &solution.change_ids {
            println!("  {}", change_id);
        }
    }

    // Show approach
    if !solution.approach.is_empty() {
        println!("\n## Approach\n{}", solution.approach);
    }

    // Show trade-offs
    if !solution.tradeoffs.is_empty() {
        println!("\n## Trade-offs\n{}", solution.tradeoffs);
    }

    // Show critiques
    let critiques = store.get_critiques_for_solution(&solution_id)?;
    if !critiques.is_empty() {
        println!("\n## Critiques ({})", critiques.len());
        for critique in &critiques {
            let status_icon = match critique.status {
                crate::models::CritiqueStatus::Open => "?",
                crate::models::CritiqueStatus::Addressed => "+",
                crate::models::CritiqueStatus::Valid => "!",
                crate::models::CritiqueStatus::Dismissed => "-",
            };
            println!(
                "  {} {} - {} [{}, {}]",
                status_icon, critique.id, critique.title, critique.status, critique.severity
            );
        }
    }

    println!(
        "\nCreated: {}",
        solution.created_at.format("%Y-%m-%d %H:%M")
    );
    println!("Updated: {}", solution.updated_at.format("%Y-%m-%d %H:%M"));

    crate::commands::show_related_items(ctx, "solution", &solution.id)?;

    Ok(())
}

fn edit_solution(
    ctx: &CommandContext,
    solution_input: String,
    title: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    store.with_metadata(&format!("Edit solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        if let Some(new_title) = title {
            solution.title = new_title;
        }

        if let Some(status_str) = status {
            let new_status: SolutionStatus = status_str
                .parse()
                .map_err(|e: String| crate::error::JjjError::Validation(e))?;
            if !solution.can_transition_to(&new_status) {
                return Err(crate::error::JjjError::Validation(format!(
                    "Invalid status transition: {} -> {}",
                    solution.status, new_status
                )));
            }
            solution.set_status(new_status);
        }

        store.save_solution(&solution)?;
        println!("Updated solution {}", solution_id);
        Ok(())
    })
}

fn attach_change(ctx: &CommandContext, solution_input: String, force: bool) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let change_id = jj_client.current_change_id()?;

    // Validate change exists in jj
    if !force {
        if !jj_client.change_exists(&change_id)? {
            return Err(crate::error::JjjError::Other(format!(
                "Change '{}' not found in repository. Use --force to skip validation.",
                change_id
            )));
        }

        // Check no other solution already has this change attached
        let all_solutions = store.list_solutions()?;
        for other in &all_solutions {
            if other.id != solution_id && other.change_ids.contains(&change_id) {
                return Err(crate::error::JjjError::Other(format!(
                    "Change '{}' is already attached to solution {}. Use --force to attach anyway.",
                    change_id, other.id
                )));
            }
        }
    }

    store.with_metadata(
        &format!("Attach change {} to solution {}", change_id, solution_id),
        || {
            let mut solution = store.load_solution(&solution_id)?;
            solution.attach_change(change_id.clone());
            store.save_solution(&solution)?;
            println!("Attached change {} to solution {}", change_id, solution_id);
            Ok(())
        },
    )
}

fn detach_change(
    ctx: &CommandContext,
    solution_input: String,
    change_id: Option<String>,
    force: bool,
) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let change_id = match change_id {
        Some(id) => id,
        None => jj_client.current_change_id()?,
    };

    // Safety checks
    if !force {
        let solution = store.load_solution(&solution_id)?;

        // Block detach from Testing solutions
        if solution.status == SolutionStatus::Testing {
            return Err(crate::error::JjjError::Other(format!(
                "Cannot detach change from solution {} while in Testing state. Use --force to override.",
                solution_id
            )));
        }

        // Block detach of last change
        if solution.change_ids.len() <= 1 && solution.change_ids.contains(&change_id) {
            return Err(crate::error::JjjError::Other(format!(
                "Cannot detach the last change from solution {}. Use --force to override.",
                solution_id
            )));
        }
    }

    store.with_metadata(
        &format!("Detach change {} from solution {}", change_id, solution_id),
        || {
            let mut solution = store.load_solution(&solution_id)?;

            if solution.detach_change(&change_id) {
                store.save_solution(&solution)?;
                println!(
                    "Detached change {} from solution {} ({} change(s) remaining)",
                    change_id,
                    solution_id,
                    solution.change_ids.len()
                );
            } else {
                println!(
                    "Change {} was not attached to solution {}",
                    change_id, solution_id
                );
            }
            Ok(())
        },
    )
}

fn test_solution(ctx: &CommandContext, solution_input: String) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    store.with_metadata(&format!("Move solution {} to testing", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        solution.start_testing();
        store.save_solution(&solution)?;

        // Update problem status to in_progress if it's still open
        let mut problem = store.load_problem(&solution.problem_id)?;
        if problem.status == ProblemStatus::Open {
            problem.set_status(ProblemStatus::InProgress);
            store.save_problem(&problem)?;
            println!("Problem {} moved to in_progress", problem.id);
        }

        println!("Solution {} moved to testing", solution_id);
        Ok(())
    })
}

fn accept_solution(
    ctx: &CommandContext,
    solution_input: String,
    force: bool,
    rationale: Option<String>,
    no_rationale: bool,
) -> Result<()> {
    use crate::models::{Event, EventType};

    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    // Reject double-accept
    {
        let solution = store.load_solution(&solution_id)?;
        if solution.status == SolutionStatus::Accepted {
            return Err(crate::error::JjjError::Validation(format!(
                "Solution '{}' is already accepted.",
                solution.title
            )));
        }
    }

    let critiques = store.list_critiques()?;

    // Find open critiques for this solution
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| c.solution_id == solution_id && c.status == CritiqueStatus::Open)
        .collect();

    // Check critique blocking
    if !open_critiques.is_empty() {
        if !force {
            eprintln!(
                "Error: Cannot accept {} - {} open critique(s):\n",
                solution_id,
                open_critiques.len()
            );
            for c in &open_critiques {
                let location = c
                    .file_path
                    .as_ref()
                    .map(|f| format!(" - {}:{}", f, c.line_start.unwrap_or(0)))
                    .unwrap_or_default();
                eprintln!("  {}: {} [{}]{}", c.id, c.title, c.severity, location);
            }
            eprintln!();
            eprintln!(
                "Resolve with: jjj critique address {}",
                open_critiques[0].id
            );
            eprintln!(
                "Or dismiss:   jjj critique dismiss {}",
                open_critiques[0].id
            );
            eprintln!("Or force:     jjj solution accept {} --force", solution_id);
            return Err(crate::error::JjjError::CannotAcceptSolution(
                "Open critiques block acceptance".to_string(),
            ));
        }
        eprintln!(
            "Warning: Accepting with {} open critique(s):",
            open_critiques.len()
        );
        for c in &open_critiques {
            eprintln!("  {}: {} [{}]", c.id, c.title, c.severity);
        }
    }

    // Get rationale (prompt if not provided and not skipped)
    let rationale = if let Some(r) = rationale {
        Some(r)
    } else if no_rationale {
        None
    } else {
        print!("Rationale (optional, press Enter to skip): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    // Create event
    let user = store.get_current_user()?;
    let mut event = Event::new(EventType::SolutionAccepted, solution_id.clone(), user);
    if let Some(r) = &rationale {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Accept solution {}", solution_id), || {
        store.set_pending_event(event.clone());

        let mut solution = store.load_solution(&solution_id)?;
        if force {
            solution.force_accepted = true;
        }
        solution.accept();
        store.save_solution(&solution)?;

        let status = if force && !open_critiques.is_empty() {
            "accepted (forced)"
        } else {
            "accepted"
        };
        println!("Solution {} {}", solution_id, status);

        // Auto-solve the parent problem if all solutions are resolved
        let (can_solve, _) = store.can_solve_problem(&solution.problem_id)?;
        if can_solve {
            let mut problem = store.load_problem(&solution.problem_id)?;
            if problem.status != ProblemStatus::Solved {
                problem.set_status(ProblemStatus::Solved);
                store.save_problem(&problem)?;
                println!(
                    "Problem {} auto-solved (accepted solution)",
                    solution.problem_id
                );
            }
        }

        Ok(())
    })?;

    // Auto-merge GitHub PR if enabled
    if let Ok(solution) = ctx.store.load_solution(&solution_id) {
        crate::sync::hooks::auto_merge_pr(ctx, &solution);
    }

    Ok(())
}

fn refute_solution(
    ctx: &CommandContext,
    solution_input: String,
    rationale: Option<String>,
    no_rationale: bool,
) -> Result<()> {
    use crate::models::{Event, EventType};

    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    // Get rationale (prompt if not provided and not skipped)
    let rationale = if let Some(r) = rationale {
        Some(r)
    } else if no_rationale {
        None
    } else {
        print!("Rationale (optional, press Enter to skip): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    // Create event
    let user = store.get_current_user()?;
    let mut event = Event::new(EventType::SolutionRefuted, solution_id.clone(), user);
    if let Some(r) = &rationale {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Refute solution {}", solution_id), || {
        store.set_pending_event(event.clone());

        let mut solution = store.load_solution(&solution_id)?;
        solution.refute();
        store.save_solution(&solution)?;
        println!(
            "Solution {} refuted (criticism showed it won't work)",
            solution_id
        );
        Ok(())
    })
}

fn assign_solution(
    ctx: &CommandContext,
    solution_input: String,
    assignee: Option<String>,
) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    let assignee_name = match assignee {
        Some(name) => name,
        None => store.jj_client.user_identity()?,
    };

    store.with_metadata(
        &format!("Assign solution {} to {}", solution_id, assignee_name),
        || {
            let mut solution = store.load_solution(&solution_id)?;
            solution.assignee = Some(assignee_name.clone());
            store.save_solution(&solution)?;
            println!("Solution {} assigned to {}", solution_id, assignee_name);
            Ok(())
        },
    )
}

fn resume_solution(ctx: &CommandContext, solution_input: String) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let solution = store.load_solution(&solution_id)?;
    println!("Resuming solution {} ({})", solution.id, solution.title);

    // Check if solution has an active change attached
    if let Some(change_id) = solution.change_ids.last() {
        println!("Switching to change {}", change_id);
        jj_client.edit(change_id)?;
    } else {
        println!("No active change for solution. Creating new change.");

        store.with_metadata(&format!("Resume solution: {}", solution.title), || {
            jj_client.new_empty_change(&solution.title)?;
            let change_id = jj_client.current_change_id()?;

            let mut solution = store.load_solution(&solution_id)?;
            solution.attach_change(change_id);
            solution.start_testing();
            store.save_solution(&solution)?;

            // Update problem status
            let mut problem = store.load_problem(&solution.problem_id)?;
            if problem.status == ProblemStatus::Open {
                problem.set_status(ProblemStatus::InProgress);
                store.save_problem(&problem)?;
            }

            Ok(())
        })?;
    }

    Ok(())
}

fn check_for_similar_solutions(
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

    // Find similar solutions
    let threshold = local_config.duplicate_threshold();
    let results = search::similarity_search(conn, &embedding, Some("solution"), None, 5)?;
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

fn prompt_create_solution_anyway(similar: &[search::SimilarityResult]) -> Result<bool> {
    println!("\nSimilar existing solutions found:\n");
    for result in similar {
        let short_id = &result.entity_id[..6.min(result.entity_id.len())];
        println!(
            "  s/{}  [{:.2}]  \"{}\"",
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
