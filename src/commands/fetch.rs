//! Incremental delta-fetch (Pillar 1 + 2 of the agent-swarm scaling design).
//!
//! The legacy fetch enumerated *every* remote file and `jj file show`'d it one
//! at a time against a full `jjj-meta-base/` mirror — O(total corpus) per fetch
//! (~10 min @25K). This path is O(delta):
//!
//! 1. `jj git fetch` brings the remote per-pod bookmarks (`jjj`, `jjj/*`) local.
//! 2. Resolve their head commits ([`JjClient::meta_head_commits`]) — the
//!    fetch-union set (Break #5: each pod is a single-writer ref).
//! 3. For each head `H`, three-way merge against the **true** common ancestor:
//!    `base = GCA(last_synced_rev, H)` ([`JjClient::merge_base`]) — NOT
//!    `last_synced_rev` itself, which on parallel per-pod branches would
//!    reconstruct a base containing our own unpushed edits and silently revert
//!    them (audit 0.1). One full-context [`JjClient::delta_git`] returns every
//!    changed file's whole content on both sides; [`parse_git_diff`]
//!    reconstructs base + remote per file for [`merge_entity_md`].
//! 4. Apply each file delta to the working set and upsert the affected entity
//!    into the SQLite cache incrementally (Pillar 2) — no full DB rebuild on
//!    the hot path.
//!
//! The merge base is a **revision**, not a directory tree: there is no
//! `base/` mirror in this path. `last_synced_rev` advances only on a
//! successful push (see [`SyncState::advance`]); fetch never moves it.
//!
//! Cold start (no `last_synced_rev`, or it is unreachable) falls back to
//! `base = root()`, so every file shows as added and is adopted, followed by a
//! full DB rebuild — the common fresh-clone onboarding path.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::context::CommandContext;
use crate::db::events::{clear_events, insert_event};
use crate::db::{self, Database};
use crate::error::Result;
use crate::storage::delta::{parse_git_diff, DeltaKind, FileDelta};
use crate::storage::merge::{
    has_conflict_markers, merge_entity_md, merge_events_jsonl, merge_ranking_json,
};
use crate::storage::sync_state::SyncState;
use crate::storage::MetadataStore;

/// What a changed path maps to in the working set. Entity files three-way
/// merge and feed the DB; the others have their own union rules and never
/// touch the entity tables.
enum Target {
    /// One of the four entity dirs; carries the DB singular type name.
    Entity { singular: &'static str },
    /// `rankings/{milestone}/{user}.json` — per-file LWW union.
    Ranking,
    /// `config.toml` — last-fetch-wins.
    Config,
    /// `events.jsonl` — append-only line union.
    Events,
    /// Anything else (e.g. the local-only `.sync_state.json`, never synced).
    Skip,
}

/// The four entity dirs paired with their DB singular type name.
const ENTITY_KINDS: &[(&str, &str)] = &[
    ("problems", "problem"),
    ("solutions", "solution"),
    ("critiques", "critique"),
    ("milestones", "milestone"),
];

fn classify(path: &str) -> Target {
    for &(dir, singular) in ENTITY_KINDS {
        if path.starts_with(dir)
            && path.as_bytes().get(dir.len()) == Some(&b'/')
            && path.ends_with(".md")
        {
            return Target::Entity { singular };
        }
    }
    if path.starts_with("rankings/") && path.ends_with(".json") {
        return Target::Ranking;
    }
    match path {
        "config.toml" => Target::Config,
        "events.jsonl" => Target::Events,
        _ => Target::Skip,
    }
}

/// The action a single entity file delta resolves to once the local copy is
/// known. Pure (no I/O) so the three-way decision table is unit-testable
/// without a repo or DB.
#[derive(Debug, PartialEq, Eq)]
enum EntityAction {
    /// Write this merged content (it differs from local).
    Write(String),
    /// Remote deleted it and local matched the base — remove locally.
    Delete,
    /// No change relative to local.
    Keep,
    /// Remote deleted it but local was edited since the base — keep local,
    /// surface a delete/edit conflict.
    DeleteConflict,
}

/// Resolve one reconstructed entity delta against the current local content.
///
/// `delta.base` is the GCA-side content (the true merge ancestor); `delta.remote`
/// is the head-side content. For an add, base is `None`; for a delete, remote is
/// `None`.
fn resolve_entity_delta(delta: &FileDelta, local: Option<&str>) -> Result<EntityAction> {
    match delta.kind {
        DeltaKind::Added | DeltaKind::Modified => {
            let remote = delta.remote.as_deref().unwrap_or_default();
            let merged = match local {
                // No local file → adopt the remote version as-is.
                None => remote.to_string(),
                Some(l) => merge_entity_md(delta.base.as_deref(), l, remote)?,
            };
            if Some(merged.as_str()) == local {
                Ok(EntityAction::Keep)
            } else {
                Ok(EntityAction::Write(merged))
            }
        }
        DeltaKind::Deleted => {
            let base = delta.base.as_deref().unwrap_or_default();
            match local {
                None => Ok(EntityAction::Keep), // already gone locally
                // Untouched since the base → accept the remote deletion.
                Some(l) if l == base => Ok(EntityAction::Delete),
                // Edited locally since the base → keep ours, flag the conflict.
                Some(_) => Ok(EntityAction::DeleteConflict),
            }
        }
    }
}

/// Accumulates everything one fetch needs to report and to apply to the DB.
#[derive(Default)]
struct FetchOutcome {
    /// Entity files whose merged result still carries conflict markers.
    merge_conflicts: Vec<String>,
    /// Entity files the remote deleted but we kept because they were edited.
    delete_conflicts: Vec<String>,
    /// `(singular, id)` of entities to incrementally upsert into the DB.
    changed: BTreeSet<(String, String)>,
    /// `(singular, id)` of entities to remove from the DB.
    deleted: BTreeSet<(String, String)>,
    /// True if `events.jsonl` changed (drives a DB events refresh).
    events_changed: bool,
}

/// Apply one already-parsed file delta to the working set under `meta_path`,
/// recording follow-up work in `outcome`. The DB is touched later, in bulk,
/// from `outcome`.
fn apply_file_delta(meta_path: &Path, delta: &FileDelta, outcome: &mut FetchOutcome) -> Result<()> {
    match classify(&delta.path) {
        Target::Entity { singular, .. } => {
            let local_full = meta_path.join(&delta.path);
            let local = fs::read_to_string(&local_full).ok();
            let id = Path::new(&delta.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();

            match resolve_entity_delta(delta, local.as_deref())? {
                EntityAction::Write(merged) => {
                    if let Some(parent) = local_full.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&local_full, &merged)?;
                    if has_conflict_markers(&merged) {
                        // Conflicted markdown won't parse cleanly; leave the DB
                        // alone until the user resolves it (markdown canonical).
                        outcome.merge_conflicts.push(delta.path.clone());
                    } else {
                        outcome.changed.insert((singular.to_string(), id));
                    }
                }
                EntityAction::Delete => {
                    let _ = fs::remove_file(&local_full);
                    outcome.deleted.insert((singular.to_string(), id));
                }
                EntityAction::DeleteConflict => {
                    outcome.delete_conflicts.push(delta.path.clone());
                }
                EntityAction::Keep => {}
            }
        }
        Target::Ranking => {
            // Per-user file, LWW by updated_at. A deletion is honored only when
            // local still matches the base (same rule as entities).
            let local_full = meta_path.join(&delta.path);
            let local = fs::read_to_string(&local_full).ok();
            match delta.kind {
                DeltaKind::Added | DeltaKind::Modified => {
                    let remote = delta.remote.as_deref().unwrap_or_default();
                    let merged = merge_ranking_json(local.as_deref(), remote);
                    if local.as_deref() != Some(merged.as_str()) {
                        if let Some(parent) = local_full.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(&local_full, &merged)?;
                    }
                }
                DeltaKind::Deleted => {
                    if local.as_deref() == delta.base.as_deref() {
                        let _ = fs::remove_file(&local_full);
                    }
                }
            }
        }
        Target::Config => {
            // config.toml: last-fetch-wins; never delete on a remote removal.
            if let Some(remote) = delta.remote.as_deref() {
                fs::write(meta_path.join("config.toml"), remote)?;
            }
        }
        Target::Events => {
            // events.jsonl: append-only line union with the local log.
            if let Some(remote) = delta.remote.as_deref() {
                let local_full = meta_path.join("events.jsonl");
                let local = fs::read_to_string(&local_full).unwrap_or_default();
                let merged = merge_events_jsonl(&local, remote);
                if merged != local {
                    fs::write(&local_full, &merged)?;
                    outcome.events_changed = true;
                }
            }
        }
        Target::Skip => {}
    }
    Ok(())
}

/// Reload an entity from disk by id and upsert it into the DB cache.
fn upsert_entity(db: &Database, store: &MetadataStore, singular: &str, id: &str) -> Result<()> {
    match singular {
        "problem" => db::sync_problem_to_cache(db, &store.load_problem(id)?),
        "solution" => db::sync_solution_to_cache(db, &store.load_solution(id)?),
        "critique" => db::sync_critique_to_cache(db, &store.load_critique(id)?),
        "milestone" => db::sync_milestone_to_cache(db, &store.load_milestone(id)?),
        other => Err(crate::error::JjjError::Validation(format!(
            "unknown entity type for upsert: {}",
            other
        ))),
    }
}

pub fn execute(ctx: &CommandContext, remote: &str) -> Result<()> {
    let jj_client = ctx.jj();
    let sync_config = ctx.store.load_config().unwrap_or_default().sync;
    let has_git = jj_client.has_git_backend();

    let fetch_cmd = match sync_config.resolve_fetch(has_git) {
        Some(cmd) => cmd,
        None => {
            println!("No sync backend configured and no git backend detected.");
            println!("Configure [sync] fetch in config.toml for custom sync commands.");
            return Ok(());
        }
    };

    let repo_root = jj_client.repo_root().to_path_buf();
    let meta_path = repo_root.join(".jj").join("jjj-meta");
    let db_path = repo_root.join(".jj").join("jjj.db");

    // 1. Fetch from the remote (brings per-pod bookmarks local), then track.
    println!("Fetching from {}...", remote);
    let vars = [("remote", remote), ("bookmark", "jjj")];
    jj_client.execute_sync_command(&fetch_cmd, &vars)?;
    if let Some(track_cmd) = sync_config.resolve_track(has_git) {
        let _ = jj_client.execute_sync_command(&track_cmd, &vars);
    }

    // 2. Resolve the per-pod head commits to merge.
    let heads = jj_client.meta_head_commits()?;
    if heads.is_empty() {
        println!("Fetched from {}.", remote);
        println!("  No jjj bookmark on the remote yet.");
        return Ok(());
    }

    // 3. Decide delta vs cold start. Cold start = no recorded merge base, or a
    //    base that is no longer reachable (GC'd/rewritten).
    let state = SyncState::load(&meta_path);
    let cold_start = match state.last_synced_rev.as_deref() {
        None => true,
        Some(rev) => jj_client.resolve_commit(rev)?.is_none(),
    };

    fs::create_dir_all(&meta_path)?;
    let mut outcome = FetchOutcome::default();

    // 4. Union every head's delta into the working set. For each head the merge
    //    base is the TRUE common ancestor of our last-synced state and that
    //    head — falling back to root() at cold start or when no shared ancestor
    //    is reachable.
    for head in &heads {
        let base = if cold_start {
            "root()".to_string()
        } else {
            let lsr = state.last_synced_rev.as_deref().unwrap();
            jj_client
                .merge_base(lsr, head)?
                .unwrap_or_else(|| "root()".to_string())
        };

        let diff = jj_client.delta_git(&base, head)?;
        let deltas = parse_git_diff(&diff)?;
        for delta in &deltas {
            if let Err(e) = apply_file_delta(&meta_path, delta, &mut outcome) {
                eprintln!("  Warning: merge failed for {}: {}", delta.path, e);
            }
        }
    }

    // 5. Update the working copy so later commands don't hit a stale workspace.
    let ws_prefix = sync_config.workspace.as_deref();
    let _ = jj_client.execute_workspace(ws_prefix, "update-stale", &[]);

    // 6. Reconcile the SQLite cache. Cold start rebuilds in full (the slow
    //    onboarding path); a delta upserts only what changed (Pillar 2).
    let store_after = MetadataStore::new(jj_client.clone())?;
    if cold_start {
        println!("Rebuilding database...");
        if db_path.exists() {
            fs::remove_file(&db_path)?;
            for suffix in ["-wal", "-shm"] {
                let mut sidecar = db_path.clone().into_os_string();
                sidecar.push(suffix);
                let _ = fs::remove_file(std::path::PathBuf::from(sidecar));
            }
        }
        let db = Database::open(&db_path)?;
        db::load_from_markdown(&db, &store_after)?;
    } else if let Some(db) = db::open_cache_if_present(&repo_root) {
        for (singular, id) in &outcome.deleted {
            if let Err(e) = db::remove_entity_from_cache(&db, singular, id) {
                eprintln!("  Warning: DB remove failed for {} {}: {}", singular, id, e);
            }
        }
        for (singular, id) in &outcome.changed {
            if let Err(e) = upsert_entity(&db, &store_after, singular, id) {
                eprintln!("  Warning: DB upsert failed for {} {}: {}", singular, id, e);
            }
        }
        // Events have no incremental shard yet (Pillar 3 / M2): when the log
        // changed, refresh the events table from the merged file so insights
        // and timeline stay correct. Bounded and infrequent relative to the
        // entity hot path.
        if outcome.events_changed {
            let conn = db.conn();
            clear_events(conn)?;
            for event in &store_after.list_events()? {
                insert_event(conn, event)?;
            }
        }
    }

    // 7. Summary + conflict reporting.
    let new_solutions = outcome
        .changed
        .iter()
        .filter(|(s, _)| s == "solution")
        .count();
    let new_critiques = outcome
        .changed
        .iter()
        .filter(|(s, _)| s == "critique")
        .count();

    println!("Fetched from {}.", remote);
    if cold_start {
        println!("  Cold start — full reconcile + DB rebuild.");
    }
    if new_solutions > 0 {
        println!("  {} solution(s) added/updated", new_solutions);
    }
    if new_critiques > 0 {
        println!("  {} critique(s) added/updated", new_critiques);
    }
    if !cold_start && outcome.changed.is_empty() && outcome.deleted.is_empty() {
        println!("  No new jjj changes.");
    }

    if !outcome.merge_conflicts.is_empty() {
        eprintln!(
            "\nMerge conflicts in {} file(s) — both sides edited the same body:",
            outcome.merge_conflicts.len()
        );
        for path in &outcome.merge_conflicts {
            eprintln!("  {}", path);
        }
        eprintln!("Open each file, resolve the <<<<<<< / >>>>>>> markers, then save.");
    }

    if !outcome.delete_conflicts.is_empty() {
        eprintln!(
            "\nDelete/edit conflict in {} file(s) — deleted on the remote but edited locally (kept your copy):",
            outcome.delete_conflicts.len()
        );
        for path in &outcome.delete_conflicts {
            eprintln!("  {}", path);
        }
        eprintln!("Re-delete with `jjj <type> delete` to accept the removal, or push to keep it.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(status: &str, title: &str, updated: &str) -> String {
        format!(
            "---\nid: '01'\nstatus: {}\ntitle: {}\ncreated_at: 2026-05-01T00:00:00Z\nupdated_at: {}\n---\n\nbody\n",
            status, title, updated
        )
    }

    fn modified(base: &str, remote: &str) -> FileDelta {
        FileDelta {
            path: "problems/01.md".to_string(),
            kind: DeltaKind::Modified,
            base: Some(base.to_string()),
            remote: Some(remote.to_string()),
        }
    }

    #[test]
    fn classify_routes_each_family() {
        assert!(matches!(
            classify("problems/01.md"),
            Target::Entity {
                singular: "problem",
                ..
            }
        ));
        assert!(matches!(
            classify("milestones/x.md"),
            Target::Entity {
                singular: "milestone",
                ..
            }
        ));
        assert!(matches!(classify("rankings/m/u.json"), Target::Ranking));
        assert!(matches!(classify("config.toml"), Target::Config));
        assert!(matches!(classify("events.jsonl"), Target::Events));
        assert!(matches!(classify(".sync_state.json"), Target::Skip));
        // A problems-prefixed path that isn't actually under problems/.
        assert!(matches!(classify("problems_01.md"), Target::Skip));
    }

    #[test]
    fn added_with_no_local_is_adopted() {
        let remote = md("open", "New", "2026-05-01T00:00:00Z");
        let delta = FileDelta {
            path: "problems/01.md".to_string(),
            kind: DeltaKind::Added,
            base: None,
            remote: Some(remote.clone()),
        };
        assert_eq!(
            resolve_entity_delta(&delta, None).unwrap(),
            EntityAction::Write(remote)
        );
    }

    #[test]
    fn modified_only_remote_changed_takes_remote() {
        let base = md("open", "Orig", "2026-05-01T00:00:00Z");
        let remote = md("open", "Remote Title", "2026-05-03T00:00:00Z");
        // local still equals base → adopt remote.
        let action = resolve_entity_delta(&modified(&base, &remote), Some(&base)).unwrap();
        match action {
            EntityAction::Write(m) => assert!(m.contains("Remote Title")),
            other => panic!("expected Write, got {:?}", other),
        }
    }

    /// The keystone no-data-loss case: a local-only edit (open→in_progress) must
    /// survive when the remote changed a *different* field, given the correct
    /// (GCA) base.
    #[test]
    fn local_only_edit_survives_with_true_base() {
        let base = md("open", "Orig", "2026-05-01T00:00:00Z");
        let local = md("in_progress", "Orig", "2026-05-02T00:00:00Z");
        let remote = md("open", "Remote Title", "2026-05-03T00:00:00Z");
        let action = resolve_entity_delta(&modified(&base, &remote), Some(&local)).unwrap();
        match action {
            EntityAction::Write(m) => {
                assert!(m.contains("status: in_progress"), "local edit lost:\n{m}");
                assert!(m.contains("Remote Title"), "remote edit lost:\n{m}");
            }
            other => panic!("expected Write, got {:?}", other),
        }
    }

    #[test]
    fn identical_remote_is_keep() {
        let base = md("open", "Orig", "2026-05-01T00:00:00Z");
        let local = md("in_progress", "Orig", "2026-05-02T00:00:00Z");
        // remote equals local exactly → nothing to do.
        let action = resolve_entity_delta(&modified(&base, &local), Some(&local)).unwrap();
        assert_eq!(action, EntityAction::Keep);
    }

    #[test]
    fn delete_when_local_matches_base() {
        let base = md("open", "Doomed", "2026-05-01T00:00:00Z");
        let delta = FileDelta {
            path: "problems/01.md".to_string(),
            kind: DeltaKind::Deleted,
            base: Some(base.clone()),
            remote: None,
        };
        assert_eq!(
            resolve_entity_delta(&delta, Some(&base)).unwrap(),
            EntityAction::Delete
        );
        // Already gone locally → keep (idempotent).
        assert_eq!(
            resolve_entity_delta(&delta, None).unwrap(),
            EntityAction::Keep
        );
    }

    #[test]
    fn delete_when_local_edited_is_conflict() {
        let base = md("open", "Orig", "2026-05-01T00:00:00Z");
        let local = md("in_progress", "Orig", "2026-05-02T00:00:00Z");
        let delta = FileDelta {
            path: "problems/01.md".to_string(),
            kind: DeltaKind::Deleted,
            base: Some(base),
            remote: None,
        };
        assert_eq!(
            resolve_entity_delta(&delta, Some(&local)).unwrap(),
            EntityAction::DeleteConflict
        );
    }

    #[test]
    fn apply_writes_and_records_changed_entity() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path();
        let remote = md("open", "New", "2026-05-01T00:00:00Z");
        let delta = FileDelta {
            path: "problems/01.md".to_string(),
            kind: DeltaKind::Added,
            base: None,
            remote: Some(remote),
        };
        let mut outcome = FetchOutcome::default();
        apply_file_delta(meta, &delta, &mut outcome).unwrap();
        assert!(meta.join("problems/01.md").exists());
        assert!(outcome
            .changed
            .contains(&("problem".to_string(), "01".to_string())));
    }

    #[test]
    fn apply_conflict_markers_skip_db_and_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path();
        // Both sides edited the body differently → conflict markers.
        let base = md("open", "Orig", "2026-05-01T00:00:00Z");
        let local = base.replace("body", "alice body");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::write(meta.join("problems/01.md"), &local).unwrap();
        let remote = base.replace("body", "bob body");
        let delta = modified(&base, &remote);

        let mut outcome = FetchOutcome::default();
        apply_file_delta(meta, &delta, &mut outcome).unwrap();

        let written = fs::read_to_string(meta.join("problems/01.md")).unwrap();
        assert!(written.contains("<<<<<<< local"));
        assert_eq!(outcome.merge_conflicts, vec!["problems/01.md".to_string()]);
        // Conflicted entity must NOT be queued for a DB upsert.
        assert!(outcome.changed.is_empty());
    }

    #[test]
    fn apply_delete_removes_file_and_records_deletion() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path();
        let base = md("open", "Doomed", "2026-05-01T00:00:00Z");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::write(meta.join("problems/01.md"), &base).unwrap();
        let delta = FileDelta {
            path: "problems/01.md".to_string(),
            kind: DeltaKind::Deleted,
            base: Some(base),
            remote: None,
        };
        let mut outcome = FetchOutcome::default();
        apply_file_delta(meta, &delta, &mut outcome).unwrap();
        assert!(!meta.join("problems/01.md").exists());
        assert!(outcome
            .deleted
            .contains(&("problem".to_string(), "01".to_string())));
    }

    #[test]
    fn apply_events_union_sets_changed_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path();
        fs::write(
            meta.join("events.jsonl"),
            "{\"when\":\"2026-05-01T00:00:00Z\",\"by\":\"a\"}\n",
        )
        .unwrap();
        let delta = FileDelta {
            path: "events.jsonl".to_string(),
            kind: DeltaKind::Modified,
            base: Some("{\"when\":\"2026-05-01T00:00:00Z\",\"by\":\"a\"}\n".to_string()),
            remote: Some("{\"when\":\"2026-05-02T00:00:00Z\",\"by\":\"b\"}\n".to_string()),
        };
        let mut outcome = FetchOutcome::default();
        apply_file_delta(meta, &delta, &mut outcome).unwrap();
        assert!(outcome.events_changed);
        let merged = fs::read_to_string(meta.join("events.jsonl")).unwrap();
        assert_eq!(merged.lines().count(), 2);
    }
}
