use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{ProblemStatus, Solution, SolutionStatus};
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

    // 1. Review Check (solution-level)
    if !force {
        let solutions = store.list_solutions()?;
        let solution = solutions.iter().find(|s| s.change_ids.contains(&change_id));

        if let Some(sol) = solution {
            if !sol.requested_reviewers.is_empty() && !sol.has_lgtm_from_requested_reviewer() {
                return Err(format!(
                    "Solution {} has not been LGTM'd by a requested reviewer. Use --force to bypass.",
                    sol.id
                )
                .into());
            }
        }
    }

    // 2. Rebase onto main
    println!("Rebasing onto main...");
    jj_client.execute(&["rebase", "-d", "main"])?;

    // 3. Squash
    println!("Squashing...");
    jj_client.squash()?;

    // 4. Update Solution Status
    let solutions = store.list_solutions()?;
    let mut solutions_to_update = Vec::new();

    for solution in solutions {
        if solution.change_ids.contains(&change_id) {
            solutions_to_update.push(solution);
        }
    }

    for mut solution in solutions_to_update {
        // Mark solution as accepted if it was testing
        if solution.status == SolutionStatus::Testing {
            solution.accept();
            store.save_solution(&solution)?;
            println!("Solution {} accepted.", solution.id);

            // Check if we can solve the problem
            let (can_solve, _) = store.can_solve_problem(&solution.problem_id)?;
            if can_solve {
                let mut problem = store.load_problem(&solution.problem_id)?;
                problem.set_status(ProblemStatus::Solved);
                store.save_problem(&problem)?;
                println!("Problem {} solved.", problem.id);
            }
        }
    }

    println!("Submitted successfully.");
    Ok(())
}
