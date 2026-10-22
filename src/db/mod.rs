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
    is_dirty, load_from_markdown, load_from_markdown_incremental, open_cache_at,
    open_cache_if_present, rebuild_embeddings, rebuild_fts, remove_entity_from_cache, set_dirty,
    sync_critique_to_cache, sync_finding_to_cache, sync_milestone_to_cache, sync_problem_to_cache,
    sync_solution_to_cache, update_fts_entry,
};
pub use validate::{validate, ValidationError};

/// Is the SQLite cache bypassed for this process?
///
/// **This exists for measurement.** `JJJ_NO_DB=1` makes the store behave as if
/// `.jj/jjj.db` had never been built, so every read falls through to the
/// canonical markdown walk — which is how the cache-off half of the
/// `prime`/`query`/`next` benchmark in `tools/bench/cache_off.py` is taken, and
/// which answers "is the cache still load-bearing?" without deleting it first.
/// Unset, it changes nothing.
///
/// Note it only governs the *store's* opportunistic cache
/// ([`sync::open_cache_if_present`]). Commands that open the database directly
/// and have no filesystem fallback — `db rebuild`, `search`, the `--search`
/// flags on `problem list`/`solution list` — are unaffected and still require
/// the file. A *write* taken with it set also skips the cache upsert, so an
/// existing `.jj/jjj.db` is left stale; it is a read-path measurement switch,
/// not a supported way to run.
pub fn bypassed() -> bool {
    std::env::var_os("JJJ_NO_DB").is_some_and(|v| v != "0" && !v.is_empty())
}
