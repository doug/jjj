use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, ProblemStatus, SolutionStatus};
use crate::storage::MetadataStore;

pub fn execute(json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let user_identity = jj_client.user_identity()?;
    let store = MetadataStore::new(jj_client)?;

    if !json {
        println!("Dashboard for {}", user_identity);
        println!();
    }

    // Show problems assigned to user
    let problems = store.list_problems()?;
    let my_problems: Vec<_> = problems
        .iter()
        .filter(|p| {
            p.assignee.as_ref().map(|a| format!("@{}", a)).as_deref() == Some(&user_identity)
                && p.is_open()
        })
        .collect();

    if !my_problems.is_empty() {
        println!("My Problems ({}):", my_problems.len());
        for problem in &my_problems {
            println!("  {} - {} [{}]", problem.id, problem.title, problem.status);
        }
        println!();
    }

    // Show solutions assigned to user
    let solutions = store.list_solutions()?;
    let my_solutions: Vec<_> = solutions
        .iter()
        .filter(|s| {
            s.assignee.as_ref().map(|a| format!("@{}", a)).as_deref() == Some(&user_identity)
                && s.is_active()
        })
        .collect();

    if !my_solutions.is_empty() {
        println!("My Solutions ({}):", my_solutions.len());
        for solution in &my_solutions {
            println!(
                "  {} - {} [{}] ({})",
                solution.id, solution.title, solution.status, solution.problem_id
            );
        }
        println!();
    }

    // Show open critiques on my solutions
    let critiques = store.list_critiques()?;
    let open_critiques_on_my_solutions: Vec<_> = critiques
        .iter()
        .filter(|c| {
            c.status == CritiqueStatus::Open
                && my_solutions.iter().any(|s| s.id == c.solution_id)
        })
        .collect();

    if !open_critiques_on_my_solutions.is_empty() {
        println!(
            "Open Critiques on My Solutions ({}):",
            open_critiques_on_my_solutions.len()
        );
        for critique in &open_critiques_on_my_solutions {
            println!(
                "  {} - {} [{}, {}]",
                critique.id, critique.title, critique.severity, critique.solution_id
            );
        }
        println!();
    }

    // Summary stats
    let open_problems = problems
        .iter()
        .filter(|p| p.status == ProblemStatus::Open)
        .count();
    let in_progress_problems = problems
        .iter()
        .filter(|p| p.status == ProblemStatus::InProgress)
        .count();
    let testing_solutions = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Testing)
        .count();
    let open_critiques = critiques
        .iter()
        .filter(|c| c.status == CritiqueStatus::Open)
        .count();

    println!("Summary:");
    println!("  Problems: {} open, {} in progress", open_problems, in_progress_problems);
    println!("  Solutions: {} testing", testing_solutions);
    println!("  Critiques: {} open", open_critiques);

    if my_problems.is_empty()
        && my_solutions.is_empty()
    {
        println!("\nNo pending work assigned to you.");
    }

    if json {
        let dashboard_data = serde_json::json!({
            "my_problems": my_problems,
            "my_solutions": my_solutions,
            "open_critiques": open_critiques_on_my_solutions,
        });
        println!("{}", serde_json::to_string_pretty(&dashboard_data)?);
    }

    Ok(())
}
