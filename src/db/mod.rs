pub mod entities;
pub mod events;
pub mod schema;
pub mod search;
pub mod sync;
pub mod validate;

pub use schema::{Database, SCHEMA_VERSION};
pub use search::{search, search_events, SearchResult};
pub use sync::{dump_to_markdown, is_dirty, load_from_markdown, rebuild_fts, set_dirty};
pub use validate::{validate, ValidationError};
