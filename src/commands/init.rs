use crate::db::{self, Database};
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute() -> Result<()> {
    println!("Initializing jjj in the current repository...");

    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    store.init()?;

    // Create and initialize the database
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
    let db = Database::open(&db_path)?;
    db::load_from_markdown(&db, &store)?;

    let sync_config = store.load_config().unwrap_or_default().sync;
    let has_git = jj_client.has_git_backend();

    println!("✓ jjj initialized successfully!");
    println!();
    println!("Metadata stored in .jj/jjj-meta/");
    if sync_config.has_sync(has_git) {
        println!("Use 'jjj push' / 'jjj fetch' to sync with collaborators.");
    } else {
        println!("No sync backend configured. Add [sync] to config.toml for custom sync.");
    }
    println!("Run 'jjj ui' to launch the interactive TUI.");

    Ok(())
}
