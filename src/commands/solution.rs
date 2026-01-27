use crate::cli::SolutionAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, ProblemStatus, Solution, SolutionStatus};
use crate::storage::MetadataStore;
use std::io::{self, Write};

pub fn execute(action: SolutionAction) -> Result<()> {
    match action {
        SolutionAction::New { title, problem, supersedes, review } => new_solution(title, problem, supersedes, review),
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
        } => edit_solution(solution_id, title, status),
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
        SolutionAction::Lgtm { solution_id, comment } => lgtm_solution(solution_id, comment),
        SolutionAction::Resume { solution_id } => resume_solution(solution_id),
    }
}

fn new_solution(title: String, problem_id: String, supersedes: Option<String>, reviewers: Vec<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Validate problem exists
    let _problem = store.load_problem(&problem_id)?;

    store.with_metadata(&format!("Create solution: {}", title), || {
        let solution_id = store.next_solution_id()?;
        let mut solution = Solution::new(solution_id.clone(), title.clone(), problem_id.clone());

        // Set supersedes
        solution.supersedes = supersedes.clone();

        // Add reviewers
        for reviewer in &reviewers {
            let name = reviewer.trim_start_matches('@').to_string();
            solution.add_reviewer(name);
        }

        store.save_solution(&solution)?;

        // Update problem's solution_ids
        let mut problem = store.load_problem(&problem_id)?;
        problem.add_solution(solution_id.clone());
        store.save_problem(&problem)?;

        println!("Created solution {} ({})", solution.id, solution.title);
        println!("  Addresses: {} - {}", problem.id, problem.title);
        if let Some(ref sup) = solution.supersedes {
            println!("  Supersedes: {}", sup);
        }
        if !solution.reviewers.is_empty() {
            println!("  Reviewers: {}", solution.reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));
        }

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

    // Show reviewers and sign-offs
    if !solution.reviewers.is_empty() {
        println!("\n## Reviewers");
        for reviewer in &solution.reviewers {
            if let Some(so) = solution.sign_offs.iter().find(|so| &so.reviewer == reviewer) {
                let comment_str = so.comment.as_ref().map(|c| format!(" — {}", c)).unwrap_or_default();
                println!("  @{}: signed off {}{}", reviewer, so.at.format("%Y-%m-%d"), comment_str);
            } else {
                println!("  @{}: pending", reviewer);
            }
        }
        // Show non-reviewer sign-offs
        let non_reviewer_sign_offs: Vec<_> = solution.sign_offs.iter()
            .filter(|so| !solution.reviewers.contains(&so.reviewer))
            .collect();
        if !non_reviewer_sign_offs.is_empty() {
            println!("  Also endorsed by: {}", non_reviewer_sign_offs.iter().map(|so| format!("@{}", so.reviewer)).collect::<Vec<_>>().join(", "));
        }
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
    let requires_review = solution.requires_review();
    if requires_review {
        if !solution.all_reviewers_signed_off() {
            if !force {
                eprintln!("Error: Solution requires sign-off from all reviewers.");
                eprintln!("Reviewers: {}", solution.reviewers.join(", "));
                let signed: Vec<_> = solution.sign_offs.iter().map(|so| so.reviewer.as_str()).collect();
                eprintln!("Signed off: {}", if signed.is_empty() { "none".to_string() } else { signed.join(", ") });
                eprintln!("Or force:  jjj solution accept {} --force", solution_id);
                return Err(crate::error::JjjError::CannotAcceptSolution("Not all reviewers signed off".to_string()));
            }
            eprintln!("Warning: Accepting without full reviewer sign-off.");
        }
    }

    store.with_metadata(&format!("Accept solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        if force {
            solution.force_accepted = true;
        }
        solution.accept();
        store.save_solution(&solution)?;

        let status = if force && (!open_critiques.is_empty() || requires_review) {
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
            solution.add_reviewer(reviewer.clone());
        }

        store.save_solution(&solution)?;

        println!("Review requested for solution {}", solution_id);
        println!("  reviewers: {}", reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));

        Ok(())
    })
}

fn resume_solution(solution_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

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

fn lgtm_solution(solution_id: String, comment: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("LGTM solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        let user = store.jj_client.user_identity()?;

        // Check if user is a requested reviewer
        if !solution.reviewers.iter().any(|r| user.contains(r)) {
            println!("Warning: You were not a requested reviewer for this solution.");
            println!("Only sign-offs from requested reviewers count toward acceptance.");
        }

        // Find matching reviewer name (if any) to ensure sign-off matches exactly
        let reviewer_name = solution.reviewers.iter()
            .find(|r| user.contains(r.as_str()))
            .cloned()
            .unwrap_or_else(|| user.clone());

        let added = solution.add_sign_off(reviewer_name, comment);
        store.save_solution(&solution)?;

        if added {
            println!("Sign-off recorded for solution {}", solution_id);
        } else {
            println!("You have already signed off on solution {}", solution_id);
        }

        Ok(())
    })
}
