pub mod automation;
pub mod completion;
pub mod conflicts;
pub mod contention;
pub mod critique;
pub mod db;
pub mod doctor;
pub mod escalate;
pub mod events;
pub mod fetch;
pub mod finding;
pub mod init;
pub mod insights;
pub mod milestone;
pub mod next;
pub mod overlaps;
pub mod plan;
pub mod prime;
pub mod problem;
pub mod push;
pub mod query;
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
/// Refuse a write when the entity moved between the caller reading it and
/// writing it back.
///
/// `edit --body` replaces a body wholesale, so an agent that writes one without
/// having seen the current text destroys whatever another agent put there — and
/// in a fleet, "I read this entity three minutes ago" is not the same as "this
/// is what it says now". The Lamport clock an entity already carries is exactly
/// the token needed to say so: read it from `show --json`, pass it back, and the
/// write is refused if anything intervened.
///
/// This is a fence within one clone, not a distributed lock. Two clones can
/// still both pass the check and merge afterwards — which is what jjj's merge is
/// for. beads reaches for the same primitive (`--if-revision`) on top of a real
/// database, and even there its own docs call the compare-and-set "a correctness
/// fence against concurrent claims, not an authorization boundary".
///
/// The refusal deliberately does **not** offer "drop --if-lamport" as the fix.
/// The guard fired precisely because the entity moved, so an agent told it can
/// drop the flag will drop the flag — defeating the fence at the one moment it
/// had something to say. beads reached the same conclusion about its migration
/// refusal and wrote down why: migrating is one-way, so handing an agent the
/// override as "the fix" lets it do the irreversible thing, and the override
/// "stays out of this payload entirely". The remedy named here is the one that
/// is always correct — re-read, reapply — and the overwrite is described as a
/// decision about someone else's work rather than a retry flag.
pub(crate) fn guard_lamport(
    expected: Option<u64>,
    actual: u64,
    kind: &str,
    id: &str,
) -> Result<()> {
    match expected {
        Some(want) if want != actual => Err(crate::error::JjjError::Precondition(format!(
            "{short} has changed since you read it — its clock is {actual}, you expected \
             {want}. Re-read it with `jjj {kind} show {short}` and reapply your change to \
             the text that is there now. Writing it unguarded would discard whatever \
             changed in between, which is a decision about someone else's work.",
            short = crate::display::short_id(id)
        ))),
        _ => Ok(()),
    }
}

/// Resolve `--body` / `--clear-body` for an `edit`, refusing a silent erase.
///
/// `--body ""` replaces a body with nothing, and an empty string is exactly what
/// a dead command substitution collapses to: `--body "$(cat notes.md)"` with the
/// file missing erases the entity's entire content and reports success. Measured
/// before this guard existed, that is precisely what happened — a problem's
/// description went from "Three weeks of measurements." to `''`, exit 0,
/// `Updated problem`. An empty `--body -` from a closed or empty stdin is the
/// same accident by another route.
///
/// At the flag layer the accident and a deliberate clear are the same bytes, so
/// the value is refused and the deliberate clear gets its own flag. This is
/// beads' `--notes ""` guard, which reaches the same conclusion from the same
/// reasoning; the difference is only in which field is irreplaceable.
///
/// `new` is deliberately **not** guarded: creating an entity with no body is
/// ordinary — most critiques have no separate argument at first — and there is
/// nothing there to lose.
pub(crate) fn edit_body(
    body: Option<String>,
    clear_body: bool,
    field: &str,
) -> Result<Option<String>> {
    if clear_body {
        return Ok(Some(String::new()));
    }
    let Some(arg) = body else {
        return Ok(None);
    };
    let text = read_body(Some(arg))?;
    if text.trim().is_empty() {
        return Err(crate::error::JjjError::Validation(format!(
            "--body carried no content, which would erase the {field}. An empty command \
             substitution or an empty file is the usual cause — check what you passed. \
             To erase it deliberately, use --clear-body."
        )));
    }
    Ok(Some(text))
}

/// Text to add to the end of a body, read the same way `--body` is.
///
/// Refuses empty for the same reason `edit_body` does: an empty command
/// substitution is the usual cause, and appending nothing is never what the
/// caller meant — but unlike `--body` it cannot be a request to clear, so there
/// is no `--clear` counterpart to point at.
pub(crate) fn append_body(append: Option<String>, field: &str) -> Result<Option<String>> {
    let Some(arg) = append else {
        return Ok(None);
    };
    let text = read_body(Some(arg))?;
    if text.trim().is_empty() {
        return Err(crate::error::JjjError::Validation(format!(
            "--append-body carried no content, so there is nothing to add to the \
             {field}. An empty command substitution or an empty file is the usual cause."
        )));
    }
    Ok(Some(text))
}

/// Join an existing body and an addition with one blank line between them.
///
/// The point of doing this here rather than in the caller's shell is *when* it
/// happens. An agent appending to a body had to read it, build the new text, and
/// write the whole thing back; anything written between its read and its write
/// was silently discarded, and `--if-lamport` could only turn that into a
/// refusal after the fact. Appending inside the write transaction reads the body
/// at write time, so there is no window to lose an edit in.
///
/// Two clones appending concurrently still contend — the body is one key in the
/// per-field merge, so one side's addition wins. That is a smaller problem than
/// the one this removes, and pretending otherwise would be the real hazard.
pub(crate) fn appended_body(current: &str, addition: &str) -> String {
    let current = current.trim_end();
    let addition = addition.trim();
    if current.is_empty() {
        return addition.to_string();
    }
    format!("{current}\n\n{addition}")
}

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
        Commands::Finding { action } => finding::execute(ctx, action),
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
            tag,
        } => next::execute(ctx, top, mine, json, claim, tag),

        Commands::Plan { action } => match action {
            crate::cli::PlanAction::Apply {
                file,
                dry_run,
                json,
            } => plan::execute(ctx, file, dry_run, json),
        },

        // Escalation: the swarm asking for a person
        Commands::Escalate {
            reason,
            about,
            list,
            clear,
            json,
        } => escalate::execute(ctx, reason, about, list, clear, json),

        // Where the fleet is doubled up, and where nobody is
        Commands::Contention { json } => contention::execute(ctx, json),

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

        // One call that orients an agent at the start of a turn
        Commands::Prime { full, json } => prime::execute(ctx, json, full),

        // One query grammar across every kind
        Commands::Query {
            expr,
            json,
            count_by,
        } => query::execute(ctx, expr, json, count_by),

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
