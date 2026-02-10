//! Command execution context providing shared access to storage and JJ client.

use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

/// Shared context for all command execution.
pub struct CommandContext {
    pub store: MetadataStore,
}

impl CommandContext {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        Ok(Self { store })
    }

    pub fn jj(&self) -> &JjClient {
        &self.store.jj_client
    }
}
