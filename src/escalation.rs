//! Escalations: the swarm asking for a person.
//!
//! When the host's OAuth session expired mid-run, the fleet failed 400
//! consecutive turns over 6.8 hours. Every container stayed up, the sampler kept
//! writing rows, and the score sat frozen. Nothing in the system could say *"I
//! am blocked on something only a person can fix"* — it was found by reading
//! logs, after the fact.
//!
//! # Why events rather than an entity
//!
//! An escalation is a transient signal, not a thing with a lifecycle worth
//! reviewing: it is raised, a person acts, it is cleared. Making it an entity
//! would buy a status column and cost a table, a directory, a merge path and a
//! migration. The event log already syncs, already merges by append, and already
//! carries actor and timestamp — everything an escalation needs.
//!
//! "Open" is therefore derived: every `EscalationRaised` whose entity id has no
//! later `EscalationCleared`. Deriving rather than storing means two clones that
//! saw different halves of the exchange converge on the same answer once their
//! shards meet, with no last-writer race over a status field.

use crate::models::{Event, EventType};

/// An escalation that has been raised and not yet cleared.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenEscalation {
    /// Stable id, used to clear it.
    pub id: String,
    /// Who raised it.
    pub by: String,
    /// Why a person is needed.
    pub reason: String,
    /// Entities this concerns, if any.
    pub about: Vec<String>,
    /// When it was raised.
    pub raised_at: chrono::DateTime<chrono::Utc>,
}

impl OpenEscalation {
    /// How long this has been waiting, as of `now`.
    pub fn age(&self, now: chrono::DateTime<chrono::Utc>) -> chrono::Duration {
        now.signed_duration_since(self.raised_at)
    }
}

/// Derive the open escalations from an event log.
///
/// `events` need not be sorted: a clear is honoured wherever it appears,
/// because clearing is idempotent and an escalation raised twice with the same
/// id is the same escalation. Result is oldest first — the one that has been
/// waiting longest is the one a person should look at.
pub fn open_escalations(events: &[Event]) -> Vec<OpenEscalation> {
    use std::collections::HashSet;

    let cleared: HashSet<&str> = events
        .iter()
        .filter(|e| e.event_type == EventType::EscalationCleared)
        .map(|e| e.entity.as_str())
        .collect();

    let mut open: Vec<OpenEscalation> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for e in events {
        if e.event_type != EventType::EscalationRaised {
            continue;
        }
        if cleared.contains(e.entity.as_str()) || !seen.insert(e.entity.as_str()) {
            continue;
        }
        open.push(OpenEscalation {
            id: e.entity.clone(),
            by: e.by.clone(),
            reason: e
                .rationale
                .clone()
                .unwrap_or_else(|| "(no reason given)".to_string()),
            about: e.refs.clone(),
            raised_at: e.when,
        });
    }

    open.sort_by_key(|e| e.raised_at);
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raised(id: &str, by: &str, reason: &str) -> Event {
        Event::new(EventType::EscalationRaised, id, by).with_rationale(reason)
    }

    fn cleared(id: &str, by: &str) -> Event {
        Event::new(EventType::EscalationCleared, id, by)
    }

    #[test]
    fn an_unanswered_escalation_is_open() {
        let events = vec![raised("e1", "agent-a", "OAuth token expired")];
        let open = open_escalations(&events);
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "e1");
        assert_eq!(open[0].reason, "OAuth token expired");
    }

    #[test]
    fn clearing_closes_it() {
        let events = vec![raised("e1", "agent-a", "token"), cleared("e1", "doug")];
        assert!(open_escalations(&events).is_empty());
    }

    #[test]
    fn a_clear_is_honoured_whatever_order_the_shards_merged_in() {
        // Per-user event shards are unioned and sorted by timestamp, but two
        // agents' clocks are not synchronized, so a clear can legitimately sort
        // before the raise it answers. Treating that as "still open" would mean
        // a resolved escalation reappearing every time the shards merged
        // differently.
        let events = vec![cleared("e1", "doug"), raised("e1", "agent-a", "token")];
        assert!(open_escalations(&events).is_empty());
    }

    #[test]
    fn the_same_escalation_raised_twice_is_one_escalation() {
        let events = vec![
            raised("e1", "agent-a", "token"),
            raised("e1", "agent-a", "token"),
        ];
        assert_eq!(open_escalations(&events).len(), 1);
    }

    #[test]
    fn oldest_first() {
        use chrono::{Duration, Utc};
        let now = Utc::now();
        let events = vec![
            raised("new", "a", "second").with_timestamp(now),
            raised("old", "a", "first").with_timestamp(now - Duration::hours(3)),
        ];
        let open = open_escalations(&events);
        assert_eq!(open[0].id, "old", "the longest wait comes first");
        assert!(open[0].age(now) >= Duration::hours(3));
    }

    #[test]
    fn other_events_are_ignored() {
        let events = vec![
            Event::new(EventType::ProblemCreated, "p1", "a"),
            raised("e1", "a", "blocked"),
            Event::new(EventType::SolutionApproved, "s1", "a"),
        ];
        assert_eq!(open_escalations(&events).len(), 1);
    }
}
