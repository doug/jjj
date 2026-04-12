use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::models::CritiqueStatus;
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
        check_and_prompt_approve_solve(ctx)?;
    }

    Ok(())
}

fn check_and_prompt_approve_solve(ctx: &CommandContext) -> Result<()> {
    let store = &ctx.store;

    // Find user's active solutions
    let solutions = store.list_solutions()?;
    let user = store.jj_client.user_name().unwrap_or_default();

    for solution in solutions
        .iter()
        .filter(|s| s.is_submitted() && s.assignee.as_deref() == Some(&user))
    {
        // Check if all critiques are resolved
        let critiques = store.list_critiques_for_solution(&solution.id)?;
        let open_critiques: Vec<_> = critiques
            .iter()
            .filter(|c| c.status == CritiqueStatus::Open)
            .collect();

        if open_critiques.is_empty() && !critiques.is_empty() {
            // All critiques resolved - prompt to approve
            if prompt_yes_no(&format!(
                "All critiques on {} \"{}\" resolved. Approve solution?",
                solution.id, solution.title
            )) {
                crate::domain::approve_solution(&ctx.store, &solution.id, false, None)?;
                println!("  Solution {} approved.", solution.id);
            }
        }
    }

    Ok(())
}
