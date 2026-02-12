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

    println!("✓ jjj initialized successfully!");
    println!();
    println!("The jjj bookmark has been created to store task and review metadata.");
    println!("Run 'jjj ui' to launch the interactive TUI.");

    Ok(())
}
