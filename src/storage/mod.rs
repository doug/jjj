use crate::error::{JjjError, Result};
use crate::jj::JjClient;
use crate::models::{AutomationConfig, Event, ProblemStatus, ProjectConfig, SolutionStatus};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

mod critiques;
pub mod delta;
mod event_shards;
mod events;
pub mod merge;
mod milestones;
mod problems;
mod solutions;
pub mod sync_state;

/// Write `content` to `path` atomically by writing to a uniquely-named `.tmp`
/// sibling first, then renaming. The temp name includes the process ID and
/// sub-second nanoseconds so concurrent writers cannot clobber each other's
/// temp file. Works for any file type (entity markdown, ranking JSON, …).
pub fn atomic_write(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    // Hidden sibling so the temp never matches an entity/ranking glob.
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "tmp".to_string());
    let tmp = path.with_file_name(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nanos
    ));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// RAII guard for the repo-wide write lock.
///
/// Holds an flocked lock file (`unix`) so concurrent local writers serialize:
/// without it, two processes can both load an entity, mutate it, and save —
/// the second save clobbering the first's update (e.g. a `solution_ids`
/// back-reference). flock is released by the kernel when the process dies, so
/// a crashed holder can't strand the lock the way an O_EXCL pid-file would.
///
/// The guard always decrements the store's re-entrancy depth on drop; it only
/// holds the actual lock file at the outermost level.
struct WriteLockGuard<'a> {
    depth: &'a RefCell<u32>,
    /// `Some` only for the outermost acquisition; dropping it releases flock.
    _file: Option<std::fs::File>,
}

impl Drop for WriteLockGuard<'_> {
    fn drop(&mut self) {
        let mut d = self.depth.borrow_mut();
        *d = d.saturating_sub(1);
        // _file (if any) is closed here, releasing the flock.
    }
}

/// Acquire (or re-enter) the repo-wide write lock for `meta_path`.
fn acquire_write_lock<'a>(
    meta_path: &std::path::Path,
    depth: &'a RefCell<u32>,
) -> Result<WriteLockGuard<'a>> {
    let file = {
        let d = *depth.borrow();
        if d == 0 {
            Some(flock_exclusive(meta_path)?)
        } else {
            None
        }
    };
    *depth.borrow_mut() += 1;
    Ok(WriteLockGuard { depth, _file: file })
}

/// Open `.write.lock` under `meta_path` and take an exclusive advisory lock,
/// blocking until any other process releases it.
#[cfg(unix)]
fn flock_exclusive(meta_path: &std::path::Path) -> Result<std::fs::File> {
    use std::os::unix::io::AsRawFd;
    std::fs::create_dir_all(meta_path)?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(meta_path.join(".write.lock"))?;
    // SAFETY: `file` owns a valid open fd for the duration of the call.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(file)
}

/// Non-unix fallback: open the lock file without advisory locking (Windows is
/// not a supported target; this keeps the code compiling).
#[cfg(not(unix))]
fn flock_exclusive(meta_path: &std::path::Path) -> Result<std::fs::File> {
    std::fs::create_dir_all(meta_path)?;
    Ok(std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(meta_path.join(".write.lock"))?)
}

pub const META_BOOKMARK: &str = "jjj";
pub(super) const CONFIG_FILE: &str = "config.toml";
/// Machine-local automation rules. Deliberately **not** part of the synced
/// metadata set — see [`crate::models::AutomationConfig`].
pub const AUTOMATION_FILE: &str = "automation.toml";
pub(super) const PROBLEMS_DIR: &str = "problems";
pub(super) const SOLUTIONS_DIR: &str = "solutions";
pub(super) const CRITIQUES_DIR: &str = "critiques";
pub(super) const MILESTONES_DIR: &str = "milestones";

/// The core storage abstraction for jjj metadata.
///
/// Manages reading/writing Problems, Solutions, Critiques, and Milestones as
/// markdown files in `.jj/jjj-meta/`. Events are appended to `events.jsonl`.
///
/// The metadata lives entirely outside the working copy — operations here never
/// touch the user's working changes. Sync (push/fetch) is handled separately
/// and only requires a configured sync backend.
///
/// # Cache
///
/// If `.jj/jjj.db` exists at construction time, the store opens a long-lived
/// SQLite connection and uses it for:
/// - Per-entity FTS + table sync on save/delete (see `db::sync`).
/// - Cache-aware list helpers (e.g., [`list_solutions_for_problem_cached`])
///   that do SQL joins instead of walking the filesystem.
///
/// If the DB is missing, all reads fall back to filesystem walks (correct but
/// slower) and saves skip the cache update. Run `jjj db rebuild` to populate.
pub struct MetadataStore {
    /// Path to the metadata directory (.jj/jjj-meta/)
    meta_path: PathBuf,

    /// JJ client for interacting with the repository
    pub jj_client: JjClient,

    /// Events to append to the current writer's shard on the next flush
    pending_events: RefCell<Vec<Event>>,

    /// Long-lived SQLite cache, if present.
    ///
    /// Opened in `new()` from `.jj/jjj.db`. `None` if the DB hasn't been
    /// built yet. Wrapped in `RefCell` to allow lazy lifecycle (e.g., a
    /// caller building the DB after the store exists could install it).
    cache: RefCell<Option<crate::db::Database>>,

    /// Re-entrancy depth for the repo-wide write lock held by
    /// [`Self::with_metadata`]. Only the outermost call acquires the flock;
    /// nested calls just increment this so a single process can't deadlock
    /// against its own lock.
    write_lock_depth: RefCell<u32>,
}

/// Report ignored automation rules found in a synced `config.toml`, once.
///
/// `load_config` runs on nearly every command, so the notice is latched — a
/// per-invocation warning would drown the output it is attached to.
fn warn_legacy_automation(count: usize) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        crate::output::warn(&format!(
            "ignoring {} automation rule(s) in config.toml — automation is \
             machine-local since 0.5.1 because config.toml syncs through the \
             shared bookmark. Run `jjj automation migrate` to move them to \
             automation.toml.",
            count
        ));
    });
}

/// Load the global user config from ~/.config/jjj/config.toml.
fn load_global_config() -> ProjectConfig {
    let config_dir = global_config_dir().join("config.toml");
    if !config_dir.exists() {
        return ProjectConfig::default();
    }
    std::fs::read_to_string(&config_dir)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

/// Get the global jjj config directory (~/.config/jjj/).
fn global_config_dir() -> std::path::PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(xdg).join("jjj");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home).join(".config").join("jjj");
    }
    std::path::PathBuf::from(".config").join("jjj")
}

/// Merge project config on top of global config.
fn merge_config(base: &mut ProjectConfig, project: &ProjectConfig) {
    if project.name.is_some() {
        base.name = project.name.clone();
    }
    if !project.default_reviewers.is_empty() {
        base.default_reviewers = project.default_reviewers.clone();
    }
    if !project.settings.is_empty() {
        base.settings.extend(project.settings.clone());
    }
    base.github = project.github.clone();
    if project.sync.fetch.is_some() {
        base.sync.fetch = project.sync.fetch.clone();
    }
    if project.sync.push.is_some() {
        base.sync.push = project.sync.push.clone();
    }
    if project.sync.track.is_some() {
        base.sync.track = project.sync.track.clone();
    }
    if project.sync.workspace.is_some() {
        base.sync.workspace = project.sync.workspace.clone();
    }
    // NOTE: `automation` is deliberately NOT merged from the project
    // `config.toml`. That file is synced through the shared `jjj` bookmark, so
    // honoring rules from it would let any collaborator run arbitrary shell
    // commands on every clone. Rules come from the machine-local
    // `automation.toml` instead; see `MetadataStore::load_config`.
}

/// FNV-1a over raw file bytes — a cheap, stable-across-runs change fingerprint
/// for [`MetadataStore::list_fs_changed`]. Not cryptographic: it only needs to
/// detect "this file's content differs from what the cache last saw", and a
/// hand-rolled algorithm avoids depending on `DefaultHasher`, whose output is
/// explicitly not guaranteed stable across Rust versions.
fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET_BASIS;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

// =============================================================================
// Markdown Frontmatter Parsing
// =============================================================================

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter<T: serde::de::DeserializeOwned>(content: &str) -> Result<(T, String)> {
    let content = content.trim();

    // Check for frontmatter delimiter
    if !content.starts_with("---") {
        return Err(JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: "File must start with YAML frontmatter (---)".to_string(),
        });
    }

    // Find the closing delimiter
    let rest = &content[3..];
    let end_pos = rest
        .find("\n---")
        .ok_or_else(|| JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: "Missing closing frontmatter delimiter".to_string(),
        })?;

    let yaml_str = &rest[..end_pos].trim();
    let body = rest[end_pos + 4..].trim().to_string();

    let frontmatter: T =
        serde_norway::from_str(yaml_str).map_err(|e| JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: e.to_string(),
        })?;

    Ok((frontmatter, body))
}

/// Add entity context to a FrontmatterParse error
fn add_frontmatter_context(err: JjjError, entity_type: &str, entity_id: &str) -> JjjError {
    match err {
        JjjError::FrontmatterParse { message, .. } => JjjError::FrontmatterParse {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            message,
        },
        other => other,
    }
}

/// Serialize an entity to markdown with YAML frontmatter, stripping the
/// body field from the frontmatter (it lives in the markdown body below).
///
/// `body_field` is the name of the field on `T` that holds the markdown body
/// (`description`, `approach`, `argument`, etc.). The field is removed from
/// the serialized YAML map before rendering, then `body` is appended after
/// the closing `---`. The entity type itself keeps the field for in-memory
/// access and for full JSON output via other code paths.
fn to_markdown_strip<T: serde::Serialize>(
    entity: &T,
    body: &str,
    body_field: &str,
) -> Result<String> {
    let mut value = serde_norway::to_value(entity)?;
    if let Some(map) = value.as_mapping_mut() {
        map.remove(serde_norway::Value::String(body_field.to_string()));
    }
    let yaml = serde_norway::to_string(&value)?;
    Ok(format!("---\n{}---\n\n{}", yaml, body))
}

// =============================================================================
// Persist trait: shared CRUD shape for markdown-backed entities
// =============================================================================

/// Contract for the four markdown-backed entity types (Problem, Solution,
/// Critique, Milestone).
///
/// Implementors expose enough metadata for the generic [`MetadataStore`]
/// load/save/list/delete methods to work polymorphically — directory name,
/// body field name, error variants, and the per-entity cache-sync hook.
///
/// This trait is the seam between the type system and the four
/// near-identical CRUD blocks the storage layer used to have. Concrete
/// `load_problem` / `save_solution` / etc. methods on `MetadataStore` are
/// thin delegates over the generic methods so callers don't need turbofish.
pub trait Persist: serde::Serialize + serde::de::DeserializeOwned + Clone + Sized {
    /// Directory under `.jj/jjj-meta/` where instances of this type live.
    const DIR: &'static str;

    /// Frontmatter field name whose value is stored as the markdown body
    /// (e.g. `description`, `approach`, `argument`).
    const BODY_FIELD: &'static str;

    /// Short type tag used in cache rows, error context, and warnings.
    const ENTITY_TYPE: &'static str;

    /// The entity's stable UUID.
    fn id(&self) -> &str;

    /// Borrow the markdown-body field.
    fn body(&self) -> &str;

    /// Set the markdown-body field, used by `load` after parsing the YAML
    /// frontmatter and reading the body section.
    fn set_body(&mut self, body: String);

    /// Clear derived back-reference fields before the markdown write (Pillar 4).
    ///
    /// These fields (e.g. `Problem::solution_ids`) are populated at read time
    /// from forward references and appear in `--json` output, but must never be
    /// persisted — storing them re-introduces parent-rewrite amplification and
    /// merge conflicts. Default is a no-op; types with derived back-refs
    /// override it. `save` calls this on a clone just before serializing.
    fn clear_derived_fields(&mut self) {}

    /// Construct the appropriate `EntityNotFound` error for this type.
    fn not_found(id: &str) -> JjjError;

    /// Whether the given error is the `not_found` variant for this type.
    ///
    /// Used by [`MetadataStore::load_by_ids`] to skip rows that exist in the
    /// SQLite cache but whose markdown file has been deleted concurrently.
    fn is_not_found(err: &JjjError) -> bool;

    /// Best-effort sync of this entity to the SQLite cache row + FTS index.
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()>;

    /// Whether the SQLite cache is a **faithful** index for this type — i.e. a
    /// DB row reconstructs to an entity byte-identical to a markdown load, so a
    /// DB-primary list (Pillar 2) is safe.
    ///
    /// `true` for every entity type: the critiques table now stores the full
    /// code-location fields (`line_end`, `code_context`, `context_before`,
    /// `context_after`) as of schema v10, so a critique row reconstructs
    /// losslessly — the last type that was forced onto the filesystem walk is
    /// now cache-faithful too.
    const CACHE_FAITHFUL: bool;

    /// List every instance of this type from the SQLite cache, repopulating any
    /// derived back-reference fields so the result matches a markdown load.
    ///
    /// Only invoked by [`MetadataStore::list`] when [`Self::CACHE_FAITHFUL`] is
    /// true and the cache is clean.
    fn list_from_cache(db: &crate::db::Database) -> Result<Vec<Self>>;
}

impl Persist for crate::models::Problem {
    const DIR: &'static str = PROBLEMS_DIR;
    const BODY_FIELD: &'static str = "description";
    const ENTITY_TYPE: &'static str = "problem";

    fn id(&self) -> &str {
        &self.id
    }
    fn body(&self) -> &str {
        &self.description
    }
    fn set_body(&mut self, body: String) {
        self.description = body;
    }
    fn clear_derived_fields(&mut self) {
        self.solution_ids.clear();
    }
    fn not_found(id: &str) -> JjjError {
        JjjError::ProblemNotFound(id.to_string())
    }
    fn is_not_found(err: &JjjError) -> bool {
        matches!(err, JjjError::ProblemNotFound(_))
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_problem_to_cache(db, self)
    }
    const CACHE_FAITHFUL: bool = true;
    fn list_from_cache(db: &crate::db::Database) -> Result<Vec<Self>> {
        // Bare rows only. The derived back-reference `solution_ids` is attached
        // uniformly (DB or filesystem) by `list_problems`/`load_problem`.
        Ok(crate::db::entities::list_problems(db.conn())?)
    }
}

impl Persist for crate::models::Solution {
    const DIR: &'static str = SOLUTIONS_DIR;
    const BODY_FIELD: &'static str = "approach";
    const ENTITY_TYPE: &'static str = "solution";

    fn id(&self) -> &str {
        &self.id
    }
    fn body(&self) -> &str {
        &self.approach
    }
    fn set_body(&mut self, body: String) {
        self.approach = body;
    }
    fn clear_derived_fields(&mut self) {
        self.critique_ids.clear();
    }
    fn not_found(id: &str) -> JjjError {
        JjjError::SolutionNotFound(id.to_string())
    }
    fn is_not_found(err: &JjjError) -> bool {
        matches!(err, JjjError::SolutionNotFound(_))
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_solution_to_cache(db, self)
    }
    const CACHE_FAITHFUL: bool = true;
    fn list_from_cache(db: &crate::db::Database) -> Result<Vec<Self>> {
        // Bare rows only; `critique_ids` is attached by `list_solutions`/
        // `load_solution` (see Problem).
        Ok(crate::db::entities::list_solutions(db.conn())?)
    }
}

impl Persist for crate::models::Critique {
    const DIR: &'static str = CRITIQUES_DIR;
    const BODY_FIELD: &'static str = "argument";
    const ENTITY_TYPE: &'static str = "critique";

    fn id(&self) -> &str {
        &self.id
    }
    fn body(&self) -> &str {
        &self.argument
    }
    fn set_body(&mut self, body: String) {
        self.argument = body;
    }
    fn not_found(id: &str) -> JjjError {
        JjjError::CritiqueNotFound(id.to_string())
    }
    fn is_not_found(err: &JjjError) -> bool {
        matches!(err, JjjError::CritiqueNotFound(_))
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_critique_to_cache(db, self)
    }
    // Faithful as of schema v10: the critiques table stores line_end and the
    // code_context/context_before/context_after JSON arrays, so a row
    // reconstructs byte-identically to a markdown load.
    const CACHE_FAITHFUL: bool = true;
    fn list_from_cache(db: &crate::db::Database) -> Result<Vec<Self>> {
        Ok(crate::db::entities::list_critiques(db.conn())?)
    }
}

impl Persist for crate::models::Milestone {
    const DIR: &'static str = MILESTONES_DIR;
    const BODY_FIELD: &'static str = "description";
    const ENTITY_TYPE: &'static str = "milestone";

    fn id(&self) -> &str {
        &self.id
    }
    fn body(&self) -> &str {
        &self.description
    }
    fn set_body(&mut self, body: String) {
        self.description = body;
    }
    fn clear_derived_fields(&mut self) {
        self.problem_ids.clear();
    }
    fn not_found(id: &str) -> JjjError {
        JjjError::MilestoneNotFound(id.to_string())
    }
    fn is_not_found(err: &JjjError) -> bool {
        matches!(err, JjjError::MilestoneNotFound(_))
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_milestone_to_cache(db, self)
    }
    // problem_ids is stored as a JSON column, so the DB row is faithful as-is.
    const CACHE_FAITHFUL: bool = true;
    fn list_from_cache(db: &crate::db::Database) -> Result<Vec<Self>> {
        Ok(crate::db::entities::list_milestones(db.conn())?)
    }
}

/// Content hashes for the entity files of one type, keyed by entity id.
pub(crate) type ContentHashes = std::collections::HashMap<String, u64>;

/// What a hash-compared directory walk found: entities whose content changed or
/// are new, ids that vanished from disk, and the full current hash map for the
/// caller to persist as the next baseline.
pub(crate) type FsChanges<T> = (Vec<T>, Vec<String>, ContentHashes);

impl MetadataStore {
    /// Create a new metadata store
    pub fn new(jj_client: JjClient) -> Result<Self> {
        let repo_root = jj_client.repo_root().to_path_buf();
        let meta_path = repo_root.join(".jj").join("jjj-meta");

        let cache = crate::db::sync::open_cache_if_present(&repo_root);

        let store = Self {
            meta_path,
            jj_client,
            pending_events: RefCell::new(Vec::new()),
            cache: RefCell::new(cache),
            write_lock_depth: RefCell::new(0),
        };

        Ok(store)
    }

    /// Borrow the SQLite cache, if present.
    ///
    /// Returns `None` when `.jj/jjj.db` was missing at construction time.
    /// Callers that need cache-aware reads should fall back to filesystem
    /// walks in the `None` case.
    pub fn cache(&self) -> std::cell::Ref<'_, Option<crate::db::Database>> {
        self.cache.borrow()
    }

    /// Install (or replace) the SQLite cache after construction.
    ///
    /// Used by `jjj db rebuild` and tests that build the DB after the store
    /// exists.
    pub fn install_cache(&self, db: crate::db::Database) {
        *self.cache.borrow_mut() = Some(db);
    }

    /// Re-open the cache from disk if a DB file is present.
    ///
    /// Call after operations that rebuild the DB file from scratch (e.g.,
    /// `fetch` which deletes and re-creates the .db).
    pub fn reload_cache(&self) {
        let new_cache = crate::db::sync::open_cache_if_present(self.jj_client.repo_root());
        *self.cache.borrow_mut() = new_cache;
    }

    /// Get the path to the metadata directory
    pub fn meta_path(&self) -> &std::path::Path {
        &self.meta_path
    }

    // =========================================================================
    // Generic Persist CRUD
    // =========================================================================
    //
    // These methods are the single implementation of load/save/list for all
    // four entity types. The type-specific wrappers (`load_problem`,
    // `save_solution`, etc.) in `storage/{problems,solutions,critiques,
    // milestones}.rs` are 1-line delegates over these.

    /// Load an entity from disk by ID. Returns `T::not_found(id)` if the
    /// markdown file is absent.
    pub(super) fn load<T: Persist>(&self, id: &str) -> Result<T> {
        self.ensure_meta_checkout()?;

        let path = self.meta_path.join(T::DIR).join(format!("{}.md", id));
        if !path.exists() {
            return Err(T::not_found(id));
        }

        let content = fs::read_to_string(path)?;
        let (mut entity, body): (T, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, T::ENTITY_TYPE, id))?;
        entity.set_body(body);
        Ok(entity)
    }

    /// Persist an entity to disk and best-effort sync to the SQLite cache.
    ///
    /// The markdown is canonical; cache-sync failures emit a warning but do
    /// not fail the save.
    pub(super) fn save<T: Persist>(&self, entity: &T) -> Result<()> {
        self.ensure_meta_checkout()?;

        let dir = self.meta_path.join(T::DIR);
        fs::create_dir_all(&dir)?;

        let body = if entity.body().is_empty() {
            String::new()
        } else {
            format!("{}\n", entity.body())
        };
        // Strip derived back-references so they never reach disk (Pillar 4).
        let mut for_disk = entity.clone();
        for_disk.clear_derived_fields();
        let content = to_markdown_strip(&for_disk, &body, T::BODY_FIELD)?;
        let path = dir.join(format!("{}.md", entity.id()));
        atomic_write(&path, content.as_bytes())?;

        if let Some(ref db) = *self.cache() {
            if let Err(e) = entity.sync_to_cache(db) {
                crate::output::warn(&format!(
                    "cache sync failed for {} {}: {}",
                    T::ENTITY_TYPE,
                    entity.id(),
                    e
                ));
            }
        }

        Ok(())
    }

    /// List every entity of a given type.
    ///
    /// DB-primary (Pillar 2): when the SQLite cache is present and not dirty it
    /// is an always-current index of the canonical markdown — every
    /// save/delete/fetch upserts it synchronously — so list reads are served
    /// from it in O(query) rather than walking and re-parsing the whole entity
    /// directory. A missing DB (never built) or a dirty one (an interrupted
    /// bulk load, which `Database::open` would rebuild to empty) falls back to
    /// the authoritative filesystem walk below.
    ///
    /// In the FS path, files that fail to parse are skipped with a per-file
    /// warning; the rest of the directory is returned.
    pub(super) fn list<T: Persist>(&self) -> Result<Vec<T>> {
        if T::CACHE_FAITHFUL {
            if let Some(ref db) = *self.cache() {
                match crate::db::sync::is_dirty(db) {
                    // Clean cache → serve directly (the O(query) hot path).
                    Ok(false) => return T::list_from_cache(db),
                    // Present-but-dirty cache: an interrupted bulk load or a
                    // schema/version rebuild emptied it, so it can no longer be
                    // trusted (it would report zero entities). Heal it once from
                    // canonical markdown — the Pillar 2 "next command rebuilds"
                    // recovery — then serve from the now-clean cache. If the
                    // rebuild fails or doesn't clear the flag, fall through to the
                    // authoritative filesystem walk below.
                    Ok(true) => {
                        if crate::db::sync::load_from_markdown(db, self).is_ok()
                            && !crate::db::sync::is_dirty(db).unwrap_or(true)
                        {
                            return T::list_from_cache(db);
                        }
                    }
                    // Dirty check itself errored → distrust the cache, use FS.
                    Err(_) => {}
                }
            }
        }
        self.list_fs::<T>()
    }

    /// List every entity of a given type by walking the filesystem directly,
    /// bypassing the DB-primary path.
    ///
    /// This is the authoritative canonical read. [`Self::list`] uses it as the
    /// fallback when the cache is absent/dirty/unfaithful, and the DB rebuild
    /// (`db::load_from_markdown`) MUST use it directly — reading markdown → DB
    /// via the DB-primary [`Self::list`] would read from the very DB being
    /// rebuilt (empty), losing every entity.
    pub(crate) fn list_fs<T: Persist>(&self) -> Result<Vec<T>> {
        self.ensure_meta_checkout()?;

        let dir = self.meta_path.join(T::DIR);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut entities = Vec::new();
        let mut failures = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match self.load::<T>(stem) {
                Ok(entity) => entities.push(entity),
                Err(e) => failures.push(format!("{}: {}", stem, e)),
            }
        }

        if !failures.is_empty() {
            crate::output::warn(&format!(
                "Failed to load {} {}(s):",
                failures.len(),
                T::ENTITY_TYPE
            ));
            for failure in &failures {
                crate::output::warn(&format!("  {}", failure));
            }
        }

        Ok(entities)
    }

    /// Like [`Self::list_fs`], but skips YAML parsing for any file whose
    /// content hash matches `known_hashes` — used by the incremental push
    /// validation reload so an unchanged corpus costs a `read_dir` + hash per
    /// file instead of a full parse + DB upsert per file.
    ///
    /// Returns the entities that need re-upserting into the cache, ids present
    /// in `known_hashes` but no longer on disk (deleted by another tool), and
    /// the current id→hash map for the caller to persist as the new baseline.
    pub(crate) fn list_fs_changed<T: Persist>(
        &self,
        known_hashes: &ContentHashes,
    ) -> Result<FsChanges<T>> {
        self.ensure_meta_checkout()?;

        let dir = self.meta_path.join(T::DIR);
        let mut current_hashes = std::collections::HashMap::new();
        if !dir.exists() {
            let removed = known_hashes.keys().cloned().collect();
            return Ok((Vec::new(), removed, current_hashes));
        }

        let mut changed = Vec::new();
        let mut failures = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    failures.push(format!("{}: {}", stem, e));
                    continue;
                }
            };
            let hash = fnv1a(content.as_bytes());
            current_hashes.insert(stem.to_string(), hash);
            if known_hashes.get(stem) == Some(&hash) {
                continue; // unchanged since the cache last saw it — skip the parse
            }
            match parse_frontmatter::<T>(&content) {
                Ok((mut entity, body)) => {
                    entity.set_body(body);
                    changed.push(entity);
                }
                Err(e) => failures.push(format!(
                    "{}: {}",
                    stem,
                    add_frontmatter_context(e, T::ENTITY_TYPE, stem)
                )),
            }
        }

        if !failures.is_empty() {
            crate::output::warn(&format!(
                "Failed to load {} {}(s):",
                failures.len(),
                T::ENTITY_TYPE
            ));
            for failure in &failures {
                crate::output::warn(&format!("  {}", failure));
            }
        }

        let removed = known_hashes
            .keys()
            .filter(|id| !current_hashes.contains_key(id.as_str()))
            .cloned()
            .collect();

        Ok((changed, removed, current_hashes))
    }

    /// Delete an entity's markdown file and remove it from the cache.
    ///
    /// Returns `T::not_found(id)` if the file doesn't exist. Type-specific
    /// `delete_*` methods perform their own pre-cleanup (orphaning children,
    /// removing back-references) and then call this to do the final removal.
    pub(super) fn delete_file_and_cache<T: Persist>(&self, id: &str) -> Result<()> {
        let path = self.meta_path.join(T::DIR).join(format!("{}.md", id));
        if !path.exists() {
            return Err(T::not_found(id));
        }
        fs::remove_file(path)?;
        if let Some(ref db) = *self.cache() {
            if let Err(e) = crate::db::sync::remove_entity_from_cache(db, T::ENTITY_TYPE, id) {
                crate::output::warn(&format!(
                    "cache removal failed for {} {}: {}",
                    T::ENTITY_TYPE,
                    id,
                    e
                ));
            }
        }
        Ok(())
    }

    /// Load each entity in `ids`, skipping IDs whose markdown file is missing.
    ///
    /// Used by cache-aware query helpers: the SQLite cache lists candidate IDs,
    /// but a concurrent delete may have removed the markdown file in between.
    /// Other load errors (parse failures, IO problems) propagate.
    pub(super) fn load_by_ids<T: Persist>(&self, ids: Vec<String>) -> Result<Vec<T>> {
        let mut out = Vec::with_capacity(ids.len());
        let mut failures = Vec::new();
        for id in ids {
            match self.load::<T>(&id) {
                Ok(entity) => out.push(entity),
                Err(e) if T::is_not_found(&e) => continue,
                // Match the FS-walk path (see `list`): a single malformed file
                // must not abort the whole query — skip it and warn, so one
                // bad entity another user pushed doesn't take down every list.
                Err(e) => failures.push(format!("{}: {}", id, e)),
            }
        }
        if !failures.is_empty() {
            crate::output::warn(&format!(
                "Failed to load {} {}(s) from cache index:",
                failures.len(),
                T::ENTITY_TYPE
            ));
            for failure in &failures {
                crate::output::warn(&format!("  {}", failure));
            }
        }
        Ok(out)
    }

    /// Query the SQLite cache for a list of entity IDs, then materialize them
    /// via [`load_by_ids`]. If the cache is missing, run `fallback` instead.
    ///
    /// Lets the entity-specific query helpers express their cache path as a
    /// single SQL statement and their FS-walk fallback as a closure, without
    /// re-duplicating the surrounding scaffolding on every call.
    pub(super) fn query_ids_or_fallback<T, P, F>(
        &self,
        sql: &str,
        params: P,
        fallback: F,
    ) -> Result<Vec<T>>
    where
        T: Persist,
        P: rusqlite::Params,
        F: FnOnce() -> Result<Vec<T>>,
    {
        // Only trust the cache when it is present AND clean; a dirty cache is
        // empty/stale (see `list`) and would return no ids.
        if let Some(ref db) = *self.cache() {
            if !crate::db::sync::is_dirty(db).unwrap_or(true) {
                let mut stmt = db.conn().prepare(sql)?;
                let ids: Vec<String> = stmt
                    .query_map(params, |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                return self.load_by_ids::<T>(ids);
            }
        }
        fallback()
    }

    /// Batch-derive back-reference id lists keyed by a forward-reference parent
    /// (Pillar 4). DB-clean → one indexed projection `SELECT <parent_col>, id
    /// FROM <table>`; otherwise a one-pass filesystem group-by over the
    /// children's forward refs. Values are sorted by id so the DB and FS paths
    /// return identical results.
    pub(super) fn reverse_ids_batch<T, F>(
        &self,
        projection_sql: &str,
        forward_key: F,
    ) -> Result<std::collections::HashMap<String, Vec<String>>>
    where
        T: Persist,
        F: Fn(&T) -> String,
    {
        use std::collections::HashMap;
        let mut map: HashMap<String, Vec<String>> = HashMap::new();

        let served_by_db = if let Some(ref db) = *self.cache() {
            if crate::db::sync::is_dirty(db).unwrap_or(true) {
                false
            } else {
                let mut stmt = db.conn().prepare(projection_sql)?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let parent: String = row.get(0)?;
                    let child: String = row.get(1)?;
                    map.entry(parent).or_default().push(child);
                }
                true
            }
        } else {
            false
        };

        if !served_by_db {
            for child in self.list_fs::<T>()? {
                map.entry(forward_key(&child))
                    .or_default()
                    .push(child.id().to_string());
            }
        }

        for ids in map.values_mut() {
            ids.sort();
        }
        Ok(map)
    }

    /// Single-parent variant of [`Self::reverse_ids_batch`]: the derived
    /// back-reference id list for one parent. `query_sql` selects the child ids
    /// for a `?1`-bound parent; `matches` is the filesystem-fallback predicate.
    pub(super) fn reverse_ids_for<T, M>(
        &self,
        query_sql: &str,
        parent_id: &str,
        matches: M,
    ) -> Result<Vec<String>>
    where
        T: Persist,
        M: Fn(&T) -> bool,
    {
        let mut ids: Vec<String> = Vec::new();
        let served_by_db = {
            let cache = self.cache();
            match *cache {
                Some(ref db) if !crate::db::sync::is_dirty(db).unwrap_or(true) => {
                    let mut stmt = db.conn().prepare(query_sql)?;
                    let rows = stmt.query_map([parent_id], |row| row.get::<_, String>(0))?;
                    for id in rows {
                        ids.push(id?);
                    }
                    true
                }
                _ => false,
            }
        };
        if !served_by_db {
            ids = self.fs_child_ids::<T>(matches)?;
        }
        ids.sort();
        Ok(ids)
    }

    /// Filesystem-walk child ids matching a forward-ref predicate.
    fn fs_child_ids<T: Persist>(&self, matches: impl Fn(&T) -> bool) -> Result<Vec<String>> {
        Ok(self
            .list_fs::<T>()?
            .into_iter()
            .filter(|c| matches(c))
            .map(|c| c.id().to_string())
            .collect())
    }

    /// Initialize the metadata store (create directory structure)
    pub fn init(&self) -> Result<()> {
        if self.meta_path.join(CONFIG_FILE).exists() {
            return Err(crate::error::JjjError::Validation(
                "jjj is already initialized".to_string(),
            ));
        }
        self.ensure_meta_dirs()?;
        let default_config = ProjectConfig::default();
        self.save_config(&default_config)?;
        Ok(())
    }

    /// Ensure the metadata directory structure exists.
    pub(super) fn ensure_meta_dirs(&self) -> Result<()> {
        fs::create_dir_all(self.meta_path.join(PROBLEMS_DIR))?;
        fs::create_dir_all(self.meta_path.join(SOLUTIONS_DIR))?;
        fs::create_dir_all(self.meta_path.join(CRITIQUES_DIR))?;
        fs::create_dir_all(self.meta_path.join(MILESTONES_DIR))?;
        Ok(())
    }

    pub(super) fn ensure_meta_checkout(&self) -> Result<()> {
        self.ensure_meta_dirs()
    }

    // =========================================================================
    // Config Operations
    // =========================================================================

    /// Load project configuration, merging with global config.
    ///
    /// Load order (later overrides earlier):
    /// 1. `~/.config/jjj/config.toml` (global user defaults; machine-local)
    /// 2. `.jj/jjj-meta/config.toml` (project-specific; **synced**)
    /// 3. `.jj/jjj-meta/automation.toml` (automation rules; machine-local)
    ///
    /// Automation is loaded from its own file on purpose. `config.toml` travels
    /// through the shared `jjj` bookmark and fetch applies the remote copy
    /// wholesale, so a rule read from it would let any collaborator execute
    /// arbitrary shell commands here. Rules found in a legacy `config.toml` are
    /// dropped and reported once per process; `jjj automation migrate` moves
    /// them across.
    pub fn load_config(&self) -> Result<ProjectConfig> {
        self.ensure_meta_checkout()?;

        let mut config = load_global_config();

        let config_path = self.meta_path.join(CONFIG_FILE);
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let project: ProjectConfig = toml::from_str(&content)?;
            if !project.automation.is_empty() {
                warn_legacy_automation(project.automation.len());
            }
            merge_config(&mut config, &project);
        }

        if let Some(local) = self.load_automation_config()? {
            config.automation = local.automation;
        }

        Ok(config)
    }

    /// Read the machine-local `automation.toml`, if it exists.
    pub fn load_automation_config(&self) -> Result<Option<AutomationConfig>> {
        let path = self.meta_path.join(AUTOMATION_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        Ok(Some(toml::from_str(&content)?))
    }

    /// Write the machine-local `automation.toml`.
    pub fn save_automation_config(&self, automation: &AutomationConfig) -> Result<()> {
        self.ensure_meta_checkout()?;
        let path = self.meta_path.join(AUTOMATION_FILE);
        let content = toml::to_string_pretty(automation)?;
        atomic_write(&path, content.as_bytes())?;
        Ok(())
    }

    /// Rules present in the project `config.toml` that are being ignored.
    ///
    /// Used by `jjj automation migrate` / `jjj doctor` to report and relocate
    /// pre-0.5.1 rules.
    pub fn legacy_config_automation(&self) -> Result<Vec<crate::models::AutomationRule>> {
        let config_path = self.meta_path.join(CONFIG_FILE);
        if !config_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(config_path)?;
        let project: ProjectConfig = toml::from_str(&content)?;
        Ok(project.automation)
    }

    /// Rewrite the project `config.toml` with the `automation` key removed.
    ///
    /// Operates on the parsed TOML tree rather than re-serializing
    /// `ProjectConfig`, so unknown/future keys written by a newer jjj survive.
    /// Returns `true` if the file changed.
    pub fn strip_config_automation(&self) -> Result<bool> {
        let config_path = self.meta_path.join(CONFIG_FILE);
        if !config_path.exists() {
            return Ok(false);
        }
        let content = fs::read_to_string(&config_path)?;
        let mut value: toml::Value = toml::from_str(&content)?;
        let removed = value
            .as_table_mut()
            .map(|t| t.remove("automation").is_some())
            .unwrap_or(false);
        if removed {
            atomic_write(&config_path, toml::to_string_pretty(&value)?.as_bytes())?;
        }
        Ok(removed)
    }

    /// Save project configuration
    pub fn save_config(&self, config: &ProjectConfig) -> Result<()> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        let content = toml::to_string_pretty(config)?;
        atomic_write(&config_path, content.as_bytes())?;

        Ok(())
    }

    // =========================================================================
    // High-Level Operations
    // =========================================================================

    /// Check whether a problem can transition to `Solved` status.
    ///
    /// A problem is solvable if:
    /// 1. It has at least one `Approved` solution, **or**
    /// 2. All of its direct subproblems are `Solved`.
    ///
    /// Returns `(can_solve, reason)` where `reason` is non-empty when `can_solve`
    /// is `false` (explaining the blocker) or when it is `true` via subproblem path
    /// (confirming all subproblems are solved). Returns an error if the problem
    /// cannot be found.
    pub fn can_solve_problem(&self, problem_id: &str) -> Result<(bool, String)> {
        let problem = self.load_problem(problem_id)?;

        // Check if already solved
        if problem.status == ProblemStatus::Solved {
            return Ok((false, "Problem is already solved".to_string()));
        }

        // Check for approved solutions
        let solutions = self.list_solutions_for_problem(problem_id)?;
        let has_approved = solutions
            .iter()
            .any(|s| s.status == SolutionStatus::Approved);

        if has_approved {
            return Ok((true, String::new()));
        }

        // Check if all subproblems are solved
        let subproblems = self.list_subproblems(problem_id)?;
        if !subproblems.is_empty() {
            let all_solved = subproblems
                .iter()
                .all(|p| p.status == ProblemStatus::Solved);
            if all_solved {
                return Ok((true, "All subproblems are solved".to_string()));
            }
            return Ok((
                false,
                "Not all subproblems are solved and no approved solution exists".to_string(),
            ));
        }

        Ok((false, "No approved solution exists".to_string()))
    }

    /// Determine whether a solution is eligible for `Approved` status.
    ///
    /// A solution can be approved if:
    /// 1. It is not already in a finalized state (`Approved` or `Withdrawn`), **and**
    /// 2. It has no `Valid` critiques (validated critiques block approval).
    ///
    /// Open critiques do not block approval but produce a warning in the returned
    /// message. Returns `(can_approve, message)` where `message` may describe
    /// blockers or warnings.
    pub fn can_approve_solution(&self, solution_id: &str) -> Result<(bool, String)> {
        let solution = self.load_solution(solution_id)?;

        // Check if already finalized
        if solution.is_finalized() {
            return Ok((false, format!("Solution is already {:?}", solution.status)));
        }

        // Check for valid critiques
        if self.has_valid_critiques(solution_id)? {
            return Ok((
                false,
                "Solution has valid critiques that block approval".to_string(),
            ));
        }

        // Check for open critiques (warning but not blocking)
        let open_critiques = self.list_open_critiques_for_solution(solution_id)?;
        if !open_critiques.is_empty() {
            return Ok((
                true,
                format!(
                    "Warning: {} open critique(s) remain unaddressed",
                    open_critiques.len()
                ),
            ));
        }

        Ok((true, String::new()))
    }

    // =========================================================================
    // Commit Operations
    // =========================================================================

    /// Flush pending events to the current writer's per-user shard (Pillar 3).
    ///
    /// Events append to `events/{author}.jsonl` — a single-writer file, so
    /// parallel pods never contend and the shards never conflict on sync. The
    /// pending queue is drained only after the whole append succeeds, so a
    /// failure leaves it intact for a retry.
    pub fn commit_changes(&self) -> Result<()> {
        let pending = self.pending_events.borrow();
        if pending.is_empty() {
            return Ok(());
        }
        let events: Vec<_> = pending.clone();
        drop(pending);

        self.append_events_to_shards(&events)?;

        // Only drain after a fully successful write.
        self.pending_events.borrow_mut().clear();

        Ok(())
    }

    /// Execute an operation on the metadata store and flush events.
    ///
    /// This is the primary mechanism for all metadata writes. The `operation`
    /// closure runs first; if it succeeds, any events queued via
    /// [`set_pending_event`](MetadataStore::set_pending_event) are appended
    /// to `events.jsonl`.
    ///
    /// If `operation` returns an error, no events are flushed.
    ///
    /// The `_message` parameter is unused at present; it is retained so a
    /// future implementation can annotate the audit log with a batch
    /// description.
    pub fn with_metadata<F, R>(&self, _message: &str, operation: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        // Serialize concurrent local writers for the whole load-modify-save
        // critical section so a back-reference update can't be lost. The guard
        // is re-entrant: a nested with_metadata won't re-acquire (and so can't
        // deadlock against itself).
        let _lock = acquire_write_lock(&self.meta_path, &self.write_lock_depth)?;
        let result = operation()?;
        self.commit_changes()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Problem;

    #[test]
    fn write_lock_is_reentrant_and_decrements() {
        // A nested acquisition must NOT block on the outer one (same process),
        // and depth must return to 0 once both guards drop.
        let tmp = tempfile::tempdir().unwrap();
        let depth = RefCell::new(0u32);
        {
            let _outer = acquire_write_lock(tmp.path(), &depth).unwrap();
            assert_eq!(*depth.borrow(), 1);
            {
                let _inner = acquire_write_lock(tmp.path(), &depth).unwrap();
                assert_eq!(*depth.borrow(), 2);
            }
            assert_eq!(*depth.borrow(), 1);
        }
        assert_eq!(*depth.borrow(), 0);
        // The lock file is reusable after release.
        let _again = acquire_write_lock(tmp.path(), &depth).unwrap();
        assert_eq!(*depth.borrow(), 1);
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
id: p1
title: Test Problem
status: open
priority: medium
created_at: 2024-01-15T10:30:00Z
updated_at: 2024-01-15T10:30:00Z
---

## Description

This is a test problem.

## Context

Some context here.
"#;

        let (problem, body): (Problem, String) = parse_frontmatter(content).unwrap();
        assert_eq!(problem.id, "p1");
        assert_eq!(problem.title, "Test Problem");
        assert!(body.contains("## Description"));
        // The description field defaults to empty when missing from YAML;
        // storage::load_problem assigns it from the body after parsing.
        assert!(problem.description.is_empty());
    }

    #[test]
    fn test_to_markdown_strips_body_field() {
        let mut problem = Problem::new("p1".to_string(), "Test".to_string());
        problem.description = "irrelevant — this lives in the body".to_string();

        let body = "Test description\n";
        let result = to_markdown_strip(&problem, body, "description").unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("id: p1"));
        assert!(result.contains("Test description"));
        // `description` must not appear in the YAML frontmatter — it lives
        // in the body section after the closing `---`.
        let frontmatter_end = result.find("\n---\n\n").unwrap();
        let frontmatter = &result[..frontmatter_end];
        assert!(!frontmatter.contains("description:"));
    }

    #[test]
    fn test_to_markdown_strip_critique_with_reviewer() {
        use crate::models::Critique;

        let mut critique = Critique::new(
            "c1".to_string(),
            "Awaiting review".to_string(),
            "s1".to_string(),
        );
        critique.reviewer = Some("bob".to_string());

        let body = format!("{}\n", critique.argument);
        let markdown = to_markdown_strip(&critique, &body, "argument").unwrap();
        assert!(markdown.contains("reviewer: bob"));
    }
}
