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
            assignee,
            search,
            sort,
            json,
        } => list_solutions(ctx, problem, status, assignee, search.as_deref(), &sort, json),
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
        SolutionAction::Submit { solution_id } => submit_solution(ctx, solution_id),
        SolutionAction::Withdraw {
            solution_id,
            rationale,
            no_rationale,
        } => withdraw_solution(ctx, solution_id, rationale, no_rationale),
        SolutionAction::Approve { solution_id, force, rationale, no_rationale } => {
            approve_solution(ctx, solution_id, force, rationale, no_rationale)
        }
        SolutionAction::Assign { solution_id, to } => assign_solution(ctx, solution_id, to),
        SolutionAction::Resume { solution_id } => resume_solution(ctx, solution_id),
        SolutionAction::Lgtm { solution_id } => lgtm_solution(ctx, solution_id),
        SolutionAction::Comment { solution_id, critique, body } => {
            comment_solution(ctx, solution_id, critique, body)
        }
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
                        return Err(crate::error::JjjError::Validation(
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
    assignee_filter: Option<String>,
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

    // Filter by assignee (substring match)
    if let Some(ref assignee_pattern) = assignee_filter {
        let pattern = assignee_pattern.trim_start_matches('@').to_lowercase();
        solutions.retain(|s| {
            s.assignee
                .as_deref()
                .map(|a| a.to_lowercase().contains(&pattern))
                .unwrap_or(false)
        });
    }

    // Filter by search query using FTS (auto-populate DB if needed)
    if let Some(query) = search_query {
        let jj_client = ctx.jj();
        let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
        let db = Database::open(&db_path)?;
        crate::db::load_from_markdown(&db, &ctx.store)?;
        let results = search::search(db.conn(), query, Some("solution"))?;
        let matching_ids: std::collections::HashSet<_> =
            results.iter().map(|r| r.entity_id.as_str()).collect();
        solutions.retain(|s| matching_ids.contains(s.id.as_str()));
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
            SolutionStatus::Submitted => ">",
            SolutionStatus::Approved => "+",
            SolutionStatus::Withdrawn => "x",
        };

        println!(
            "{:<10} {}{:<11} {:<10} {}",
            sol_prefix, status_icon, solution.status, prob_prefix, solution.title
        );
    }

    Ok(())
}

fn show_solution(ctx: &CommandContext, solution_input: String, json: bool) -> Result<()> {
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    

    let solution = store.load_solution(&solution_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&solution)?);
        return Ok(());
    }

    println!("Solution: {} - {}", solution.id, solution.title);
    println!("Status: {}", solution.status);
    if solution.force_approved {
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
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    

    store.with_metadata(&format!("Edit solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        if let Some(new_title) = title {
            solution.title = new_title;
        }

        if let Some(status_str) = status {
            let new_status: SolutionStatus = status_str
                .parse()
                .map_err(|e: String| crate::error::JjjError::Validation(e))?;
            solution.try_set_status(new_status)
                .map_err(|e| crate::error::JjjError::Validation(e))?;
        }

        store.save_solution(&solution)?;
        println!("Updated solution {}", solution_id);
        Ok(())
    })
}

fn attach_change(ctx: &CommandContext, solution_input: String, force: bool) -> Result<()> {
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    
    let jj_client = ctx.jj();

    let change_id = jj_client.current_change_id()?;

    // Validate change exists in jj
    if !force {
        if !jj_client.change_exists(&change_id)? {
            return Err(crate::error::JjjError::Validation(format!(
                "Change '{}' not found in repository. Use --force to skip validation.",
                change_id
            )));
        }

        // Check no other solution already has this change attached
        let all_solutions = store.list_solutions()?;
        for other in &all_solutions {
            if other.id != solution_id && other.change_ids.contains(&change_id) {
                return Err(crate::error::JjjError::Validation(format!(
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
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    
    let jj_client = ctx.jj();

    let change_id = match change_id {
        Some(id) => id,
        None => jj_client.current_change_id()?,
    };

    // Safety checks
    if !force {
        let solution = store.load_solution(&solution_id)?;

        // Block detach from Review solutions
        if solution.status == SolutionStatus::Submitted {
            return Err(crate::error::JjjError::Validation(format!(
                "Cannot detach change from solution {} while in Submitted state. Use --force to override.",
                solution_id
            )));
        }

        // Block detach of last change
        if solution.change_ids.len() <= 1 && solution.change_ids.contains(&change_id) {
            return Err(crate::error::JjjError::Validation(format!(
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

fn submit_solution(ctx: &CommandContext, solution_input: String) -> Result<()> {
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    
    let user = store.get_current_user().unwrap_or_default();

    store.with_metadata(&format!("Submit solution {} for review", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        solution.submit();
        store.save_solution(&solution)?;

        let event = Event::new(EventType::SolutionSubmitted, solution_id.clone(), user.clone())
            .with_extra(EventExtra {
                problem: Some(solution.problem_id.clone()),
                ..Default::default()
            });
        store.set_pending_event(event);

        // Update problem status to in_progress if it's still open
        let mut problem = store.load_problem(&solution.problem_id)?;
        if problem.status == ProblemStatus::Open {
            problem.set_status(ProblemStatus::InProgress);
            store.save_problem(&problem)?;
            println!("Problem {} moved to in_progress", problem.id);
        }

        println!("Solution {} submitted for review", solution_id);
        Ok(())
    })
}

fn approve_solution(
    ctx: &CommandContext,
    solution_input: Option<String>,
    force: bool,
    rationale: Option<String>,
    no_rationale: bool,
) -> Result<()> {
    use crate::sync::SyncProvider as _;

    let store = &ctx.store;
    let jj_client = ctx.jj();

    let solution = if let Some(input) = solution_input {
        let id = ctx.resolve_solution(&input)?;
        store.load_solution(&id)?
    } else {
        let change_id = jj_client.current_change_id()?;
        let solutions = store.list_solutions()?;
        match solutions.into_iter().find(|s| s.change_ids.contains(&change_id)) {
            Some(s) => s,
            None => {
                return Err(crate::error::JjjError::Validation(
                    "No solution found for current change. Specify a solution: jjj solution approve <title-or-id>".to_string(),
                ));
            }
        }
    };

    if solution.status == SolutionStatus::Approved {
        return Err(crate::error::JjjError::Validation(format!(
            "Solution '{}' is already approved.",
            solution.title,
        )));
    }

    if solution.status == SolutionStatus::Proposed {
        return Err(crate::error::JjjError::Validation(format!(
            "Solution '{}' is proposed — submit it for review first:\n  jjj solution submit {}",
            solution.title, solution.id,
        )));
    }

    // Finalize (critique check, events, auto-solve).
    let rationale_str = rationale.as_deref().filter(|_| !no_rationale);
    finalize_solution(ctx, &solution.id, force, rationale_str)?;
    println!("Solution '{}' approved.", solution.title);

    // Merge PR if one is linked.
    if let Some(pr_number) = solution.github_pr {
        let config = store.load_config()?;
        let repo_root = jj_client.repo_root();
        let provider = crate::sync::github::GitHubProvider::from_config(repo_root, &config.github)?;
        provider.merge_pr(pr_number)?;
        println!("  Merged PR #{}", pr_number);
    }

    Ok(())
}


/// Core acceptance logic shared by `submit` and `github merge`.
/// Checks critiques, validates state, emits events, accepts, and auto-solves.
/// Does NOT squash code or merge PRs — that is the caller's responsibility.

pub(crate) fn finalize_solution(
    ctx: &CommandContext,
    solution_id: &str,
    force: bool,
    rationale: Option<&str>,
) -> Result<()> {
    let store = &ctx.store;


    let solution = store.load_solution(solution_id)?;

    // Already accepted — idempotent
    if solution.status == SolutionStatus::Approved {
        return Ok(());
    }

    // Solution must be in Review to submit (or force)
    if solution.status != SolutionStatus::Submitted && !force {
        return Err(crate::error::JjjError::Validation(format!(
            "Solution '{}' is {} — move it to review first:\n  jjj solution submit {}",
            solution.title, solution.status, solution_id,
        )));
    }

    // Check open critiques (before state — critiques are the most actionable blocker)
    let critiques = store.list_critiques()?;
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| {
            c.solution_id == solution_id
                && (c.status == CritiqueStatus::Open || c.status == CritiqueStatus::Valid)
        })
        .collect();

    if !open_critiques.is_empty() {
        if !force {
            eprintln!("\n  {} open critique(s):", open_critiques.len());
            for c in &open_critiques {
                let loc = c
                    .file_path
                    .as_ref()
                    .map(|f| format!(" {}:{}", f, c.line_start.unwrap_or(0)))
                    .unwrap_or_default();
                eprintln!("    [{:}] {}{}", c.severity, c.title, loc);
                eprintln!("       jjj critique address {}", c.id);
            }
            eprintln!("\nAddress critiques before submitting, or use --force to override.");
            return Err(crate::error::JjjError::CannotAcceptSolution(
                "Unresolved critiques block submission".to_string(),
            ));
        }
        eprintln!(
            "Warning: submitting with {} open critique(s).",
            open_critiques.len()
        );
    }

    let user = store.get_current_user()?;
    let mut event = Event::new(EventType::SolutionApproved, solution_id.to_string(), user);
    if let Some(r) = rationale {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Approve solution {}", solution_id), || {
        store.set_pending_event(event.clone());
        let mut sol = store.load_solution(solution_id)?;
        if force {
            sol.force_approved = true;
        }
        sol.approve();
        store.save_solution(&sol)?;

        // Auto-solve the parent problem
        let (can_solve, _) = store.can_solve_problem(&sol.problem_id)?;
        if can_solve {
            let mut problem = store.load_problem(&sol.problem_id)?;
            if problem.status != ProblemStatus::Solved {
                problem.set_status(ProblemStatus::Solved);
                store.save_problem(&problem)?;
                let solve_event = Event::new(
                    EventType::ProblemSolved,
                    problem.id.clone(),
                    event.by.clone(),
                );
                store.set_pending_event(solve_event);
                println!("  Problem '{}' solved.", problem.title);
            }
        }

        Ok(())
    })
}

fn withdraw_solution(
    ctx: &CommandContext,
    solution_input: String,
    rationale: Option<String>,
    no_rationale: bool,
) -> Result<()> {
    let store = &ctx.store;
    use crate::models::{Event, EventType};

    let solution_id = ctx.resolve_solution(&solution_input)?;
    

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
    let mut event = Event::new(EventType::SolutionWithdrawn, solution_id.clone(), user);
    if let Some(r) = &rationale {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Withdraw solution {}", solution_id), || {
        store.set_pending_event(event.clone());

        let mut solution = store.load_solution(&solution_id)?;
        solution.withdraw();
        store.save_solution(&solution)?;
        println!(
            "Solution {} withdrawn",
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
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    

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
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    
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
            solution.submit();
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

fn lgtm_solution(ctx: &CommandContext, solution_input: String) -> Result<()> {
    let store = &ctx.store;
    let solution_id = ctx.resolve_solution(&solution_input)?;
    
    let current_user = store.get_current_user()?;

    let solution = store.load_solution(&solution_id)?;
    let critiques = store.get_critiques_for_solution(&solution_id)?;

    // Find an open review critique assigned to (or matching) the current user
    let my_review = critiques.iter().find(|c| {
        c.status == CritiqueStatus::Open
            && c.reviewer.as_ref().map_or(false, |r| {
                r.contains(&current_user) || current_user.contains(r.as_str())
            })
    });

    let critique = match my_review {
        Some(c) => c,
        None => {
            // Check if there are any open review critiques at all (assigned to others)
            let any_review = critiques.iter().any(|c| {
                c.status == CritiqueStatus::Open && c.reviewer.is_some()
            });
            if any_review {
                return Err(crate::error::JjjError::Validation(format!(
                    "No open review critique assigned to you on '{}'.\n\
                     (There are review critiques assigned to others — are you the right reviewer?)\n\n\
                     To add yourself: jjj critique new \"{}\" \"Review\" --reviewer @{}",
                    solution.title, solution_input, current_user
                )));
            } else {
                return Err(crate::error::JjjError::Validation(format!(
                    "No review critique assigned to you on '{}'.\n\n\
                     To request review from yourself: jjj critique new \"{}\" \"Review\" --reviewer @{}\n\
                     Or use solution new --reviewer @{} when creating solutions.",
                    solution.title, solution_input, current_user, current_user
                )));
            }
        }
    };

    let critique_id = critique.id.clone();
    store.with_metadata(&format!("LGTM on solution {}", solution_id), || {
        let mut c = store.load_critique(&critique_id)?;
        c.address();
        store.save_critique(&c)?;
        println!("Signed off on '{}' as @{}", solution.title, current_user);

        // Check if this was the last blocking item
        let remaining = store
            .get_critiques_for_solution(&solution_id)?
            .into_iter()
            .filter(|c| {
                c.status == CritiqueStatus::Open || c.status == CritiqueStatus::Valid
            })
            .count();

        if remaining == 0 {
            println!("All critiques resolved. Ready to accept:");
            println!("  jjj solution approve \"{}\"", solution.title);
        } else {
            println!("{} critique(s) still open.", remaining);
        }

        Ok(())
    })
}

fn comment_solution(
    ctx: &CommandContext,
    solution_input: Option<String>,
    critique_input: Option<String>,
    body: Option<String>,
) -> Result<()> {
    let store = &ctx.store;
    

    // Resolve solution — use explicit input or fall back to the change attached to @
    let solution_id = if let Some(ref input) = solution_input {
        ctx.resolve_solution(input)?
    } else {
        // Try active change first
        let change_id = ctx
            .jj()
            .execute(&["log", "-r", "@", "-T", "change_id", "--no-graph"])
            .unwrap_or_default();
        let change_id = change_id.trim();

        let solutions = store.list_solutions()?;
        let by_change = solutions
            .iter()
            .find(|s| s.change_ids.iter().any(|c| c == change_id));

        if let Some(s) = by_change {
            s.id.clone()
        } else {
            // Fall back to any active solution
            let active: Vec<_> = solutions.iter().filter(|s| s.is_active()).collect();
            match active.len() {
                0 => {
                    return Err(crate::error::JjjError::Validation(
                        "No active solution found. Specify a solution ID.".to_string(),
                    ))
                }
                1 => active[0].id.clone(),
                _ => {
                    return Err(crate::error::JjjError::Validation(format!(
                        "Multiple active solutions. Specify one:\n{}",
                        active
                            .iter()
                            .map(|s| format!("  jjj solution comment \"{}\"", s.title))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )))
                }
            }
        }
    };

    // Get open critiques for the solution
    let critiques: Vec<_> = store
        .get_critiques_for_solution(&solution_id)?
        .into_iter()
        .filter(|c| c.status == CritiqueStatus::Open || c.status == CritiqueStatus::Valid)
        .collect();

    if critiques.is_empty() {
        return Err(crate::error::JjjError::Validation(
            "No open critiques on this solution to reply to.".to_string(),
        ));
    }

    // Resolve critique — explicit, single, or interactive picker
    let critique_id = if let Some(ref input) = critique_input {
        // Resolve by prefix/title within this solution's critiques
        let resolved = ctx.resolve_critique(input)?;
        // Verify it belongs to this solution
        if !critiques.iter().any(|c| c.id == resolved) {
            return Err(crate::error::JjjError::Validation(format!(
                "Critique '{}' is not an open critique for this solution.",
                input
            )));
        }
        resolved
    } else if critiques.len() == 1 {
        critiques[0].id.clone()
    } else {
        // Interactive picker
        println!("Open critiques:");
        for (i, c) in critiques.iter().enumerate() {
            println!("  [{}] {} [{}]", i + 1, c.title, c.severity);
        }
        print!("Select [1-{}]: ", critiques.len());
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let idx: usize = line.trim().parse().unwrap_or(0);
        critiques
            .get(idx.saturating_sub(1))
            .filter(|_| idx > 0)
            .ok_or_else(|| {
                crate::error::JjjError::Validation("Invalid selection.".to_string())
            })?
            .id
            .clone()
    };

    // Get reply body — positional arg or prompt
    let reply_body = if let Some(b) = body {
        b
    } else {
        print!("Reply: ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let b = line.trim().to_string();
        if b.is_empty() {
            return Err(crate::error::JjjError::Validation(
                "Reply cannot be empty.".to_string(),
            ));
        }
        b
    };

    let user = store.get_current_user()?;
    store.with_metadata(&format!("Reply to critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        critique.add_reply(user.clone(), reply_body.clone());
        store.save_critique(&critique)?;

        let event = Event::new(EventType::CritiqueReplied, critique_id.clone(), user.clone())
            .with_extra(EventExtra {
                target: Some(solution_id.clone()),
                ..Default::default()
            });
        store.set_pending_event(event);

        println!("Replied to critique '{}'.", critique.title);
        Ok(())
    })
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
