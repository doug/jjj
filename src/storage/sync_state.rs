//! Local-only sync state — the merge-base pointer for incremental fetch.
//!
//! # Why this exists (Pillar 1 of the agent-swarm scaling design)
//!
//! The legacy fetch path enumerated *every* remote file and three-way merged it
//! against a full `jjj-meta-base/` mirror tree — O(total corpus) per fetch. The
//! incremental design replaces that mirror with a single **revision id**: the
//! jjj-bookmark commit we last merged and pushed. The merge ancestor of any file
//! `F` is then `jj file show -r <last_synced_rev> F` — one immutable,
//! jj-reachable revision, never a parallel directory tree that can drift.
//!
//! This file is **local-only and never synced**. It lives at
//! `.jj/jjj-meta/.sync_state.json` (a dotfile, so it is excluded from the entity
//! globs *and* from the push workspace copy, which only mirrors the entity dirs,
//! `config.toml`, `events*`, and `rankings/`). Each clone tracks its own pointer.
//!
//! # The state-machine invariant (highest-risk area — audit 0.1 lived here)
//!
//! Advancing `last_synced_rev` to the wrong commit silently loses local edits:
//! if it points at a state that already folds in unpushed local work, that work
//! looks like part of the shared base on the next fetch and a divergent remote
//! reverts it with no conflict. The rule, enforced by construction here: only
//! advance to a commit that is *itself* reachable on the remote — never to a
//! merged-but-unpushed local working state.
//!
//! 1. A `jjj sync` runs **fetch → merge → push**, in that order.
//! 2. The three-way merge base for that fetch is the **pre-sync**
//!    `last_synced_rev` (read it *before* mutating anything).
//! 3. After a successful push, call [`SyncState::advance`] with the
//!    **just-pushed** commit (which already contains merged remote + local).
//! 4. After a successful fetch that resolved to exactly one remote head, call
//!    [`SyncState::advance`] with that head directly — it already existed on
//!    the remote before this fetch touched anything, so recording it carries
//!    none of the risk in point 3 (never with a commit built from this
//!    clone's own working files); it just avoids a permanent cold start for a
//!    pod that only ever fetches. See `src/commands/fetch.rs` step 6b.
//!
//! Never advance to a merged-but-unpushed working state — a commit whose
//! content includes local edits that have not themselves been pushed.
//! [`SyncState::advance`] is the *only* way to move the pointer, so every
//! caller goes through the same documented gate.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The shared bookmark prefix. A pod pushes to its own single-writer ref
/// `jjj/{pod}` to avoid the thundering-herd contention M0 measured on the one
/// shared `jjj` bookmark (~quadratic; 14.7s to drain 20 pods). Fetch reads and
/// unions all `jjj*` refs. A pod-less clone falls back to the bare `jjj`
/// bookmark for single-writer / backward-compatible operation.
pub const BOOKMARK_PREFIX: &str = "jjj";

/// File name of the local-only sync-state pointer, stored under the meta dir.
/// The leading dot keeps it out of every entity glob and the push copy.
pub const SYNC_STATE_FILE: &str = ".sync_state.json";

/// Current on-disk schema version. Bumped if the shape changes so an older
/// jjj reading a newer file (or vice versa) degrades to a safe cold start
/// rather than mis-parsing.
const CURRENT_VERSION: u32 = 1;

/// Local-only pointer state persisted at `.jj/jjj-meta/.sync_state.json`.
///
/// A missing, unreadable, or version-mismatched file deserializes to
/// [`SyncState::default`] — i.e. a **cold start**: `last_synced_rev == None`
/// triggers a full reconcile + DB rebuild on the next fetch (the common
/// fresh-clone onboarding path, not an error).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncState {
    /// Schema version of this file.
    #[serde(default)]
    pub version: u32,

    /// Commit id of the jjj bookmark this clone last **merged and pushed**.
    /// It is the merge ancestor for the next fetch. `None` ⇒ cold start.
    #[serde(default)]
    pub last_synced_rev: Option<String>,

    /// This clone's pod id. Determines the per-pod push bookmark `jjj/{pod}`.
    /// `None` ⇒ push to the bare `jjj` bookmark (single-writer / back-compat).
    #[serde(default)]
    pub pod: Option<String>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            last_synced_rev: None,
            pod: None,
        }
    }
}

impl SyncState {
    /// Path to the sync-state file for a given meta dir (`.jj/jjj-meta/`).
    pub fn path(meta_path: &Path) -> PathBuf {
        meta_path.join(SYNC_STATE_FILE)
    }

    /// Load the sync state for `meta_path`.
    ///
    /// Any failure to read or parse — missing file, corrupt JSON, or a
    /// `version` this build doesn't recognize — yields [`Self::default`] (a
    /// safe cold start) rather than an error. A stale pointer is always
    /// recoverable by re-reconciling against the remote, so we never let a bad
    /// state file block a command.
    pub fn load(meta_path: &Path) -> Self {
        let mut state = match std::fs::read_to_string(Self::path(meta_path)) {
            Ok(raw) => match serde_json::from_str::<SyncState>(&raw) {
                // A future/unknown version → cold start instead of trusting a
                // pointer whose semantics we can't guarantee.
                Ok(state) if state.version == CURRENT_VERSION => state,
                _ => Self::default(),
            },
            Err(_) => Self::default(),
        };
        // Per-process pod override (coordination): an agent sets `JJJ_POD` to run
        // as a distinct single-writer pod without editing the state file. This
        // only steers *its own* push bookmark / event shard; `last_synced_rev`
        // (the merge base) still comes from the file.
        if let Some(pod) = std::env::var("JJJ_POD")
            .ok()
            .filter(|p| !p.trim().is_empty())
        {
            state.pod = Some(pod);
        }
        state
    }

    /// Persist the sync state atomically (write-tmp-then-rename) so a crash
    /// mid-write can never leave a half-written pointer that parses wrong.
    pub fn save(&self, meta_path: &Path) -> Result<()> {
        if let Some(parent) = Self::path(meta_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(crate::error::JjjError::JsonParse)?;
        super::atomic_write(&Self::path(meta_path), json.as_bytes())?;
        Ok(())
    }

    /// Advance the merge-base pointer to a commit known to already exist on
    /// the remote — a **just-pushed** commit, or (on fetch) a remote head that
    /// required no local content to identify.
    ///
    /// This is the ONLY way to move `last_synced_rev`. Per the state-machine
    /// invariant (see module docs), call it only with a commit that is already
    /// on the remote — never a merged-but-unpushed state built from this
    /// clone's own working files.
    pub fn advance(&mut self, pushed_rev: impl Into<String>) {
        self.last_synced_rev = Some(pushed_rev.into());
    }

    /// True when there is no recorded merge base yet — the next fetch must do a
    /// full reconcile + DB rebuild (cold start) rather than a delta.
    pub fn is_cold_start(&self) -> bool {
        self.last_synced_rev.is_none()
    }

    /// The bookmark this clone pushes its own writes to: `jjj-{pod}` when a pod
    /// id is set, else the bare `jjj` bookmark. Single-writer by construction,
    /// so concurrent pods never contend on the same ref (Break #5).
    ///
    /// **The separator is `-`, not `/`, and that is load-bearing.** A git ref is
    /// a path: `refs/heads/jjj` is a *file*, so `refs/heads/jjj/theory` would
    /// require that same path to also be a *directory*. Git rejects it —
    /// `cannot lock ref 'refs/heads/jjj/theory': 'refs/heads/jjj' exists` — and
    /// since a plain `jjj push` (no pod) creates the bare bookmark, every real
    /// repository has it. With `/`, per-pod push therefore failed everywhere it
    /// mattered and Break #5's fix was inoperative. Discovered by the swarm
    /// trial in `tools/swarm/`; see `pod_and_bare_bookmarks_coexist_on_a_remote`
    /// in `tests/push_fetch_test.rs`.
    ///
    /// Both names still match the `jjj*` glob used to track bookmarks and to
    /// discover heads, so fetch is unaffected.
    pub fn push_bookmark(&self) -> String {
        match &self.pod {
            Some(pod) if !pod.is_empty() => format!("{}-{}", BOOKMARK_PREFIX, sanitize_pod(pod)),
            _ => BOOKMARK_PREFIX.to_string(),
        }
    }
}

/// Sanitize a pod id into a single bookmark path segment. Namespaced ids like
/// `team/alice` collapse to one segment (`team-alice`) so the push ref is
/// always exactly `jjj-<segment>` — never nested, and never containing
/// characters a git ref name rejects.
fn sanitize_pod(pod: &str) -> String {
    pod.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '-',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SyncState::load` deliberately overrides `pod` from `JJJ_POD` (see its
    /// doc comment), so any test that round-trips `pod` through `load` must
    /// not inherit the ambient value a swarm agent runs with — otherwise the
    /// test's outcome depends on who happens to run it. Restores whatever was
    /// there before, even on panic, so it can't leak into other tests.
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn default_is_cold_start() {
        let s = SyncState::default();
        assert!(s.is_cold_start());
        assert_eq!(s.version, CURRENT_VERSION);
        assert_eq!(s.push_bookmark(), "jjj");
    }

    #[test]
    fn missing_file_loads_as_cold_start() {
        let tmp = tempfile::tempdir().unwrap();
        let s = SyncState::load(tmp.path());
        assert!(s.is_cold_start());
    }

    #[test]
    fn save_then_load_roundtrips() {
        // `load` applies a per-process `JJJ_POD` override (see the module
        // docs), so a swarm/CI environment that already exports `JJJ_POD`
        // would silently overwrite `pod` and fail the equality check below.
        // Unset it for the duration of this test so the roundtrip is
        // verified against the file content, not the ambient environment.
        let _guard = EnvVarGuard::unset("JJJ_POD");

        let tmp = tempfile::tempdir().unwrap();
        let mut s = SyncState {
            pod: Some("theory".to_string()),
            ..Default::default()
        };
        s.advance("abc123");
        s.save(tmp.path()).unwrap();

        let loaded = SyncState::load(tmp.path());

        assert_eq!(loaded, s);
        assert_eq!(loaded.last_synced_rev.as_deref(), Some("abc123"));
        assert!(!loaded.is_cold_start());
    }

    #[test]
    fn corrupt_file_falls_back_to_cold_start() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(SyncState::path(tmp.path()), b"{ not json ]").unwrap();
        let s = SyncState::load(tmp.path());
        assert!(s.is_cold_start());
    }

    #[test]
    fn future_version_falls_back_to_cold_start() {
        let tmp = tempfile::tempdir().unwrap();
        // A well-formed file from a newer jjj we don't understand.
        std::fs::write(
            SyncState::path(tmp.path()),
            br#"{"version":999,"last_synced_rev":"deadbeef","pod":"x"}"#,
        )
        .unwrap();
        let s = SyncState::load(tmp.path());
        assert!(
            s.is_cold_start(),
            "an unrecognized version must not be trusted as a merge base"
        );
    }

    #[test]
    fn advance_is_the_only_mutator_and_sets_rev() {
        let mut s = SyncState::default();
        assert!(s.is_cold_start());
        s.advance("rev-1");
        assert_eq!(s.last_synced_rev.as_deref(), Some("rev-1"));
        s.advance("rev-2".to_string());
        assert_eq!(s.last_synced_rev.as_deref(), Some("rev-2"));
    }

    #[test]
    fn push_bookmark_is_per_pod_when_set() {
        let s = SyncState {
            pod: Some("theory".to_string()),
            ..Default::default()
        };
        assert_eq!(s.push_bookmark(), "jjj-theory");
    }

    #[test]
    fn push_bookmark_sanitizes_namespaced_pod_to_one_segment() {
        let mut s = SyncState {
            pod: Some("team/alice".to_string()),
            ..Default::default()
        };
        // A '/' in the pod id would otherwise nest the ref deeper still.
        assert_eq!(s.push_bookmark(), "jjj-team-alice");

        s.pod = Some("pod theory!".to_string());
        assert_eq!(s.push_bookmark(), "jjj-pod-theory-");
    }

    #[test]
    fn empty_pod_falls_back_to_shared_bookmark() {
        let s = SyncState {
            pod: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(s.push_bookmark(), "jjj");
    }

    /// The state file is a dotfile so it is excluded from entity globs and the
    /// push workspace copy. Guard the name so a refactor can't silently start
    /// syncing local-only state.
    #[test]
    fn state_file_is_a_dotfile() {
        assert!(SYNC_STATE_FILE.starts_with('.'));
    }
}
