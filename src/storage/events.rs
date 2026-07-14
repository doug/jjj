use super::MetadataStore;
use crate::error::Result;
use crate::models::Event;

impl MetadataStore {
    /// Queue an event to be appended to events.jsonl on the next flush.
    ///
    /// Multiple events can be queued within a single [`with_metadata`] call —
    /// all will be appended as separate JSON lines when the operation commits.
    pub fn set_pending_event(&self, event: Event) {
        self.pending_events.borrow_mut().push(event);
    }

    /// Load all events, unioning the legacy `events.jsonl` with every per-user
    /// shard (`events/{user}.jsonl`, Pillar 3).
    ///
    /// Events are one JSON object per line (NDJSON). Lines are deduped exactly
    /// (a fetched shard may overlap a locally-seen one) and sorted
    /// chronologically by `when`.
    pub fn list_events(&self) -> Result<Vec<Event>> {
        self.ensure_meta_dirs()?;

        let mut events: Vec<Event> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut skipped = 0usize;

        for (_rel, path) in self.event_files() {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
                    continue;
                }
                match serde_json::from_str::<Event>(trimmed) {
                    Ok(e) => events.push(e),
                    // Don't silently drop: a malformed line or an event type from
                    // a newer jjj version would otherwise vanish without a trace.
                    Err(_) => skipped += 1,
                }
            }
        }

        if skipped > 0 {
            eprintln!("Warning: skipped {} unparseable event line(s)", skipped);
        }

        events.sort_by_key(|e| e.when);

        Ok(events)
    }

    /// Reload events from events.jsonl.
    ///
    /// Provided as a named alias so callers with a "rebuild" mental model find
    /// a familiar entry point.
    pub fn rebuild_events_from_commits(&self) -> Result<Vec<Event>> {
        self.list_events()
    }

    /// Resolve the current actor identity used for event authorship, claim
    /// assignment, and ranking (coordination decision 9 — one namespaced id).
    ///
    /// Resolution order: `JJJ_USER` env override → the pod id (so a pod that
    /// sets only `JJJ_POD` still gets a stable identity) → the jj user name.
    pub fn get_current_user(&self) -> Result<String> {
        if let Some(user) = std::env::var("JJJ_USER")
            .ok()
            .filter(|u| !u.trim().is_empty())
        {
            return Ok(user);
        }
        if let Some(pod) = super::sync_state::SyncState::load(self.meta_path())
            .pod
            .filter(|p| !p.trim().is_empty())
        {
            return Ok(pod);
        }
        self.jj_client.user_name()
    }
}
