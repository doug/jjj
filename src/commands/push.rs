use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
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
    ctx: &CommandContext,
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    // Sync SQLite to markdown and validate before pushing
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
    if db_path.exists() {
        let db = Database::open(&db_path)?;

        println!("Syncing database to files...");
        db::dump_to_markdown(&db, store)?;

        println!("Validating metadata...");
        let errors = db::validate(&db)?;
        if !errors.is_empty() {
            println!("Validation errors:");
            for error in &errors {
                println!("  \u{2717} {}", error);
            }
            return Err(crate::error::JjjError::Validation(
                "Push aborted. Fix errors and retry.".to_string(),
            ));
        }
        println!("  \u{2713} All checks passed");

        // Commit the changes from dump_to_markdown
        store.commit_changes("jjj: sync database before push")?;
    }

    if dry_run {
        println!("Would push to {}:", remote);
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj");
        return Ok(());
    }

    // 1. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        // Use --allow-empty-description since metadata commits may lack descriptions
        let result = jj_client.execute(&[
            "git",
            "push",
            "-b",
            bookmark,
            "--remote",
            remote,
            "--allow-empty-description",
        ]);
        if result.is_err() {
            // Retry with --allow-new for new bookmarks (deprecated but still works)
            jj_client.execute(&[
                "git",
                "push",
                "-b",
                bookmark,
                "--remote",
                remote,
                "--allow-empty-description",
                "--allow-new",
            ])?;
        }
    }

    // 2. Always push jjj bookmark
    println!("Pushing jjj...");
    // The jjj bookmark points to orphan commits that may lack descriptions
    let result = jj_client.execute(&[
        "git",
        "push",
        "-b",
        "jjj",
        "--remote",
        remote,
        "--allow-empty-description",
    ]);
    if result.is_err() {
        // Retry with --allow-new for new bookmarks (deprecated but still works)
        jj_client.execute(&[
            "git",
            "push",
            "-b",
            "jjj",
            "--remote",
            remote,
            "--allow-empty-description",
            "--allow-new",
        ])?;
    }

    println!("Pushed to {}.", remote);

    // Clear dirty flag after successful push
    if db_path.exists() {
        let db = Database::open(&db_path)?;
        db::set_dirty(&db, false)?;
    }

    // 3. Smart prompts (unless --no-prompt)
    if !no_prompt {
        check_and_prompt_accept_solve(store)?;
    }

    Ok(())
}

fn check_and_prompt_accept_solve(store: &MetadataStore) -> Result<()> {
    // Find user's active solutions
    let solutions = store.list_solutions()?;
    let user = store.jj_client.user_name().unwrap_or_default();

    for solution in solutions
        .iter()
        .filter(|s| s.is_active() && s.assignee.as_deref() == Some(&user))
    {
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
                solution.approve();
                store.save_solution(&solution)?;
                println!("  Solution {} accepted.", solution.id);

                // Check if problem can be solved
                let problem = store.load_problem(&solution.problem_id)?;
                if problem.status == ProblemStatus::Open
                    || problem.status == ProblemStatus::InProgress
                {
                    // Check for other active solutions
                    let other_active: Vec<_> = solutions
                        .iter()
                        .filter(|s| {
                            s.problem_id == solution.problem_id
                                && s.is_active()
                                && s.id != solution.id
                        })
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
