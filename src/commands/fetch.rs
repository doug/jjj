use crate::context::CommandContext;
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute(ctx: &CommandContext, remote: &str) -> Result<()> {
    let jj_client = ctx.jj();

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

    // 3. Show summary - need fresh store to see changes after fetch
    let store_after = MetadataStore::new(jj_client.clone())?;
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
