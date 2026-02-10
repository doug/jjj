use crate::cli::SolutionAction;
use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{Critique, CritiqueSeverity, CritiqueStatus, ProblemStatus, Solution, SolutionStatus};
use std::io::{self, Write};

pub fn execute(ctx: &CommandContext, action: SolutionAction) -> Result<()> {
    match action {
        SolutionAction::New { title, problem, supersedes, reviewer } => new_solution(ctx, title, problem, supersedes, reviewer),

        SolutionAction::List {
            problem,
            status,
            json,
        } => list_solutions(ctx, problem, status, json),
        SolutionAction::Show { solution_id, json } => show_solution(ctx, solution_id, json),
        SolutionAction::Edit {
            solution_id,
            title,
            status,
        } => edit_solution(ctx, solution_id, title, status),
        SolutionAction::Attach { solution_id } => attach_change(ctx, solution_id),
        SolutionAction::Detach {
            solution_id,
            change_id,
        } => detach_change(ctx, solution_id, change_id),
        SolutionAction::Test { solution_id } => test_solution(ctx, solution_id),
        SolutionAction::Accept { solution_id, force, rationale, no_rationale } => accept_solution(ctx, solution_id, force, rationale, no_rationale),
        SolutionAction::Refute { solution_id, rationale, no_rationale } => refute_solution(ctx, solution_id, rationale, no_rationale),
        SolutionAction::Assign { solution_id, to } => assign_solution(ctx, solution_id, to),
        SolutionAction::Resume { solution_id } => resume_solution(ctx, solution_id),
    }
}

fn new_solution(ctx: &CommandContext, title: String, problem_id: Option<String>, supersedes: Option<String>, reviewer_critiques: Vec<String>) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    // Resolve problem ID: use provided value or prompt interactively
    let problem_id = match problem_id {
        Some(pid) => pid,
        None => {
            // List open problems for interactive selection
            let problems = store.list_problems()?;
            let open_problems: Vec<_> = problems.into_iter().filter(|p| p.is_open()).collect();

            if open_problems.is_empty() {
                return Err("No open problems found. Create a problem first with: jjj problem new \"title\"".into());
            }

            println!("Select a problem to address:\n");
            for (i, p) in open_problems.iter().enumerate() {
                println!("  {}. {} - {} [{}]", i + 1, p.id, p.title, p.priority);
            }
            print!("\nChoice [1-{}]: ", open_problems.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice: usize = input.trim().parse()
                .map_err(|_| crate::error::JjjError::Other("Invalid choice".to_string()))?;

            if choice < 1 || choice > open_problems.len() {
                return Err("Invalid selection".into());
            }

            open_problems[choice - 1].id.clone()
        }
    };

    // Validate problem exists
    let _problem = store.load_problem(&problem_id)?;

    store.with_metadata(&format!("Start solution: {}", title), || {
        let solution_id = store.next_solution_id()?;
        let mut solution = Solution::new(solution_id.clone(), title.clone(), problem_id.clone());

        // Set supersedes
        solution.supersedes = supersedes.clone();

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
            let names: Vec<_> = reviewer_critiques.iter()
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
    json: bool,
) -> Result<()> {
    let store = &ctx.store;

    let mut solutions = store.list_solutions()?;

    // Filter by problem
    if let Some(ref pid) = problem_filter {
        solutions.retain(|s| &s.problem_id == pid);
    }

    // Filter by status
    if let Some(status_str) = status_filter {
        let status: SolutionStatus = status_str.parse().map_err(|e: String| e)?;
        solutions.retain(|s| s.status == status);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&solutions)?);
        return Ok(());
    }

    if solutions.is_empty() {
        println!("No solutions found.");
        return Ok(());
    }

    println!(
        "{:<8} {:<12} {:<10} TITLE",
        "ID", "STATUS", "PROBLEM",
    );
    println!("{}", "-".repeat(70));

    for solution in &solutions {
        let status_icon = match solution.status {
            SolutionStatus::Proposed => " ",
            SolutionStatus::Testing => ">",
            SolutionStatus::Accepted => "+",
            SolutionStatus::Refuted => "x",
        };

        println!(
            "{:<8} {}{:<11} {:<10} {}",
            solution.id, status_icon, solution.status, solution.problem_id, solution.title
        );
    }

    Ok(())
}

fn show_solution(ctx: &CommandContext, solution_id: String, json: bool) -> Result<()> {
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

    println!("\nCreated: {}", solution.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", solution.updated_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

fn edit_solution(
    ctx: &CommandContext,
    solution_id: String,
    title: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let store = &ctx.store;

    store.with_metadata(&format!("Edit solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        if let Some(new_title) = title {
            solution.title = new_title;
        }

        if let Some(status_str) = status {
            let new_status: SolutionStatus = status_str.parse().map_err(|e: String| e)?;
            solution.set_status(new_status);
        }

        store.save_solution(&solution)?;
        println!("Updated solution {}", solution_id);
        Ok(())
    })
}

fn attach_change(ctx: &CommandContext, solution_id: String) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let change_id = jj_client.current_change_id()?;

    store.with_metadata(&format!("Attach change {} to solution {}", change_id, solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        solution.attach_change(change_id.clone());
        store.save_solution(&solution)?;
        println!("Attached change {} to solution {}", change_id, solution_id);
        Ok(())
    })
}

fn detach_change(ctx: &CommandContext, solution_id: String, change_id: Option<String>) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let change_id = match change_id {
        Some(id) => id,
        None => jj_client.current_change_id()?,
    };

    store.with_metadata(&format!("Detach change {} from solution {}", change_id, solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        if solution.detach_change(&change_id) {
            store.save_solution(&solution)?;
            println!("Detached change {} from solution {}", change_id, solution_id);
        } else {
            println!("Change {} was not attached to solution {}", change_id, solution_id);
        }
        Ok(())
    })
}

fn test_solution(ctx: &CommandContext, solution_id: String) -> Result<()> {
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

fn accept_solution(ctx: &CommandContext, solution_id: String, force: bool, rationale: Option<String>, no_rationale: bool) -> Result<()> {
    use crate::models::{Event, EventType};

    let store = &ctx.store;

    let critiques = store.list_critiques()?;

    // Find open critiques for this solution
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| c.solution_id == solution_id && c.status == CritiqueStatus::Open)
        .collect();

    // Check critique blocking
    if !open_critiques.is_empty() {
        if !force {
            eprintln!("Error: Cannot accept {} - {} open critique(s):\n", solution_id, open_critiques.len());
            for c in &open_critiques {
                let location = c.file_path.as_ref()
                    .map(|f| format!(" - {}:{}", f, c.line_start.unwrap_or(0)))
                    .unwrap_or_default();
                eprintln!("  {}: {} [{}]{}", c.id, c.title, c.severity, location);
            }
            eprintln!();
            eprintln!("Resolve with: jjj critique address {}", open_critiques[0].id);
            eprintln!("Or dismiss:   jjj critique dismiss {}", open_critiques[0].id);
            eprintln!("Or force:     jjj solution accept {} --force", solution_id);
            return Err(crate::error::JjjError::CannotAcceptSolution("Open critiques block acceptance".to_string()));
        }
        eprintln!("Warning: Accepting with {} open critique(s):", open_critiques.len());
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
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
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

        // Check if we should solve the parent problem
        let (can_solve, _) = store.can_solve_problem(&solution.problem_id)?;
        if can_solve {
            print!(
                "Solution accepted. Mark problem {} as solved? [y/N] ",
                solution.problem_id
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                let mut problem = store.load_problem(&solution.problem_id)?;
                problem.set_status(ProblemStatus::Solved);
                store.save_problem(&problem)?;
                println!("Problem {} marked as solved", solution.problem_id);
            }
        }

        Ok(())
    })
}

fn refute_solution(ctx: &CommandContext, solution_id: String, rationale: Option<String>, no_rationale: bool) -> Result<()> {
    use crate::models::{Event, EventType};

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
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
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

fn assign_solution(ctx: &CommandContext, solution_id: String, assignee: Option<String>) -> Result<()> {
    let store = &ctx.store;

    let assignee_name = match assignee {
        Some(name) => name,
        None => store.jj_client.user_identity()?,
    };

    store.with_metadata(&format!("Assign solution {} to {}", solution_id, assignee_name), || {
        let mut solution = store.load_solution(&solution_id)?;
        solution.assignee = Some(assignee_name.clone());
        store.save_solution(&solution)?;
        println!("Solution {} assigned to {}", solution_id, assignee_name);
        Ok(())
    })
}

fn resume_solution(ctx: &CommandContext, solution_id: String) -> Result<()> {
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
