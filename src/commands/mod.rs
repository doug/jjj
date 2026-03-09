pub mod completion;
pub mod critique;
pub mod db;
pub mod events;
pub mod fetch;
pub mod init;
pub mod insights;
pub mod milestone;
pub mod next;
pub mod overlaps;
pub mod problem;
pub mod push;
pub mod search;
pub mod solution;
pub mod status;
pub mod sync;
pub mod tags;
pub mod timeline;
pub mod ui;

use crate::cli::{Cli, Commands};
use crate::context::CommandContext;
use crate::db::{search as db_search, Database};
use crate::error::Result;

/// Entry point for CLI command dispatch.
///
/// Commands that manage their own context (`Init`, `Ui`, `Completion`) are
/// handled directly. All other commands get a shared [`CommandContext`] created
/// once and passed to [`execute_with_context`].
pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        // Commands that don't need context (they create their own store or don't need one)
        Commands::Init => init::execute(),
        Commands::Ui => ui::execute(),
        Commands::Completion { shell } => completion::execute(shell),

        // All other commands need context - create it once here
        _ => {
            let ctx = CommandContext::new()?;
            execute_with_context(&ctx, cli.command)
        }
    }
}

/// Dispatch a command that requires shared storage/jj access.
///
/// All entity commands, sync, push/fetch, search, and event log commands route
/// through here, sharing a single [`CommandContext`] (which holds the
/// `MetadataStore` and `JjClient`).
fn execute_with_context(ctx: &CommandContext, command: Commands) -> Result<()> {
    match command {
        Commands::Problem { action } => problem::execute(ctx, action),
        Commands::Solution { action } => solution::execute(ctx, action),
        Commands::Critique { action } => critique::execute(ctx, action),
        Commands::Milestone { action } => milestone::execute(ctx, action),

        // Status (replaces dashboard + next)
        Commands::Status {
            all,
            mine,
            limit,
            json,
        } => status::execute(ctx, all, mine, limit, json),

        Commands::Next {
            top,
            mine,
            json,
            claim,
        } => next::execute(ctx, top, mine, json, claim),

        // Overlap detection
        Commands::Overlaps { json } => overlaps::execute(ctx, json),

        // Insights
        Commands::Insights { json } => insights::execute(ctx, json),

        // Transport: fetch, push, sync (fetch + push)
        Commands::Fetch { remote } => fetch::execute(ctx, &remote),
        Commands::Push {
            bookmarks,
            remote,
            no_prompt,
            dry_run,
        } => push::execute(ctx, bookmarks, &remote, no_prompt, dry_run),
        Commands::Sync {
            remote,
            no_prompt,
            dry_run,
        } => {
            fetch::execute(ctx, &remote)?;
            push::execute(ctx, vec![], &remote, no_prompt, dry_run)
        }

        // GitHub bridge
        Commands::Github { action, dry_run } => sync::execute(ctx, action, dry_run),

        // These are handled by execute() before calling this function
        Commands::Init | Commands::Ui | Commands::Completion { .. } => {
            unreachable!("These commands should be handled in execute()")
        }

        // Event log
        Commands::Events {
            action,
            from,
            to,
            problem,
            solution,
            event_type,
            search,
            since,
            json,
            limit,
        } => events::execute(
            ctx, action, from, to, problem, solution, event_type, search, since, json, limit,
        ),

        // Timeline
        Commands::Timeline { problem_id, json } => timeline::execute(ctx, problem_id, json),

        // Tags
        Commands::Tags { json } => tags::execute(ctx, json),

        // Search
        Commands::Search {
            query,
            r#type,
            text_only,
            json,
        } => search::execute(ctx, &query, r#type.as_deref(), text_only, json),

        // Database management
        Commands::Db { action } => db::execute(ctx, action),
    }
}

/// Print semantically related entities below a command's output.
///
/// Queries the local SQLite cache for similar entities using cosine similarity
/// on stored embeddings. Silently skips if the database doesn't exist, the
/// entity has no embedding, or no results exceed the 0.5 similarity threshold.
pub(crate) fn show_related_items(
    ctx: &CommandContext,
    entity_type: &str,
    entity_id: &str,
) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    if !db_path.exists() {
        return Ok(());
    }

    let db = Database::open(&db_path)?;
    let conn = db.conn();

    // Check if this entity has an embedding
    let has_embedding =
        crate::db::embeddings::load_embedding(conn, entity_type, entity_id)?.is_some();

    if !has_embedding {
        return Ok(());
    }

    let results = db_search::find_similar(conn, entity_type, entity_id, None, 5)?;

    if results.is_empty() {
        return Ok(());
    }

    // Filter to similarity > 0.5
    let results: Vec<_> = results.into_iter().filter(|r| r.similarity > 0.5).collect();

    if results.is_empty() {
        return Ok(());
    }

    println!("\n{}", "─".repeat(50));
    println!("\nRelated:");
    for result in results {
        let short_id = &result.entity_id[..6.min(result.entity_id.len())];
        let type_prefix = result.entity_type.chars().next().unwrap_or('?');
        println!(
            "  {}/{}  [{:.2}]  \"{}\"",
            type_prefix,
            short_id,
            result.similarity,
            truncate_title(&result.title, 50)
        );
    }

    Ok(())
}

pub(crate) fn truncate_title(s: &str, max_len: usize) -> String {
    crate::utils::truncate(s, max_len)
}
