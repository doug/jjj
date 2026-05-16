use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::storage::MetadataStore;

/// Merge remote events into the local events.jsonl, deduplicating by content.
fn merge_events_jsonl(local_path: &Path, remote_content: &str) {
    let existing: HashSet<String> = if local_path.exists() {
        fs::read_to_string(local_path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect()
    } else {
        HashSet::new()
    };

    let new_lines: Vec<&str> = remote_content
        .lines()
        .filter(|l| !l.trim().is_empty() && !existing.contains(*l))
        .collect();

    if new_lines.is_empty() {
        return;
    }

    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(local_path)
    {
        for line in &new_lines {
            let _ = writeln!(file, "{}", line);
        }
    }
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

    // 2. Extract updated files from the fetched jjj bookmark.
    // Use `jj file show` to read files from the remote bookmark without
    // needing a workspace checkout.
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    if jj_client.bookmark_exists("jjj")? {
        fs::create_dir_all(&meta_path)?;
        // Copy entity files from the fetched bookmark into the local meta directory
        for dir in &["problems", "solutions", "critiques", "milestones"] {
            fs::create_dir_all(meta_path.join(dir))?;
            // List files in this directory at the bookmark revision
            if let Ok(listing) =
                jj_client.execute(&["file", "list", "-r", "jjj", &format!("{}/", dir)])
            {
                for file_path in listing.lines().filter(|l| !l.trim().is_empty()) {
                    if let Ok(content) =
                        jj_client.execute(&["file", "show", "-r", "jjj", file_path])
                    {
                        let local_path = meta_path.join(file_path);
                        let _ = fs::write(&local_path, &content);
                    }
                }
            }
        }
        // Also fetch config.toml
        if let Ok(content) = jj_client.execute(&["file", "show", "-r", "jjj", "config.toml"]) {
            let _ = fs::write(meta_path.join("config.toml"), &content);
        }
        // Merge events: fetch remote events.jsonl and append any new events
        if let Ok(remote_events) = jj_client.execute(&["file", "show", "-r", "jjj", "events.jsonl"])
        {
            merge_events_jsonl(&meta_path.join("events.jsonl"), &remote_events);
        }
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

    Ok(())
}
