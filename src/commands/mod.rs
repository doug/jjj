pub mod automation;
pub mod completion;
pub mod conflicts;
pub mod critique;
pub mod db;
pub mod doctor;
pub mod events;
pub mod fetch;
pub mod init;
pub mod insights;
pub mod milestone;
pub mod next;
pub mod overlaps;
pub mod problem;
pub mod push;
pub mod rank;
pub mod search;
pub mod solution;
pub mod status;
pub mod sync;
pub mod tags;
pub mod timeline;
pub mod ui;
pub mod whoami;

use crate::cli::{Cli, Commands};

/// Resolve a `--body` argument into entity body text.
///
/// Every entity has one free-form body — a problem's description, a solution's
/// approach, a critique's argument — and until this existed the only way to
/// write one was an interactive `$EDITOR`. A headless agent has no editor, so
/// agents in a swarm trial did the only thing left to them and crammed
/// multi-paragraph reasoning into the *title*, producing critique titles
/// hundreds of characters long. In a tool whose entire model is conjecture and
/// refutation, the argument is the payload; it must be writable from a script.
///
/// `-` reads stdin, so a long argument can be piped in without shell quoting
/// mangling it.
pub fn read_body(body: Option<String>) -> Result<String> {
    match body {
        None => Ok(String::new()),
        Some(arg) if arg == "-" => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(crate::error::JjjError::Io)?;
            Ok(buf.trim_end().to_string())
        }
        Some(arg) => Ok(arg),
    }
}
use crate::context::CommandContext;
use crate::db::{search as db_search, Database};
use crate::display::short_id;
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
            if dry_run {
                println!("Sync dry-run: would fetch from '{remote}' and push to it.");
                println!("Run without --dry-run to perform the sync.");
                Ok(())
            } else {
                fetch::execute(ctx, &remote)?;
                push::execute(ctx, vec![], &remote, no_prompt, dry_run)
            }
        }

        // GitHub bridge
        Commands::Github { action, dry_run } => sync::execute(ctx, action, dry_run),

        // Coordination identity
        Commands::Automation { command } => match command {
            crate::cli::AutomationCommands::List { json } => automation::list(ctx, json),
            crate::cli::AutomationCommands::Migrate { force } => automation::migrate(ctx, force),
        },
        Commands::Doctor { json } => doctor::execute(ctx, json),
        Commands::Whoami { json } => whoami::execute(ctx, json),

        // Conflict discovery + resolution
        Commands::Conflicts { json } => conflicts::list(ctx, json),
        Commands::Resolve {
            id,
            ours,
            theirs,
            rationale,
        } => {
            use crate::storage::merge::ConflictSide;
            let side = if theirs {
                ConflictSide::Remote
            } else if ours {
                ConflictSide::Local
            } else {
                return Err(crate::error::JjjError::Validation(
                    "Specify which side to keep: --ours (your local edit) or --theirs (the fetched edit)."
                        .to_string(),
                ));
            };
            conflicts::resolve(ctx, &id, side, rationale.as_deref())
        }

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
            user,
            mine,
            since,
            json,
            limit,
        } => events::execute(
            ctx,
            action,
            from,
            to,
            problem,
            solution,
            event_type,
            search,
            crate::commands::problem::resolve_mine(ctx, user, mine),
            since,
            json,
            limit,
        ),

        // Timeline
        Commands::Timeline { problem_id, json } => timeline::execute(ctx, problem_id, json),

        // Tags
        Commands::Tags { json } => tags::execute(ctx, json),

        // Rank
        Commands::Rank { action } => rank::execute(ctx, action),

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
        let type_prefix = result.entity_type.chars().next().unwrap_or('?');
        println!(
            "  {}/{}  [{:.2}]  \"{}\"",
            type_prefix,
            short_id(&result.entity_id),
            result.similarity,
            truncate_title(&result.title, 50)
        );
    }

    Ok(())
}

pub(crate) fn truncate_title(s: &str, max_len: usize) -> String {
    crate::utils::truncate(s, max_len)
}

/// Best-effort FTS lookup for entities whose title resembles `title`.
///
/// Used by `problem new` and `solution new` to short-circuit creation when a
/// near-duplicate already exists. Returns `Ok(())` when the user should be
/// allowed to proceed, or a `Validation` error when a similar entity was
/// found and `--force` would override.
///
/// `entity_type` is the FTS tag (`"problem"`, `"solution"`, etc.) and
/// `type_prefix` is the short reference prefix (`"p"`, `"s"`, etc.) used in
/// the warning. Skips silently if the SQLite DB hasn't been built yet.
pub(crate) fn check_similar_entities(
    ctx: &CommandContext,
    title: &str,
    entity_type: &str,
    type_prefix: char,
) -> Result<()> {
    let repo_root = ctx.jj().repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");
    if !db_path.exists() {
        return Ok(());
    }
    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(_) => return Ok(()),
    };
    let results = match db_search::search(db.conn(), title, Some(entity_type)) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    if results.is_empty() {
        return Ok(());
    }

    eprintln!("A similar {} already exists:", entity_type);
    for r in &results {
        eprintln!(
            "  {}/{} — \"{}\"",
            type_prefix,
            short_id(&r.entity_id),
            r.title
        );
    }

    // `--force` used to be the whole of the advice, which is the wrong move in
    // the common case: when two people (or two agents) race onto the same work,
    // forcing creates exactly the duplicate that was just detected. Name the
    // two situations instead, because they call for opposite actions.
    eprintln!();
    eprintln!("If that is the same work, someone already has it — pick something else.");
    eprintln!("If you are proposing a genuinely different approach, give it a title that says");
    eprintln!(
        "how it differs, then --force. Rival {}s are welcome; duplicates are not.",
        entity_type
    );

    Err(crate::error::JjjError::Validation(format!(
        "a similar {entity_type} already exists"
    )))
}
