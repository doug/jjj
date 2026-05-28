use std::fs;
use std::path::Path;

use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::storage::merge::{
    merge_entity_md, merge_events_jsonl, read_base_file, snapshot_base, write_base_file,
};
use crate::storage::MetadataStore;

/// Merge a single remote entity file into the local copy using a three-way
/// merge against the base snapshot.
///
/// Returns true when the merged result differs from the existing local file
/// (i.e. the file changed as a result of the fetch).
fn merge_entity_into_local(
    base_path: &Path,
    local_path: &Path,
    relative: &Path,
    remote_content: &str,
) -> Result<bool> {
    let local_full = local_path.join(relative);
    let local_existing = fs::read_to_string(&local_full).ok();
    let base_existing = read_base_file(base_path, relative);

    let merged = match local_existing.as_deref() {
        // No local file → adopt the remote version as-is.
        None => remote_content.to_string(),
        Some(local) => merge_entity_md(base_existing.as_deref(), local, remote_content)?,
    };

    let changed = local_existing.as_deref() != Some(merged.as_str());

    if changed {
        if let Some(parent) = local_full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local_full, &merged)?;
    }

    // Advance the ancestor for the next fetch regardless of whether the
    // merged result equals the local — the merge base must be the latest
    // remote content we've observed.
    write_base_file(base_path, relative, remote_content)?;
    Ok(changed)
}

pub fn execute(ctx: &CommandContext, remote: &str) -> Result<()> {
    let jj_client = ctx.jj();
    let sync_config = ctx.store.load_config().unwrap_or_default().sync;
    let has_git = jj_client.has_git_backend();

    // Resolve sync commands: explicit config > git default > skip
    let fetch_cmd = match sync_config.resolve_fetch(has_git) {
        Some(cmd) => cmd,
        None => {
            println!("No sync backend configured and no git backend detected.");
            println!("Configure [sync] fetch in config.toml for custom sync commands.");
            return Ok(());
        }
    };

    // Check if we need to save local changes before fetch
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
    if db_path.exists() {
        let db = Database::open(&db_path)?;
        if db::is_dirty(&db)? {
            println!("Saving local changes before fetch...");
            db::dump_to_markdown(&db, &ctx.store)?;
            ctx.store.commit_changes()?;
        }
    }

    // Snapshot counts before fetch
    let solutions_before = ctx.store.list_solutions().unwrap_or_default().len();
    let critiques_before = ctx.store.list_critiques().unwrap_or_default().len();

    // 1. Fetch from remote using configured or default command.
    println!("Fetching from {}...", remote);
    let vars = [("remote", remote), ("bookmark", "jjj")];
    jj_client.execute_sync_command(&fetch_cmd, &vars)?;

    // Track the jjj bookmark from the remote if it exists
    if let Some(track_cmd) = sync_config.resolve_track(has_git) {
        let _ = jj_client.execute_sync_command(&track_cmd, &vars);
    }

    // 2. Extract updated files from the fetched jjj bookmark and three-way
    //    merge them into the local working set.
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    let base_path = ctx.store.base_path();
    let mut merge_conflicts: Vec<String> = Vec::new();
    if jj_client.bookmark_exists("jjj")? {
        fs::create_dir_all(&meta_path)?;
        fs::create_dir_all(&base_path)?;

        for dir in &["problems", "solutions", "critiques", "milestones"] {
            fs::create_dir_all(meta_path.join(dir))?;
            if let Ok(listing) =
                jj_client.execute(&["file", "list", "-r", "jjj", &format!("{}/", dir)])
            {
                for file_path in listing.lines().filter(|l| !l.trim().is_empty()) {
                    let remote = match jj_client.execute(&["file", "show", "-r", "jjj", file_path])
                    {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let relative = Path::new(file_path);
                    match merge_entity_into_local(&base_path, &meta_path, relative, &remote) {
                        Ok(_) => {
                            // Detect any conflict markers we emitted so we can
                            // surface them to the user once at the end.
                            let local_full = meta_path.join(relative);
                            if let Ok(content) = fs::read_to_string(&local_full) {
                                if content.contains("<<<<<<< local") {
                                    merge_conflicts.push(file_path.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  Warning: merge failed for {}: {}", file_path, e);
                        }
                    }
                }
            }
        }

        // config.toml: last-fetch-wins (rarely a hot conflict surface).
        if let Ok(content) = jj_client.execute(&["file", "show", "-r", "jjj", "config.toml"]) {
            let _ = fs::write(meta_path.join("config.toml"), &content);
        }

        // events.jsonl: append-only line union. Base not needed.
        if let Ok(remote_events) = jj_client.execute(&["file", "show", "-r", "jjj", "events.jsonl"])
        {
            let local_events_path = meta_path.join("events.jsonl");
            let local_events = fs::read_to_string(&local_events_path).unwrap_or_default();
            let merged = merge_events_jsonl(&local_events, &remote_events);
            if merged != local_events {
                let _ = fs::write(&local_events_path, &merged);
            }
        }

        // Capture the new ancestor for the next fetch.
        snapshot_base(&meta_path, &base_path)?;
    }

    // 3. Update working copy to avoid stale workspace errors
    let ws_prefix = sync_config.workspace.as_deref();
    let _ = jj_client.execute_workspace(ws_prefix, "update-stale", &[]);

    // 4. Rebuild database from updated markdown files
    println!("Rebuilding database...");
    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    let db = Database::open(&db_path)?;
    let store_after = MetadataStore::new(jj_client.clone())?;
    db::load_from_markdown(&db, &store_after)?;

    // 4. Show summary - store_after already created above
    let solutions_after = store_after.list_solutions().unwrap_or_default().len();
    let critiques_after = store_after.list_critiques().unwrap_or_default().len();

    let new_solutions = solutions_after.saturating_sub(solutions_before);
    let new_critiques = critiques_after.saturating_sub(critiques_before);

    println!("Fetched from {}.", remote);
    if new_solutions > 0 {
        println!("  {} new solution(s)", new_solutions);
    }
    if new_critiques > 0 {
        println!("  {} new critique(s)", new_critiques);
    }
    if new_solutions == 0 && new_critiques == 0 {
        println!("  No new jjj changes.");
    }

    if !merge_conflicts.is_empty() {
        eprintln!(
            "\nMerge conflicts in {} file(s) — both sides edited the same body:",
            merge_conflicts.len()
        );
        for path in &merge_conflicts {
            eprintln!("  {}", path);
        }
        eprintln!("Open each file, resolve the <<<<<<< / >>>>>>> markers, then save.");
    }

    Ok(())
}
