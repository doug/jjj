use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::storage::merge::{
    merge_entity_md, merge_events_jsonl, merge_ranking_json, read_base_file, remove_base_file,
    write_base_file,
};
use crate::storage::MetadataStore;

/// Merge a single remote entity file into the local copy using a three-way
/// merge against the base snapshot.
///
/// Returns true when the merged result differs from the existing local file
/// (i.e. the file changed as a result of the fetch).
fn merge_entity_into_local(
    base_path: &Path,
    local_path: &Path,
    relative: &Path,
    remote_content: &str,
) -> Result<bool> {
    let local_full = local_path.join(relative);
    let local_existing = fs::read_to_string(&local_full).ok();
    let base_existing = read_base_file(base_path, relative);

    let merged = match local_existing.as_deref() {
        // No local file → adopt the remote version as-is.
        None => remote_content.to_string(),
        Some(local) => merge_entity_md(base_existing.as_deref(), local, remote_content)?,
    };

    let changed = local_existing.as_deref() != Some(merged.as_str());

    if changed {
        if let Some(parent) = local_full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local_full, &merged)?;
    }

    // Advance the ancestor for the next fetch to the remote content we just
    // observed — NOT the merged result. The merge base must be the last state
    // common to both sides; that is the raw remote, since local == remote plus
    // our local-only edits. (Writing the merged result here would make those
    // local-only edits look like part of the base and silently lose them on a
    // subsequent fetch.)
    write_base_file(base_path, relative, remote_content)?;
    Ok(changed)
}

/// Reconcile entities deleted on the remote for a single directory.
///
/// The merge loop only visits files the remote still has. A file present
/// locally but absent from `remote_files` was either deleted on the remote or
/// created locally since the last sync. We tell them apart via the base
/// snapshot:
/// - in base and unchanged locally → the remote deleted it; remove it locally.
/// - in base but edited locally → delete/edit conflict; keep local, report it.
/// - not in base → purely local creation not yet pushed; keep it.
fn reconcile_remote_deletions(
    meta_path: &Path,
    base_path: &Path,
    dir: &str,
    remote_files: &HashSet<PathBuf>,
    delete_conflicts: &mut Vec<String>,
) -> Result<()> {
    let local_dir = meta_path.join(dir);
    if !local_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&local_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let relative = Path::new(dir).join(entry.file_name());
        if remote_files.contains(&relative) {
            continue; // still on the remote — already handled by the merge loop
        }
        let base_existing = match read_base_file(base_path, &relative) {
            Some(b) => b,
            None => continue, // never shared → purely local, keep it
        };
        let local_existing = fs::read_to_string(&path).unwrap_or_default();
        if local_existing == base_existing {
            // Remote deleted it and we didn't touch it → delete locally.
            let _ = fs::remove_file(&path);
            remove_base_file(base_path, &relative)?;
        } else {
            // We edited a file the remote deleted → conflict; keep ours.
            delete_conflicts.push(relative.display().to_string());
        }
    }
    Ok(())
}

pub fn execute(ctx: &CommandContext, remote: &str) -> Result<()> {
    let jj_client = ctx.jj();
    let sync_config = ctx.store.load_config().unwrap_or_default().sync;
    let has_git = jj_client.has_git_backend();

    // Resolve sync commands: explicit config > git default > skip
    let fetch_cmd = match sync_config.resolve_fetch(has_git) {
        Some(cmd) => cmd,
        None => {
            println!("No sync backend configured and no git backend detected.");
            println!("Configure [sync] fetch in config.toml for custom sync commands.");
            return Ok(());
        }
    };

    // NOTE: we deliberately do NOT dump the DB to markdown here. Markdown is
    // canonical (entity saves write it first), and `Database::open` rebuilds a
    // dirty/interrupted DB to *empty* — dumping that over the markdown would
    // wipe it. The DB is rebuilt from markdown at the end of the fetch anyway.
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");

    // Snapshot counts before fetch
    let solutions_before = ctx.store.list_solutions().unwrap_or_default().len();
    let critiques_before = ctx.store.list_critiques().unwrap_or_default().len();

    // 1. Fetch from remote using configured or default command.
    println!("Fetching from {}...", remote);
    let vars = [("remote", remote), ("bookmark", "jjj")];
    jj_client.execute_sync_command(&fetch_cmd, &vars)?;

    // Track the jjj bookmark from the remote if it exists
    if let Some(track_cmd) = sync_config.resolve_track(has_git) {
        let _ = jj_client.execute_sync_command(&track_cmd, &vars);
    }

    // 2. Extract updated files from the fetched jjj bookmark and three-way
    //    merge them into the local working set.
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    let base_path = ctx.store.base_path();
    let mut merge_conflicts: Vec<String> = Vec::new();
    let mut delete_conflicts: Vec<String> = Vec::new();
    if jj_client.bookmark_exists("jjj")? {
        fs::create_dir_all(&meta_path)?;
        fs::create_dir_all(&base_path)?;

        for dir in &["problems", "solutions", "critiques", "milestones"] {
            fs::create_dir_all(meta_path.join(dir))?;
            let mut remote_files: HashSet<PathBuf> = HashSet::new();
            if let Ok(listing) =
                jj_client.execute(&["file", "list", "-r", "jjj", &format!("{}/", dir)])
            {
                for file_path in listing.lines().filter(|l| !l.trim().is_empty()) {
                    let remote = match jj_client.execute(&["file", "show", "-r", "jjj", file_path])
                    {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let relative = Path::new(file_path);
                    remote_files.insert(relative.to_path_buf());
                    match merge_entity_into_local(&base_path, &meta_path, relative, &remote) {
                        Ok(_) => {
                            // Detect any conflict markers we emitted so we can
                            // surface them to the user once at the end.
                            let local_full = meta_path.join(relative);
                            if let Ok(content) = fs::read_to_string(&local_full) {
                                if content.contains("<<<<<<< local") {
                                    merge_conflicts.push(file_path.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  Warning: merge failed for {}: {}", file_path, e);
                        }
                    }
                }
            }

            // Remove (or flag) entities the remote deleted. The per-file base
            // is advanced inside merge_entity_into_local; this is the only
            // place the base learns about deletions now that the wholesale
            // snapshot_base pass is gone.
            reconcile_remote_deletions(
                &meta_path,
                &base_path,
                dir,
                &remote_files,
                &mut delete_conflicts,
            )?;
        }

        // rankings/{milestone}/{user}.json: per-file last-writer-wins union.
        // Each file is owned by one user, so there is no cross-user conflict;
        // we adopt any remote file we don't have and keep the newer of the two
        // when both exist. No base snapshot needed (this is not a three-way
        // merge).
        if let Ok(listing) = jj_client.execute(&["file", "list", "-r", "jjj", "rankings/"]) {
            for file_path in listing.lines().filter(|l| !l.trim().is_empty()) {
                let remote = match jj_client.execute(&["file", "show", "-r", "jjj", file_path]) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let local_full = meta_path.join(file_path);
                let local = fs::read_to_string(&local_full).ok();
                let merged = merge_ranking_json(local.as_deref(), &remote);
                if local.as_deref() != Some(merged.as_str()) {
                    if let Some(parent) = local_full.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&local_full, &merged)?;
                }
            }
        }

        // config.toml: last-fetch-wins (rarely a hot conflict surface).
        if let Ok(content) = jj_client.execute(&["file", "show", "-r", "jjj", "config.toml"]) {
            let _ = fs::write(meta_path.join("config.toml"), &content);
        }

        // events.jsonl: append-only line union. Base not needed.
        if let Ok(remote_events) = jj_client.execute(&["file", "show", "-r", "jjj", "events.jsonl"])
        {
            let local_events_path = meta_path.join("events.jsonl");
            let local_events = fs::read_to_string(&local_events_path).unwrap_or_default();
            let merged = merge_events_jsonl(&local_events, &remote_events);
            if merged != local_events {
                let _ = fs::write(&local_events_path, &merged);
            }
        }

        // NOTE: the per-file ancestor is advanced inside
        // merge_entity_into_local (to the raw remote content) and pruned in
        // reconcile_remote_deletions. We deliberately do NOT re-snapshot the
        // merged working set here — doing so would fold local-only edits into
        // the base and silently drop them on the next fetch.
    }

    // 3. Update working copy to avoid stale workspace errors
    let ws_prefix = sync_config.workspace.as_deref();
    let _ = jj_client.execute_workspace(ws_prefix, "update-stale", &[]);

    // 4. Rebuild database from updated markdown files
    println!("Rebuilding database...");
    if db_path.exists() {
        fs::remove_file(&db_path)?;
        // Also drop the WAL sidecars so a fresh DB doesn't inherit a stale
        // write-ahead log from the old inode.
        for suffix in ["-wal", "-shm"] {
            let mut sidecar = db_path.clone().into_os_string();
            sidecar.push(suffix);
            let _ = fs::remove_file(std::path::PathBuf::from(sidecar));
        }
    }
    let db = Database::open(&db_path)?;
    let store_after = MetadataStore::new(jj_client.clone())?;
    db::load_from_markdown(&db, &store_after)?;

    // 4. Show summary - store_after already created above
    let solutions_after = store_after.list_solutions().unwrap_or_default().len();
    let critiques_after = store_after.list_critiques().unwrap_or_default().len();

    let new_solutions = solutions_after.saturating_sub(solutions_before);
    let new_critiques = critiques_after.saturating_sub(critiques_before);

    println!("Fetched from {}.", remote);
    if new_solutions > 0 {
        println!("  {} new solution(s)", new_solutions);
    }
    if new_critiques > 0 {
        println!("  {} new critique(s)", new_critiques);
    }
    if new_solutions == 0 && new_critiques == 0 {
        println!("  No new jjj changes.");
    }

    if !merge_conflicts.is_empty() {
        eprintln!(
            "\nMerge conflicts in {} file(s) — both sides edited the same body:",
            merge_conflicts.len()
        );
        for path in &merge_conflicts {
            eprintln!("  {}", path);
        }
        eprintln!("Open each file, resolve the <<<<<<< / >>>>>>> markers, then save.");
    }

    if !delete_conflicts.is_empty() {
        eprintln!(
            "\nDelete/edit conflict in {} file(s) — deleted on the remote but edited locally (kept your copy):",
            delete_conflicts.len()
        );
        for path in &delete_conflicts {
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

    /// Regression for the critical silent-data-loss bug: after a fetch the
    /// per-file merge ancestor must advance to the RAW REMOTE content, not the
    /// merged-local result. Otherwise a local-only edit looks like part of the
    /// base on the next fetch and a divergent remote silently reverts it.
    #[test]
    fn fetch_base_advances_to_remote_so_local_edit_survives_next_fetch() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        let rel = Path::new("problems/01.md");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(&base).unwrap();

        // Shared starting point (status: open) recorded as base + local.
        let initial = md("open", "Original", "2026-05-01T00:00:00Z");
        fs::write(meta.join(rel), &initial).unwrap();
        write_base_file(&base, rel, &initial).unwrap();

        // Local-only edit: open -> in_progress. Never pushed.
        let local_v2 = md("in_progress", "Original", "2026-05-02T00:00:00Z");
        fs::write(meta.join(rel), &local_v2).unwrap();

        // Round 1 fetch: remote is still the initial (open) state.
        merge_entity_into_local(&base, &meta, rel, &initial).unwrap();
        let after1 = fs::read_to_string(meta.join(rel)).unwrap();
        assert!(
            after1.contains("status: in_progress"),
            "round 1 must keep the local in_progress edit:\n{after1}"
        );

        // Round 2 fetch: remote edits a DIFFERENT field (title), status stays
        // open with a later timestamp.
        let remote_v2 = md("open", "Remote Title", "2026-05-03T00:00:00Z");
        merge_entity_into_local(&base, &meta, rel, &remote_v2).unwrap();
        let after2 = fs::read_to_string(meta.join(rel)).unwrap();

        assert!(
            after2.contains("status: in_progress"),
            "DATA LOSS: the local in_progress edit was silently reverted:\n{after2}"
        );
        assert!(
            after2.contains("Remote Title"),
            "the remote's title edit should merge in:\n{after2}"
        );
    }

    #[test]
    fn fetch_adopts_remote_when_no_local_file() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        let rel = Path::new("problems/01.md");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(&base).unwrap();

        let remote = md("open", "Brand New", "2026-05-01T00:00:00Z");
        let changed = merge_entity_into_local(&base, &meta, rel, &remote).unwrap();
        assert!(changed);
        let local = fs::read_to_string(meta.join(rel)).unwrap();
        assert!(local.contains("Brand New"));
        // Base advanced so a re-fetch of the same content is a no-op.
        assert_eq!(read_base_file(&base, rel).as_deref(), Some(remote.as_str()));
    }

    #[test]
    fn reconcile_deletes_remote_removed_file_when_local_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        let rel = Path::new("problems/01.md");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(base.join("problems")).unwrap();

        let content = md("open", "Doomed", "2026-05-01T00:00:00Z");
        fs::write(meta.join(rel), &content).unwrap();
        write_base_file(&base, rel, &content).unwrap();

        // Remote listing is empty for this dir → the file was deleted remotely.
        let remote_files: HashSet<PathBuf> = HashSet::new();
        let mut conflicts = Vec::new();
        reconcile_remote_deletions(&meta, &base, "problems", &remote_files, &mut conflicts)
            .unwrap();

        assert!(
            !meta.join(rel).exists(),
            "remote-deleted file should be removed locally"
        );
        assert!(
            read_base_file(&base, rel).is_none(),
            "base entry should be pruned"
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn reconcile_keeps_locally_edited_file_the_remote_deleted_and_flags_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        let rel = Path::new("problems/01.md");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(base.join("problems")).unwrap();

        let base_content = md("open", "Original", "2026-05-01T00:00:00Z");
        write_base_file(&base, rel, &base_content).unwrap();
        // Local edited it since the base.
        let local_edited = md("in_progress", "Original", "2026-05-02T00:00:00Z");
        fs::write(meta.join(rel), &local_edited).unwrap();

        let remote_files: HashSet<PathBuf> = HashSet::new();
        let mut conflicts = Vec::new();
        reconcile_remote_deletions(&meta, &base, "problems", &remote_files, &mut conflicts)
            .unwrap();

        assert!(meta.join(rel).exists(), "locally-edited file must be kept");
        assert_eq!(conflicts, vec!["problems/01.md".to_string()]);
    }

    #[test]
    fn reconcile_keeps_purely_local_file_not_in_base() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        let rel = Path::new("problems/01.md");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(base.join("problems")).unwrap();

        // Local-created file the remote has never seen (not in base).
        fs::write(
            meta.join(rel),
            md("open", "Fresh local", "2026-05-01T00:00:00Z"),
        )
        .unwrap();

        let remote_files: HashSet<PathBuf> = HashSet::new();
        let mut conflicts = Vec::new();
        reconcile_remote_deletions(&meta, &base, "problems", &remote_files, &mut conflicts)
            .unwrap();

        assert!(
            meta.join(rel).exists(),
            "purely-local new file must not be deleted"
        );
        assert!(conflicts.is_empty());
    }
}
