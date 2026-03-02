use super::MetadataStore;
use crate::error::Result;
use crate::models::Event;

/// Extract all `jjj: {...}` events embedded in a commit description.
///
/// Each commit created by [`MetadataStore::commit_changes`] may embed one or
/// more events as `jjj: <json>` lines in its description. Multiple lines can
/// occur when a single operation triggers secondary events (e.g. approving a
/// solution auto-solves its parent problem — two events, one commit).
fn parse_events_from_description(description: &str) -> Vec<Event> {
    description
        .lines()
        .filter(|l| l.starts_with("jjj: "))
        .filter_map(|l| serde_json::from_str(&l["jjj: ".len()..]).ok())
        .collect()
}

impl MetadataStore {
    /// Queue an event to be embedded in the next commit description.
    ///
    /// Multiple events can be queued within a single [`with_metadata`] call —
    /// all will be written as separate `jjj: <json>` lines in the one commit.
    pub fn set_pending_event(&self, event: Event) {
        self.pending_events.borrow_mut().push(event);
    }

    /// Load all events by replaying the jjj commit history.
    ///
    /// Every commit on the `jjj` bookmark stores its event as `jjj: <json>`
    /// in the commit description. Walking `::@` from the metadata workspace
    /// visits all commits on all branches of the history DAG, so events from
    /// every contributor are naturally included after a bookmark merge —
    /// without conflict markers, without a cache to invalidate, without any
    /// manual resolution step.
    ///
    /// Sorted chronologically by `when`.
    pub fn list_events(&self) -> Result<Vec<Event>> {
        self.ensure_meta_checkout()?;

        let descriptions = self.meta_client.log_descriptions("::@")?;

        let mut events: Vec<Event> = descriptions
            .iter()
            .flat_map(|d| parse_events_from_description(d))
            .collect();

        events.sort_by_key(|e| e.when);

        Ok(events)
    }

    /// Reparse all events from commit history (equivalent to `list_events`).
    ///
    /// Provided as a named alias so callers with a "rebuild" mental model find
    /// a familiar entry point. Since there is no cache to invalidate this is
    /// identical to a fresh `list_events` call.
    ///
    /// Since the commit history is the primary store, this is equivalent to
    /// [`list_events`](MetadataStore::list_events) — it re-reads every
    /// `jjj: <json>` line in `::@`. The result is returned and also printed
    /// as a count so the user can see the rebuild happened.
    ///
    /// Unlike the old synthesized rebuild, this is lossless: rationales,
    /// authors, and exact timestamps are preserved exactly as originally
    /// written.
    pub fn rebuild_events_from_commits(&self) -> Result<Vec<Event>> {
        self.list_events()
    }

    /// Get the current user name from jj config
    pub fn get_current_user(&self) -> Result<String> {
        self.jj_client.user_name()
    }
}
