use crate::cli::ProblemAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Problem, ProblemStatus, Priority};
use crate::storage::MetadataStore;

pub fn execute(action: ProblemAction) -> Result<()> {
    match action {
        ProblemAction::New {
            title,
            priority,
            parent,
            milestone,
        } => new_problem(title, priority, parent, milestone),
        ProblemAction::List {
            status,
            tree,
            milestone,
            json,
        } => list_problems(status, tree, milestone, json),
        ProblemAction::Show { problem_id, json } => show_problem(problem_id, json),
        ProblemAction::Edit {
            problem_id,
            title,
            status,
            priority,
            parent,
        } => edit_problem(problem_id, title, status, priority, parent),
        ProblemAction::Tree { problem_id } => show_tree(problem_id),
        ProblemAction::Solve { problem_id } => solve_problem(problem_id),
        ProblemAction::Dissolve { problem_id, reason } => dissolve_problem(problem_id, reason),
        ProblemAction::Assign { problem_id, to } => assign_problem(problem_id, to),
    }
}

fn new_problem(
    title: String,
    priority: String,
    parent: Option<String>,
    milestone: Option<String>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Validate parent if provided
    if let Some(ref parent_id) = parent {
        store.load_problem(parent_id)?;
    }

    // Validate milestone if provided
    if let Some(ref milestone_id) = milestone {
        store.load_milestone(milestone_id)?;
    }

    store.with_metadata(&format!("Create problem: {}", title), || {
        let problem_id = store.next_problem_id()?;
        let mut problem = Problem::new(problem_id.clone(), title.clone());

        // Set priority
        problem.priority = priority.parse::<Priority>()?;

        // Set parent
        if let Some(ref parent_id) = parent {
            problem.set_parent(Some(parent_id.clone()));

            // Update parent's child_ids
            let mut parent_problem = store.load_problem(parent_id)?;
            parent_problem.add_child(problem_id.clone());
            store.save_problem(&parent_problem)?;
        }

        // Set milestone
        if let Some(ref milestone_id) = milestone {
            problem.set_milestone(Some(milestone_id.clone()));

            // Update milestone's problem_ids
            let mut ms = store.load_milestone(milestone_id)?;
            ms.add_problem(problem_id.clone());
            store.save_milestone(&ms)?;
        }

        store.save_problem(&problem)?;

        println!("Created problem {} ({})", problem.id, problem.title);
        if parent.is_some() {
            println!("  Parent: {}", parent.unwrap());
        }
        Ok(())
    })
}

fn list_problems(
    status_filter: Option<String>,
    tree: bool,
    milestone_filter: Option<String>,
    json: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let mut problems = store.list_problems()?;

    // Filter by status
    if let Some(status_str) = status_filter {
        let status: ProblemStatus = status_str.parse().map_err(|e: String| e)?;
        problems.retain(|p| p.status == status);
    }

    // Filter by milestone
    if let Some(ref ms_id) = milestone_filter {
        problems.retain(|p| p.milestone_id.as_deref() == Some(ms_id.as_str()));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&problems)?);
        return Ok(());
    }

    if tree {
        // Show as tree structure
        let root_problems: Vec<_> = problems.iter().filter(|p| p.parent_id.is_none()).collect();
        for problem in root_problems {
            print_problem_tree(&store, problem, 0)?;
        }
    } else {
        // Show as flat list
        if problems.is_empty() {
            println!("No problems found.");
            return Ok(());
        }

        println!("{:<8} {:<12} TITLE", "ID", "STATUS");
        println!("{}", "-".repeat(60));

        for problem in &problems {
            let status_icon = match problem.status {
                ProblemStatus::Open => " ",
                ProblemStatus::InProgress => ">",
                ProblemStatus::Solved => "+",
                ProblemStatus::Dissolved => "~",
            };
            println!(
                "{:<8} {}{:<11} {}",
                problem.id, status_icon, problem.status, problem.title
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
    let solutions = store.get_solutions_for_problem(&problem.id)?;
    let solution_info = if solutions.is_empty() {
        String::new()
    } else {
        let accepted = solutions
            .iter()
            .filter(|s| s.status == crate::models::SolutionStatus::Accepted)
            .count();
        format!(" ({} solutions, {} accepted)", solutions.len(), accepted)
    };

    println!(
        "{}{} {} {}{}",
        indent, status_icon, problem.id, problem.title, solution_info
    );

    // Recursively print children
    let children = store.get_subproblems(&problem.id)?;
    for child in &children {
        print_problem_tree(store, child, depth + 1)?;
    }

    Ok(())
}

fn show_problem(problem_id: String, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

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
    let solutions = store.get_solutions_for_problem(&problem_id)?;
    if !solutions.is_empty() {
        println!("\n## Solutions ({})", solutions.len());
        for solution in &solutions {
            let status_icon = match solution.status {
                crate::models::SolutionStatus::Proposed => " ",
                crate::models::SolutionStatus::Testing => ">",
                crate::models::SolutionStatus::Accepted => "+",
                crate::models::SolutionStatus::Refuted => "x",
            };
            println!("  {} {} - {} [{}]", status_icon, solution.id, solution.title, solution.status);
        }
    }

    // Show subproblems
    let subproblems = store.get_subproblems(&problem_id)?;
    if !subproblems.is_empty() {
        println!("\n## Sub-problems ({})", subproblems.len());
        for sub in &subproblems {
            println!("  {} - {} [{}]", sub.id, sub.title, sub.status);
        }
    }

    println!("\nCreated: {}", problem.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", problem.updated_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

fn edit_problem(
    problem_id: String,
    title: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    parent: Option<String>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Edit problem {}", problem_id), || {
        let mut problem = store.load_problem(&problem_id)?;

        if let Some(new_title) = title {
            problem.title = new_title;
        }

        if let Some(status_str) = status {
            let new_status: ProblemStatus = status_str.parse().map_err(|e: String| e)?;
            problem.set_status(new_status);
        }

        if let Some(p_str) = priority {
            problem.priority = p_str.parse::<Priority>()?;
        }

        if let Some(new_parent) = parent {
            // Validate new parent exists
            if !new_parent.is_empty() {
                store.load_problem(&new_parent)?;
            }
            problem.set_parent(if new_parent.is_empty() {
                None
            } else {
                Some(new_parent)
            });
        }

        store.save_problem(&problem)?;
        println!("Updated problem {}", problem_id);
        Ok(())
    })
}

fn show_tree(problem_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    if let Some(pid) = problem_id {
        let problem = store.load_problem(&pid)?;
        print_problem_tree(&store, &problem, 0)?;
    } else {
        let root_problems = store.get_root_problems()?;
        if root_problems.is_empty() {
            println!("No problems found.");
            return Ok(());
        }
        for problem in &root_problems {
            print_problem_tree(&store, problem, 0)?;
        }
    }

    Ok(())
}

fn solve_problem(problem_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Check if can be solved
    let (can_solve, message) = store.can_solve_problem(&problem_id)?;

    if !can_solve {
        return Err(crate::error::JjjError::CannotSolveProblem(message));
    }

    if !message.is_empty() {
        println!("{}", message);
    }

    // Warn about active solutions still in progress
    let solutions = store.list_solutions()?;
    let active: Vec<_> = solutions.iter()
        .filter(|s| s.problem_id == problem_id && s.is_active())
        .collect();
    if !active.is_empty() {
        eprintln!("Warning: {} active solution(s) still in progress:", active.len());
        for s in &active {
            eprintln!("  {}: {} [{}]", s.id, s.title, s.status);
        }
        eprintln!("Proceeding with solve anyway.");
    }

    store.with_metadata(&format!("Solve problem {}", problem_id), || {
        let mut problem = store.load_problem(&problem_id)?;
        problem.set_status(ProblemStatus::Solved);
        store.save_problem(&problem)?;
        println!("Problem {} marked as solved.", problem_id);
        Ok(())
    })
}

fn dissolve_problem(problem_id: String, reason: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Dissolve problem {}", problem_id), || {
        let mut problem = store.load_problem(&problem_id)?;
        if let Some(reason) = reason {
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
    })
}

fn assign_problem(problem_id: String, assignee: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let assignee_name = match assignee {
        Some(name) => name,
        None => store.jj_client.user_identity()?,
    };

    store.with_metadata(&format!("Assign problem {} to {}", problem_id, assignee_name), || {
        let mut problem = store.load_problem(&problem_id)?;
        problem.assignee = Some(assignee_name.clone());
        store.save_problem(&problem)?;
        println!("Problem {} assigned to {}", problem_id, assignee_name);
        Ok(())
    })
}
