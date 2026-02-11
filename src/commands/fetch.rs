use std::fs;

use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute(ctx: &CommandContext, remote: &str) -> Result<()> {
    let jj_client = ctx.jj();

    // Check if we need to save local changes before fetch
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
    if db_path.exists() {
        let db = Database::open(&db_path)?;
        if db::is_dirty(&db)? {
            println!("Saving local changes before fetch...");
            db::dump_to_markdown(&db, &ctx.store)?;
            ctx.store
                .commit_changes("Sync local changes before fetch")?;
        }
    }

    // Snapshot counts before fetch
    let solutions_before = ctx.store.list_solutions().unwrap_or_default().len();
    let critiques_before = ctx.store.list_critiques().unwrap_or_default().len();

    // 1. Fetch from remote
    println!("Fetching from {}...", remote);
    jj_client.execute(&["git", "fetch", "--remote", remote])?;

    // 2. Update jjj-meta workspace if it exists
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    if meta_path.exists() {
        if let Ok(meta_client) = JjClient::with_root(meta_path) {
            if let Err(e) = meta_client.execute(&["new", "jjj/meta@origin"]) {
                eprintln!("Warning: could not update jjj-meta workspace: {}", e);
            }
        }
    }

    // 3. Rebuild database from updated markdown files
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
