use crate::cli::SolutionAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, ProblemStatus, Solution, SolutionStatus};
use crate::storage::MetadataStore;
use std::io::{self, Write};

pub fn execute(action: SolutionAction) -> Result<()> {
    match action {
        SolutionAction::New { title, problem, supersedes: _, tag } => new_solution(title, problem, tag),
        SolutionAction::List {
            problem,
            status,
            json,
        } => list_solutions(problem, status, json),
        SolutionAction::Show { solution_id, json } => show_solution(solution_id, json),
        SolutionAction::Edit {
            solution_id,
            title,
            status,
            add_tag,
            remove_tag,
        } => edit_solution(solution_id, title, status, add_tag, remove_tag),
        SolutionAction::Attach { solution_id } => attach_change(solution_id),
        SolutionAction::Detach {
            solution_id,
            change_id,
        } => detach_change(solution_id, change_id),
        SolutionAction::Test { solution_id } => test_solution(solution_id),
        SolutionAction::Accept { solution_id, force } => accept_solution(solution_id, force),
        SolutionAction::Refute { solution_id } => refute_solution(solution_id),
        SolutionAction::Assign { solution_id, to } => assign_solution(solution_id, to),
        SolutionAction::Review { solution_id, reviewers } => request_review(solution_id, reviewers),
        SolutionAction::Lgtm { solution_id } => lgtm_solution(solution_id),
    }
}

fn new_solution(title: String, problem_id: String, tags: Vec<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Validate problem exists
    let _problem = store.load_problem(&problem_id)?;

    store.with_metadata(&format!("Create solution: {}", title), || {
        let solution_id = store.next_solution_id()?;
        let mut solution = Solution::new(solution_id.clone(), title.clone(), problem_id.clone());

        // Add tags
        for tag in tags {
            solution.add_tag(tag);
        }

        store.save_solution(&solution)?;

        // Update problem's solution_ids
        let mut problem = store.load_problem(&problem_id)?;
        problem.add_solution(solution_id.clone());
        store.save_problem(&problem)?;

        println!("Created solution {} ({})", solution.id, solution.title);
        println!("  Addresses: {} - {}", problem.id, problem.title);

        Ok(())
    })
}

fn list_solutions(
    problem_filter: Option<String>,
    status_filter: Option<String>,
    json: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

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
        "{:<8} {:<12} {:<10} {}",
        "ID", "STATUS", "PROBLEM", "TITLE"
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

fn show_solution(solution_id: String, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let solution = store.load_solution(&solution_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&solution)?);
        return Ok(());
    }

    println!("Solution: {} - {}", solution.id, solution.title);
    println!("Status: {}", solution.status);
    println!("Addresses: {}", solution.problem_id);

    if let Some(ref assignee) = solution.assignee {
        println!("Assignee: {}", assignee);
    }

    if !solution.tags.is_empty() {
        println!(
            "Tags: {}",
            solution.tags.iter().cloned().collect::<Vec<_>>().join(", ")
        );
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
    solution_id: String,
    title: Option<String>,
    status: Option<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Edit solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        if let Some(new_title) = title {
            solution.title = new_title;
        }

        if let Some(status_str) = status {
            let new_status: SolutionStatus = status_str.parse().map_err(|e: String| e)?;
            solution.set_status(new_status);
        }

        for tag in add_tags {
            solution.add_tag(tag);
        }

        for tag in remove_tags {
            solution.remove_tag(&tag);
        }

        store.save_solution(&solution)?;
        println!("Updated solution {}", solution_id);
        Ok(())
    })
}

fn attach_change(solution_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    let change_id = jj_client.current_change_id()?;

    store.with_metadata(&format!("Attach change {} to solution {}", change_id, solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        solution.attach_change(change_id.clone());
        store.save_solution(&solution)?;
        println!("Attached change {} to solution {}", change_id, solution_id);
        Ok(())
    })
}

fn detach_change(solution_id: String, change_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

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

fn test_solution(solution_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

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

fn accept_solution(solution_id: String, force: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;
    let config = store.load_config()?;

    let solution = store.load_solution(&solution_id)?;
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

    // Check review requirement
    let requires_review = solution.requires_review.unwrap_or(config.review.default_required);
    if requires_review {
        if solution.requested_reviewers.is_empty() {
            if !force {
                eprintln!("Error: Solution requires review but no reviewers requested.");
                eprintln!("Request review: jjj solution review {} @reviewer", solution_id);
                eprintln!("Or force:       jjj solution accept {} --force", solution_id);
                return Err(crate::error::JjjError::CannotAcceptSolution("No reviewers requested".to_string()));
            }
            eprintln!("Warning: Accepting without requested reviewers.");
        } else if !solution.has_lgtm_from_requested_reviewer() {
            if !force {
                eprintln!("Error: Solution requires LGTM from a requested reviewer.");
                eprintln!("Requested: {}", solution.requested_reviewers.join(", "));
                eprintln!("LGTM'd:    {}", if solution.reviewed_by.is_empty() { "none".to_string() } else { solution.reviewed_by.join(", ") });
                eprintln!("Or force:  jjj solution accept {} --force", solution_id);
                return Err(crate::error::JjjError::CannotAcceptSolution("No LGTM from requested reviewer".to_string()));
            }
            eprintln!("Warning: Accepting without LGTM from requested reviewer.");
        }
    }

    store.with_metadata(&format!("Accept solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
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

fn refute_solution(solution_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Refute solution {}", solution_id), || {
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

fn assign_solution(solution_id: String, assignee: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

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

fn request_review(solution_id: String, reviewers: Vec<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let reviewers: Vec<String> = reviewers.iter().map(|r| r.trim_start_matches('@').to_string()).collect();

    store.with_metadata(&format!("Request review on {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        for reviewer in &reviewers {
            solution.request_review(reviewer.clone());
        }

        store.save_solution(&solution)?;

        println!("✓ Review requested for solution {}", solution_id);
        println!("  Reviewers: {}", reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));

        Ok(())
    })
}

fn lgtm_solution(solution_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("LGTM solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        let user = store.jj_client.user_identity()?;

        // Check if user is a requested reviewer
        if !solution.requested_reviewers.iter().any(|r| user.contains(r)) {
            println!("Warning: You were not a requested reviewer for this solution.");
            println!("Only LGTMs from requested reviewers count toward acceptance.");
        }

        solution.add_lgtm(user.clone());
        store.save_solution(&solution)?;

        println!("✓ LGTM recorded for solution {}", solution_id);

        Ok(())
    })
}
