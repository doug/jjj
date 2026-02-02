use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute() -> Result<()> {
    println!("Initializing jjj in the current repository...");

    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.init()?;

    println!("✓ jjj initialized successfully!");
    println!();
    println!("The jjj/meta bookmark has been created to store task and review metadata.");
    println!("Run 'jjj ui' to launch the interactive TUI.");

    Ok(())
}
