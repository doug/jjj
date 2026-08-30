//! `jjj conflicts` + `jjj resolve` — structured conflict discovery and
//! non-interactive resolution (coordination layer).
//!
//! A concurrent fetch can leave an entity body wrapped in `<<<<<<< / >>>>>>>`
//! markers when two pods edited it from the same base. Hand-editing markers is
//! fine for a human but a non-starter for an agent swarm. These two commands
//! make the conflict set machine-readable (`jjj conflicts --json`) and let an
//! agent resolve one deterministically by choosing a side
//! (`jjj resolve <id> --ours|--theirs`), which strips the markers, refreshes the
//! cache, and records a `conflict_resolved` event in the audit log.

use crate::context::CommandContext;
use crate::error::{JjjError, Result};
use crate::models::{Event, EventType};
use crate::storage::merge::{has_conflict_markers, resolve_conflict_markers, ConflictSide};
use crate::storage::MetadataStore;
use std::fs;

/// Entity directories scanned for conflicts, paired with the singular type name.
/// Event shards and rankings can't conflict (append-only / last-writer merges),
/// so only the entity dirs are relevant.
const ENTITY_DIRS: &[(&str, &str)] = &[
    ("problems", "problem"),
    ("solutions", "solution"),
    ("critiques", "critique"),
    ("milestones", "milestone"),
    ("findings", "finding"),
];

/// One conflicted entity file awaiting resolution.
#[derive(Debug, serde::Serialize)]
pub struct ConflictInfo {
    /// Singular entity type (`problem`, `solution`, `critique`, `milestone`,
    /// `finding`).
    pub entity_type: String,
    /// The entity's UUID (the file stem).
    pub id: String,
    /// Best-effort title from the (cleanly merged) frontmatter.
    pub title: String,
    /// Path relative to the meta dir, e.g. `problems/{uuid}.md`.
    pub path: String,
}

/// Scan every entity markdown file for unresolved conflict markers. Frontmatter
/// is merged cleanly (only the body is ever wrapped), so the id and title are
/// still readable off a conflicted file.
pub fn scan(store: &MetadataStore) -> Result<Vec<ConflictInfo>> {
    let meta = store.meta_path();
    let mut out = Vec::new();
    for (dir, singular) in ENTITY_DIRS {
        let entries = match fs::read_dir(meta.join(dir)) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("md") {
                continue;
            }
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if !has_conflict_markers(&content) {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let file = path.file_name().unwrap().to_string_lossy();
            out.push(ConflictInfo {
                entity_type: singular.to_string(),
                id,
                title: frontmatter_title(&content).unwrap_or_else(|| "(untitled)".to_string()),
                path: format!("{}/{}", dir, file),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Pull the `title:` value out of YAML frontmatter without a full parse — the
/// body may contain conflict markers that would trip a strict deserialize.
fn frontmatter_title(content: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some(rest) = line.strip_prefix("title:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// `jjj conflicts [--json]` — list entities with unresolved conflict markers.
pub fn list(ctx: &CommandContext, json: bool) -> Result<()> {
    let conflicts = scan(&ctx.store)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&conflicts).map_err(JjjError::JsonParse)?
        );
        return Ok(());
    }

    if conflicts.is_empty() {
        println!("No unresolved conflicts.");
        return Ok(());
    }

    println!(
        "{} unresolved conflict(s) — resolve each with `jjj resolve <id> --ours|--theirs`:",
        conflicts.len()
    );
    for c in &conflicts {
        println!(
            "  {} {}  {}",
            c.entity_type,
            crate::display::short_id(&c.id),
            c.title
        );
    }
    Ok(())
}

/// `jjj resolve <id> --ours|--theirs [--rationale]` — collapse a conflicted
/// entity to one side, refresh the cache, and log a `conflict_resolved` event.
pub fn resolve(
    ctx: &CommandContext,
    id_input: &str,
    side: ConflictSide,
    rationale: Option<&str>,
) -> Result<()> {
    let store = &ctx.store;
    let conflicts = scan(store)?;

    // Match by exact id or unambiguous prefix (min 6 chars, like other refs).
    let matches: Vec<&ConflictInfo> = conflicts
        .iter()
        .filter(|c| c.id == id_input || (id_input.len() >= 6 && c.id.starts_with(id_input)))
        .collect();

    let target = match matches.as_slice() {
        [one] => *one,
        [] => {
            return Err(JjjError::Validation(format!(
                "No unresolved conflict matches '{}'. Run `jjj conflicts` to list them.",
                id_input
            )));
        }
        many => {
            let ids = many
                .iter()
                .map(|c| crate::display::short_id(&c.id))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(JjjError::Validation(format!(
                "'{}' is ambiguous — matches {}. Use a longer prefix.",
                id_input, ids
            )));
        }
    };

    let abs_path = store.meta_path().join(&target.path);
    let content = fs::read_to_string(&abs_path).map_err(JjjError::Io)?;
    let resolved = resolve_conflict_markers(&content, side);

    if has_conflict_markers(&resolved) {
        return Err(JjjError::Validation(format!(
            "{} still has conflict markers after resolution — file may be malformed.",
            target.path
        )));
    }

    let side_label = match side {
        ConflictSide::Local => "ours (local)",
        ConflictSide::Remote => "theirs (remote)",
    };
    let user = store
        .get_current_user()
        .unwrap_or_else(|_| "unknown".to_string());
    let mut event = Event::new(EventType::ConflictResolved, target.id.clone(), user);
    event = event.with_rationale(
        rationale
            .map(|r| r.to_string())
            .unwrap_or_else(|| format!("resolved to {}", side_label)),
    );

    let id = target.id.clone();
    let singular = target.entity_type.clone();
    store.with_metadata(&format!("Resolve conflict {}", id), || {
        crate::storage::atomic_write(&abs_path, resolved.as_bytes()).map_err(JjjError::Io)?;
        store.set_pending_event(event.clone());
        Ok(())
    })?;

    // Refresh the SQLite cache for just this entity (if a cache exists).
    if let Some(db) = crate::db::open_cache_if_present(ctx.jj().repo_root()) {
        if let Err(e) = upsert_resolved(&db, store, &singular, &id) {
            eprintln!(
                "  Warning: cache refresh failed for {} {}: {}",
                singular, id, e
            );
        }
    }

    println!(
        "Resolved {} {} to {}.",
        singular,
        crate::display::short_id(&id),
        side_label
    );
    Ok(())
}

/// Reload a just-resolved entity from disk and upsert it into the DB cache.
fn upsert_resolved(
    db: &crate::db::Database,
    store: &MetadataStore,
    singular: &str,
    id: &str,
) -> Result<()> {
    use crate::db;
    match singular {
        "problem" => db::sync_problem_to_cache(db, &store.load_problem(id)?),
        "solution" => db::sync_solution_to_cache(db, &store.load_solution(id)?),
        "critique" => db::sync_critique_to_cache(db, &store.load_critique(id)?),
        "milestone" => db::sync_milestone_to_cache(db, &store.load_milestone(id)?),
        "finding" => db::sync_finding_to_cache(db, &store.load_finding(id)?),
        other => Err(JjjError::Validation(format!(
            "unknown entity type for cache refresh: {}",
            other
        ))),
    }
}
