use crate::error::Result;
use crate::jj::JjClient;

pub fn execute(remote: &str) -> Result<()> {
    let jj_client = JjClient::new()?;

    // 1. Fetch from remote
    println!("Fetching from {}...", remote);
    jj_client.execute(&["git", "fetch", "--remote", remote])?;

    // 2. Update jjj-meta workspace if it exists
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    if meta_path.exists() {
        // Update workspace to track jjj/meta bookmark
        // Create a new JjClient for the meta workspace
        if let Ok(meta_client) = JjClient::with_root(meta_path) {
            let _ = meta_client.execute(&["new", "jjj/meta@origin"]);
        }
    }

    println!("Fetched from {}", remote);
    Ok(())
}
