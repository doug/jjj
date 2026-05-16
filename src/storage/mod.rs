use crate::error::{JjjError, Result};
use crate::jj::JjClient;
use crate::models::{Event, ProblemStatus, ProjectConfig, SolutionStatus};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

mod critiques;
mod events;
mod milestones;
mod problems;
mod solutions;

/// Write `content` to `path` atomically by writing to a uniquely-named `.tmp`
/// sibling first, then renaming. The temp name includes the process ID and
/// sub-second nanoseconds so concurrent writers cannot clobber each other's
/// temp file.
pub(super) fn atomic_write(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("md.{}.{}.tmp", std::process::id(), nanos));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub const META_BOOKMARK: &str = "jjj";
pub(super) const CONFIG_FILE: &str = "config.toml";
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

    /// Path to the events log file (.jj/jjj-meta/events.jsonl)
    events_path: PathBuf,

    /// JJ client for interacting with the repository
    pub jj_client: JjClient,

    /// Events to append to events.jsonl on the next flush
    pending_events: RefCell<Vec<Event>>,

    /// Long-lived SQLite cache, if present.
    ///
    /// Opened in `new()` from `.jj/jjj.db`. `None` if the DB hasn't been
    /// built yet. Wrapped in `RefCell` to allow lazy lifecycle (e.g., a
    /// caller building the DB after the store exists could install it).
    cache: RefCell<Option<crate::db::Database>>,
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
    if !project.automation.is_empty() {
        base.automation = project.automation.clone();
    }
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

    let frontmatter: T = serde_yml::from_str(yaml_str).map_err(|e| JjjError::FrontmatterParse {
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
    let mut value = serde_yml::to_value(entity)?;
    if let Some(map) = value.as_mapping_mut() {
        map.remove(serde_yml::Value::String(body_field.to_string()));
    }
    let yaml = serde_yml::to_string(&value)?;
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
pub trait Persist: serde::Serialize + serde::de::DeserializeOwned + Sized {
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

    /// Construct the appropriate `EntityNotFound` error for this type.
    fn not_found(id: &str) -> JjjError;

    /// Best-effort sync of this entity to the SQLite cache row + FTS index.
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()>;
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
    fn not_found(id: &str) -> JjjError {
        JjjError::ProblemNotFound(id.to_string())
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_problem_to_cache(db, self)
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
    fn not_found(id: &str) -> JjjError {
        JjjError::SolutionNotFound(id.to_string())
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_solution_to_cache(db, self)
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
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_critique_to_cache(db, self)
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
    fn not_found(id: &str) -> JjjError {
        JjjError::MilestoneNotFound(id.to_string())
    }
    fn sync_to_cache(&self, db: &crate::db::Database) -> Result<()> {
        crate::db::sync::sync_milestone_to_cache(db, self)
    }
}

impl MetadataStore {
    /// Create a new metadata store
    pub fn new(jj_client: JjClient) -> Result<Self> {
        let repo_root = jj_client.repo_root().to_path_buf();
        let meta_path = repo_root.join(".jj").join("jjj-meta");
        let events_path = meta_path.join("events.jsonl");

        let cache = crate::db::sync::open_cache_if_present(&repo_root);

        let store = Self {
            meta_path,
            events_path,
            jj_client,
            pending_events: RefCell::new(Vec::new()),
            cache: RefCell::new(cache),
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
        let content = to_markdown_strip(entity, &body, T::BODY_FIELD)?;
        let path = dir.join(format!("{}.md", entity.id()));
        atomic_write(&path, content.as_bytes())?;

        if let Some(ref db) = *self.cache() {
            if let Err(e) = entity.sync_to_cache(db) {
                eprintln!(
                    "Warning: cache sync failed for {} {}: {}",
                    T::ENTITY_TYPE,
                    entity.id(),
                    e
                );
            }
        }

        Ok(())
    }

    /// List every entity of a given type by walking the directory.
    ///
    /// Files that fail to parse are skipped with a per-file warning; the
    /// rest of the directory is returned. This matches the behavior of the
    /// previous per-entity `list_*` implementations.
    pub(super) fn list<T: Persist>(&self) -> Result<Vec<T>> {
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
            eprintln!(
                "Warning: Failed to load {} {}(s):",
                failures.len(),
                T::ENTITY_TYPE
            );
            for failure in &failures {
                eprintln!("  {}", failure);
            }
        }

        Ok(entities)
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
                eprintln!(
                    "Warning: cache removal failed for {} {}: {}",
                    T::ENTITY_TYPE,
                    id,
                    e
                );
            }
        }
        Ok(())
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
    /// 1. `~/.config/jjj/config.toml` (global user defaults)
    /// 2. `.jj/jjj-meta/config.toml` (project-specific)
    pub fn load_config(&self) -> Result<ProjectConfig> {
        self.ensure_meta_checkout()?;

        let mut config = load_global_config();

        let config_path = self.meta_path.join(CONFIG_FILE);
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let project: ProjectConfig = toml::from_str(&content)?;
            merge_config(&mut config, &project);
        }

        Ok(config)
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

    /// Flush pending events to events.jsonl.
    ///
    /// All pending events are serialized into a single buffer first; only if
    /// serialization and the entire append succeed are the events drained from
    /// `pending_events`. If any step fails, the pending queue is left intact
    /// so the caller (or a later flush) can retry without losing events.
    ///
    /// A trailing serialization error on one event will still abort the flush
    /// — partial writes are not durable here.
    pub fn commit_changes(&self) -> Result<()> {
        use std::io::Write;

        let pending = self.pending_events.borrow();
        if pending.is_empty() {
            return Ok(());
        }

        // Serialize all events first; if any one fails we abort without
        // touching the file or draining the queue.
        let mut buf = String::new();
        for event in pending.iter() {
            match event.to_json_line() {
                Ok(line) => {
                    buf.push_str(&line);
                    buf.push('\n');
                }
                Err(err) => {
                    eprintln!("Warning: failed to serialize event: {}", err);
                    return Err(JjjError::JsonParse(err));
                }
            }
        }
        drop(pending);

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .map_err(JjjError::Io)?;

        file.write_all(buf.as_bytes()).map_err(JjjError::Io)?;
        file.sync_data().map_err(JjjError::Io)?;

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
