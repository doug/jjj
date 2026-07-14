//! Per-user event shards + offset index (Pillar 3 / M2).
//!
//! The legacy single `events.jsonl` was an append-contention point (every pod
//! appends to one file) and forced an O(all-time) re-ingest on every fetch. This
//! module shards the log by author — `events/{user}.jsonl` — so each writer
//! touches only its own file (no contention, and single-writer shards never
//! conflict on sync), and tracks a per-file byte offset so a fetch ingests only
//! the newly-appended events into the SQLite cache instead of clearing and
//! reloading the whole history.
//!
//! The old top-level `events.jsonl` is still read (union) for backward
//! compatibility, but is frozen: new events are only ever written to shards.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::sync_state::SyncState;
use super::MetadataStore;
use crate::db::events::{clear_events, insert_event};
use crate::db::Database;
use crate::error::{JjjError, Result};
use crate::models::Event;

/// Directory of per-user shards under `.jj/jjj-meta/`.
pub(super) const EVENTS_DIR: &str = "events";
/// Legacy single log (frozen; read-only after M2).
pub(super) const EVENTS_FILE: &str = "events.jsonl";
/// Local-only offset index (never synced — classified as Skip on fetch).
pub(super) const EVENT_OFFSETS_FILE: &str = ".events_offsets.json";

/// Map a free-form user identity to a filesystem-safe shard stem.
pub(super) fn shard_name(user: &str) -> String {
    let mut s: String = user
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        s.push_str("unknown");
    }
    s
}

/// Per-file byte offsets already ingested into the SQLite cache, keyed by the
/// path relative to the meta dir (e.g. `events/alice.jsonl`, `events.jsonl`).
type Offsets = BTreeMap<String, u64>;

fn load_offsets(meta: &Path) -> Offsets {
    let path = meta.join(EVENT_OFFSETS_FILE);
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_offsets(meta: &Path, offsets: &Offsets) -> Result<()> {
    let path = meta.join(EVENT_OFFSETS_FILE);
    let json = serde_json::to_string(offsets).map_err(JjjError::JsonParse)?;
    super::atomic_write(&path, json.as_bytes()).map_err(JjjError::Io)
}

impl MetadataStore {
    fn events_dir(&self) -> PathBuf {
        self.meta_path().join(EVENTS_DIR)
    }

    /// The shard key for events this process writes: its **pod** identity (the
    /// single-writer unit, Break #5) if set, else the jj user name, else
    /// `unknown`. A process is one pod, so all its events go to one shard and
    /// parallel pods never contend.
    fn writer_shard_key(&self) -> String {
        let pod = SyncState::load(self.meta_path())
            .pod
            .filter(|p| !p.trim().is_empty());
        let key = pod
            .or_else(|| self.jj_client.user_name().ok())
            .unwrap_or_default();
        shard_name(&key)
    }

    /// Append events to this pod's single-writer shard (Pillar 3). Each pod owns
    /// its shard, so parallel pods never contend and shards never conflict on sync.
    pub(super) fn append_events_to_shards(&self, events: &[Event]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(self.events_dir())?;

        let mut buf = String::new();
        for event in events {
            let line = event.to_json_line().map_err(JjjError::JsonParse)?;
            buf.push_str(&line);
            buf.push('\n');
        }

        let path = self
            .events_dir()
            .join(format!("{}.jsonl", self.writer_shard_key()));
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(JjjError::Io)?;
        file.write_all(buf.as_bytes()).map_err(JjjError::Io)?;
        file.sync_data().map_err(JjjError::Io)?;
        Ok(())
    }

    /// Every event source file: the legacy log plus all shards, as
    /// `(relative_key, absolute_path)`, sorted for determinism.
    pub(super) fn event_files(&self) -> Vec<(String, PathBuf)> {
        let mut files = Vec::new();
        let legacy = self.meta_path().join(EVENTS_FILE);
        if legacy.exists() {
            files.push((EVENTS_FILE.to_string(), legacy));
        }
        if let Ok(entries) = fs::read_dir(self.events_dir()) {
            let mut shards: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("jsonl"))
                .collect();
            shards.sort();
            for path in shards {
                let name = path.file_name().unwrap().to_string_lossy();
                files.push((format!("{}/{}", EVENTS_DIR, name), path));
            }
        }
        files
    }

    /// Incrementally ingest newly-appended events into the SQLite cache
    /// (Pillar 3). For each source file, ingest only the bytes past the recorded
    /// offset. If any file changed non-append (its recorded offset no longer
    /// lands on a line boundary, or it shrank — e.g. the legacy log was
    /// sort-merged on a back-compat fetch), fall back to a full events rebuild
    /// so the cache can never drift.
    pub(crate) fn ingest_events_incremental(&self, db: &Database) -> Result<()> {
        let meta = self.meta_path();
        let mut offsets = load_offsets(meta);
        let files = self.event_files();

        // Read each file once; decide append-only vs. needs-full.
        let mut contents: Vec<(String, String)> = Vec::with_capacity(files.len());
        let mut need_full = false;
        for (rel, path) in &files {
            let data = fs::read_to_string(path).unwrap_or_default();
            let off = *offsets.get(rel).unwrap_or(&0) as usize;
            if off > data.len() || (off > 0 && data.as_bytes().get(off - 1) != Some(&b'\n')) {
                need_full = true;
            }
            contents.push((rel.clone(), data));
        }

        if need_full {
            let conn = db.conn();
            clear_events(conn)?;
            for event in &self.list_events()? {
                insert_event(conn, event)?;
            }
            let full: Offsets = contents
                .iter()
                .map(|(rel, data)| (rel.clone(), data.len() as u64))
                .collect();
            save_offsets(meta, &full)?;
            return Ok(());
        }

        let conn = db.conn();
        for (rel, data) in &contents {
            let off = *offsets.get(rel).unwrap_or(&0) as usize;
            for line in data[off..].lines().filter(|l| !l.trim().is_empty()) {
                // list_events surfaces unparseable lines; silently skip here.
                if let Ok(event) = serde_json::from_str::<Event>(line) {
                    insert_event(conn, &event)?;
                }
            }
            offsets.insert(rel.clone(), data.len() as u64);
        }
        save_offsets(meta, &offsets)?;
        Ok(())
    }

    /// Reset the offset index to the current file lengths without ingesting.
    /// Called after a full DB rebuild (which already loaded every event) so the
    /// next fetch is incremental rather than re-ingesting the whole history.
    pub(crate) fn reset_event_offsets(&self) -> Result<()> {
        let offsets: Offsets = self
            .event_files()
            .iter()
            .map(|(rel, path)| {
                let len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                (rel.clone(), len)
            })
            .collect();
        save_offsets(self.meta_path(), &offsets)
    }
}
