use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, ProblemStatus, Solution, SolutionStatus};
use crate::storage::MetadataStore;

pub fn start(arg: String, problem_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    // Check if arg is a Solution ID
    if let Ok(solution) = store.load_solution(&arg) {
        println!("Resuming solution {} ({})", solution.id, solution.title);

        // Check if solution has an active change attached
        if let Some(change_id) = solution.change_ids.last() {
            println!("Switching to change {}", change_id);
            jj_client.edit(change_id)?;
        } else {
            println!("No active change for solution. Creating new change.");
            jj_client.new_empty_change(&solution.title)?;

            // Attach new change to solution
            let change_id = jj_client.current_change_id()?;
            let mut solution = solution;
            solution.attach_change(change_id);
            solution.start_testing();
            store.save_solution(&solution)?;

            // Update problem status
            let mut problem = store.load_problem(&solution.problem_id)?;
            if problem.status == ProblemStatus::Open {
                problem.set_status(ProblemStatus::InProgress);
                store.save_problem(&problem)?;
            }
        }

        return Ok(());
    }

    // Treat arg as Title for new Solution
    let title = arg;

    // Problem is required for new solutions
    let problem_id = match problem_id {
        Some(pid) => pid,
        None => {
            return Err(
                "Problem ID required for new solutions. Use --problem P-1 or provide a solution ID to resume."
                    .into(),
            );
        }
    };

    // Validate problem exists
    let problem = store.load_problem(&problem_id)?;

    println!("Starting new solution: {}", title);
    println!("  Addresses: {} - {}", problem.id, problem.title);

    store.with_metadata(&format!("Start solution: {}", title), || {
        // Create new empty change
        jj_client.new_empty_change(&title)?;
        let change_id = jj_client.current_change_id()?;

        // Create Solution
        let solution_id = store.next_solution_id()?;
        let mut solution = Solution::new(solution_id.clone(), title.clone(), problem_id.clone());
        solution.attach_change(change_id);
        solution.start_testing();

        store.save_solution(&solution)?;

        // Update problem
        let mut problem = store.load_problem(&problem_id)?;
        problem.add_solution(solution_id.clone());
        if problem.status == ProblemStatus::Open {
            problem.set_status(ProblemStatus::InProgress);
        }
        store.save_problem(&problem)?;

        println!("Started solution {} ({})", solution.id, solution.title);
        Ok(())
    })
}

pub fn submit(force: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    let change_id = jj_client.current_change_id()?;

    // Find solution for current change
    let solutions = store.list_solutions()?;
    let solution = solutions
        .iter()
        .find(|s| s.change_ids.contains(&change_id));

    let solution = match solution {
        Some(s) => s.clone(),
        None => {
            // No solution attached — just squash
            println!("No solution found for current change. Squashing only.");
            jj_client.execute(&["rebase", "-d", "main"])?;
            jj_client.squash()?;
            println!("Submitted successfully.");
            return Ok(());
        }
    };

    println!("Submitting {}: {}", solution.id, solution.title);

    if !force {
        let mut blocked = false;

        // Check critiques
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
            blocked = true;
        }

        // Check reviews
        if !solution.requested_reviewers.is_empty() && !solution.has_lgtm_from_requested_reviewer()
        {
            let pending: Vec<_> = solution
                .requested_reviewers
                .iter()
                .filter(|r| !solution.reviewed_by.contains(r))
                .collect();
            eprintln!(
                "\n  * Review pending from {}",
                pending
                    .iter()
                    .map(|r| format!("@{}", r))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            eprintln!("    -> waiting for LGTM");
            blocked = true;
        }

        if blocked {
            eprintln!("\nCannot auto-accept. Use --force to submit without acceptance.");
            return Err(crate::error::JjjError::CannotAcceptSolution(
                "Unresolved critiques or pending reviews".to_string(),
            ));
        }
    }

    // Rebase and squash
    jj_client.execute(&["rebase", "-d", "main"])?;
    jj_client.squash()?;

    // Auto-accept
    if solution.status == SolutionStatus::Testing || solution.status == SolutionStatus::Proposed {
        let mut solution = store.load_solution(&solution.id)?;
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

    println!("\nSquashed changes into trunk.");
    Ok(())
}
