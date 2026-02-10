use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, ProblemStatus};
use crate::storage::MetadataStore;
use std::io::{self, Write};

fn prompt_yes_no(message: &str) -> bool {
    print!("{} [Y/n] ", message);
    if io::stdout().flush().is_err() {
        return false;
    }

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}

pub fn execute(
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    if dry_run {
        println!("Would push to {}:", remote);
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj/meta");
        return Ok(());
    }

    // 1. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        let result = jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote]);
        if result.is_err() {
            // Retry with --allow-new for new bookmarks
            jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote, "--allow-new"])?;
        }
    }

    // 2. Always push jjj/meta
    println!("Pushing jjj/meta...");
    let result = jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote]);
    if result.is_err() {
        // Retry with --allow-new for new bookmarks
        jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote, "--allow-new"])?;
    }

    println!("Pushed to {}.", remote);

    // 3. Smart prompts (unless --no-prompt)
    if !no_prompt {
        check_and_prompt_accept_solve(&store)?;
    }

    Ok(())
}

fn check_and_prompt_accept_solve(store: &MetadataStore) -> Result<()> {
    // Find user's active solutions
    let solutions = store.list_solutions()?;
    let user = store.jj_client.user_name().unwrap_or_default();

    for solution in solutions.iter().filter(|s| {
        s.is_active() && s.assignee.as_deref() == Some(&user)
    }) {
        // Check if all critiques are resolved
        let critiques = store.get_critiques_for_solution(&solution.id)?;
        let open_critiques: Vec<_> = critiques
            .iter()
            .filter(|c| c.status == CritiqueStatus::Open)
            .collect();

        if open_critiques.is_empty() && !critiques.is_empty() {
            // All critiques resolved - prompt to accept
            if prompt_yes_no(&format!(
                "All critiques on {} \"{}\" resolved. Accept solution?",
                solution.id, solution.title
            )) {
                let mut solution = store.load_solution(&solution.id)?;
                solution.accept();
                store.save_solution(&solution)?;
                println!("  Solution {} accepted.", solution.id);

                // Check if problem can be solved
                let problem = store.load_problem(&solution.problem_id)?;
                if problem.status == ProblemStatus::Open || problem.status == ProblemStatus::InProgress {
                    // Check for other active solutions
                    let other_active: Vec<_> = solutions
                        .iter()
                        .filter(|s| s.problem_id == solution.problem_id && s.is_active() && s.id != solution.id)
                        .collect();

                    if other_active.is_empty()
                        && prompt_yes_no(&format!(
                            "Problem {} \"{}\" has no other active solutions. Mark solved?",
                            problem.id, problem.title
                        ))
                    {
                        let mut problem = store.load_problem(&solution.problem_id)?;
                        problem.set_status(ProblemStatus::Solved);
                        store.save_problem(&problem)?;
                        println!("  Problem {} solved.", problem.id);
                    }
                }
            }
        }
    }

    Ok(())
}
