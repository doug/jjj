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

    /// Load all events from events.jsonl.
    ///
    /// Events are stored as one JSON object per line (NDJSON format).
    /// Sorted chronologically by `when`.
    pub fn list_events(&self) -> Result<Vec<Event>> {
        self.ensure_meta_dirs()?;

        if !self.events_path.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(&self.events_path)?;
        let mut events: Vec<Event> = Vec::new();
        let mut skipped = 0usize;
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<Event>(line) {
                Ok(e) => events.push(e),
                // Don't silently drop: a malformed line or an event type from a
                // newer jjj version would otherwise vanish without a trace.
                Err(_) => skipped += 1,
            }
        }
        if skipped > 0 {
            eprintln!(
                "Warning: skipped {} unparseable line(s) in events.jsonl",
                skipped
            );
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

    /// Get the current user name from jj config
    pub fn get_current_user(&self) -> Result<String> {
        self.jj_client.user_name()
    }
}
