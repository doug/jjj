pub mod embeddings;
pub mod entities;
pub mod events;
pub mod migrations;
pub mod schema;
pub mod search;
pub mod sync;
pub mod validate;

pub use embeddings::{
    clear_embeddings, count_embeddings, delete_embedding, get_embedding_model, list_embeddings,
    load_embedding, upsert_embedding, EmbeddingRecord,
};
pub use schema::{Database, SCHEMA_VERSION};
pub use search::{search, search_events, SearchResult};
pub use sync::{
    dump_to_markdown, is_dirty, load_from_markdown, rebuild_embeddings, rebuild_fts, set_dirty,
    update_fts_entry,
};
pub use validate::{validate, ValidationError};
