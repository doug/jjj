use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{CritiqueStatus, ProblemStatus, SolutionStatus};

pub fn submit(ctx: &CommandContext, force: bool) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let change_id = jj_client.current_change_id()?;

    // Find solution for current change
    let solutions = store.list_solutions()?;
    let solution = solutions.iter().find(|s| s.change_ids.contains(&change_id));

    let solution = match solution {
        Some(s) => s.clone(),
        None => {
            // No solution attached — just squash
            println!("No solution found for current change. Squashing only.");
            let desc = jj_client.change_description("@").unwrap_or_default();
            jj_client.execute(&["rebase", "-d", "main"])?;
            let msg = if desc.is_empty() { None } else { Some(desc.as_str()) };
            jj_client.squash(msg)?;
            println!("Submitted successfully.");
            return Ok(());
        }
    };

    println!("Submitting {}: {}", solution.id, solution.title);

    if !force {
        // Check critiques - open critiques (including review requests) block submission
        let critiques = store.list_critiques()?;
        let open_critiques: Vec<_> = critiques
            .iter()
            .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
            .collect();

        if !open_critiques.is_empty() {
            eprintln!("\n  * {} open critique(s):", open_critiques.len());
            for c in &open_critiques {
                eprintln!("    {}: {} [{}]", c.id, c.title, c.severity);
                eprintln!("    -> jjj critique address {}", c.id);
            }
            eprintln!("\nCannot auto-accept. Use --force to submit without acceptance.");
            return Err(crate::error::JjjError::CannotAcceptSolution(
                "Unresolved critiques block submission".to_string(),
            ));
        }
    }

    // Rebase and squash — pass the current description to avoid an interactive editor prompt
    let desc = jj_client.change_description("@").unwrap_or_default();
    jj_client.execute(&["rebase", "-d", "main"])?;
    let msg = if desc.is_empty() { None } else { Some(desc.as_str()) };
    jj_client.squash(msg)?;

    // Auto-accept
    if solution.status == SolutionStatus::Review || solution.status == SolutionStatus::Proposed {
        let mut solution = store.load_solution(&solution.id)?;
        if force {
            solution.force_accepted = true;
        }
        solution.accept();
        store.save_solution(&solution)?;
        println!("  Solution {} accepted", solution.id);

        // Auto-solve problem if conditions met
        let problem_id = solution.problem_id.clone();
        let all_solutions = store.list_solutions()?;
        let active_solutions: Vec<_> = all_solutions
            .iter()
            .filter(|s| s.problem_id == problem_id && s.is_active() && s.id != solution.id)
            .collect();
        let sub_problems = store.get_subproblems(&problem_id)?;
        let open_sub_problems: Vec<_> = sub_problems.iter().filter(|p| p.is_open()).collect();

        let problem = store.load_problem(&problem_id)?;
        if active_solutions.is_empty() && open_sub_problems.is_empty() && problem.is_open() {
            let mut problem = problem;
            problem.set_status(ProblemStatus::Solved);
            store.save_problem(&problem)?;
            println!(
                "  Problem {} solved (only solution, no open sub-problems)",
                problem_id
            );
        }
    }

    // Auto-create/update GitHub PR if enabled
    if let Ok(mut sol) = store.load_solution(&solution.id) {
        crate::sync::hooks::auto_create_or_update_pr(ctx, &mut sol);
    }

    println!("\nSquashed changes into trunk.");
    Ok(())
}
