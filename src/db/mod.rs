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
    dump_to_markdown, is_dirty, load_from_markdown, open_cache_if_present, rebuild_embeddings,
    rebuild_fts, remove_entity_from_cache, set_dirty, sync_critique_to_cache,
    sync_milestone_to_cache, sync_problem_to_cache, sync_solution_to_cache, update_fts_entry,
};
pub use validate::{validate, ValidationError};
