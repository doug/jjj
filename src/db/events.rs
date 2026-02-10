//! CRUD operations for jjj events in SQLite.
//!
//! This module provides functions to store and query events from the SQLite database.
//! Events represent decision logging and timeline entries.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};

use crate::models::{Event, EventExtra, EventType};

/// Insert an event into the database.
/// Returns the row ID of the inserted event.
pub fn insert_event(conn: &Connection, event: &Event) -> SqliteResult<i64> {
    let refs_json = serde_json::to_string(&event.refs).unwrap_or_else(|_| "[]".to_string());
    let extra_json = serde_json::to_string(&event.extra).unwrap_or_else(|_| "{}".to_string());

    conn.execute(
        "INSERT INTO events (timestamp, event_type, entity_id, actor, rationale, refs, extra)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event.when.to_rfc3339(),
            event.event_type.to_string(),
            event.entity,
            event.by,
            event.rationale,
            refs_json,
            extra_json,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// List events with optional filters, ordered by timestamp DESC.
///
/// - `entity_id`: Filter to events for a specific entity (exact match)
/// - `event_type`: Filter to events of a specific type (exact match)
/// - `limit`: Maximum number of events to return
pub fn list_events(
    conn: &Connection,
    entity_id: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
) -> SqliteResult<Vec<Event>> {
    let mut sql = String::from(
        "SELECT id, timestamp, event_type, entity_id, actor, rationale, refs, extra
         FROM events WHERE 1=1",
    );

    if entity_id.is_some() {
        sql.push_str(" AND entity_id = ?1");
    }
    if event_type.is_some() {
        sql.push_str(if entity_id.is_some() {
            " AND event_type = ?2"
        } else {
            " AND event_type = ?1"
        });
    }

    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT {}", limit));

    let mut stmt = conn.prepare(&sql)?;

    let rows = match (entity_id, event_type) {
        (Some(eid), Some(etype)) => stmt.query_map(params![eid, etype], row_to_event)?,
        (Some(eid), None) => stmt.query_map(params![eid], row_to_event)?,
        (None, Some(etype)) => stmt.query_map(params![etype], row_to_event)?,
        (None, None) => stmt.query_map([], row_to_event)?,
    };

    rows.collect()
}

/// Get events where entity matches OR is referenced in refs (for timeline view).
/// Returns events ordered by timestamp ASC for chronological timeline display.
pub fn list_events_for_timeline(conn: &Connection, entity_id: &str) -> SqliteResult<Vec<Event>> {
    // Use LIKE to search refs JSON for the entity_id
    // The refs field is stored as a JSON array like '["p1", "s2"]'
    let like_pattern = format!("%\"{}\"%" , entity_id);

    let mut stmt = conn.prepare(
        "SELECT id, timestamp, event_type, entity_id, actor, rationale, refs, extra
         FROM events
         WHERE entity_id = ?1 OR refs LIKE ?2
         ORDER BY timestamp ASC",
    )?;

    let rows = stmt.query_map(params![entity_id, like_pattern], row_to_event)?;

    rows.collect()
}

/// Delete all events from the database (for rebuild).
pub fn clear_events(conn: &Connection) -> SqliteResult<()> {
    conn.execute("DELETE FROM events", [])?;
    Ok(())
}

/// Convert a database row to an Event struct.
fn row_to_event(row: &rusqlite::Row) -> SqliteResult<Event> {
    let timestamp_str: String = row.get(1)?;
    let event_type_str: String = row.get(2)?;
    let refs_json: String = row.get::<_, Option<String>>(6)?.unwrap_or_else(|| "[]".to_string());
    let extra_json: String = row.get::<_, Option<String>>(7)?.unwrap_or_else(|| "{}".to_string());

    let event_type = parse_event_type(&event_type_str);
    let refs: Vec<String> = serde_json::from_str(&refs_json).unwrap_or_default();
    let extra: EventExtra = serde_json::from_str(&extra_json).unwrap_or_default();

    Ok(Event {
        when: DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        event_type,
        entity: row.get(3)?,
        by: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        rationale: row.get(5)?,
        refs,
        extra,
    })
}

/// Parse event_type string back to EventType enum.
fn parse_event_type(s: &str) -> EventType {
    match s {
        "problem_created" => EventType::ProblemCreated,
        "problem_solved" => EventType::ProblemSolved,
        "problem_dissolved" => EventType::ProblemDissolved,
        "problem_reopened" => EventType::ProblemReopened,
        "solution_created" => EventType::SolutionCreated,
        "solution_accepted" => EventType::SolutionAccepted,
        "solution_refuted" => EventType::SolutionRefuted,
        "critique_raised" => EventType::CritiqueRaised,
        "critique_addressed" => EventType::CritiqueAddressed,
        "critique_dismissed" => EventType::CritiqueDismissed,
        "critique_validated" => EventType::CritiqueValidated,
        "milestone_created" => EventType::MilestoneCreated,
        "milestone_completed" => EventType::MilestoneCompleted,
        // Default to ProblemCreated for unknown types
        _ => EventType::ProblemCreated,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_insert_and_list_events() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create an event
        let event = Event::new(
            EventType::ProblemCreated,
            "p1".to_string(),
            "alice".to_string(),
        )
        .with_rationale("Initial problem creation")
        .with_refs(vec!["m1".to_string()]);

        // Insert the event
        let row_id = insert_event(conn, &event).expect("Failed to insert event");
        assert!(row_id > 0);

        // List all events
        let events = list_events(conn, None, None, 100).expect("Failed to list events");
        assert_eq!(events.len(), 1);

        let loaded = &events[0];
        assert_eq!(loaded.entity, "p1");
        assert_eq!(loaded.by, "alice");
        assert_eq!(loaded.event_type, EventType::ProblemCreated);
        assert_eq!(loaded.rationale, Some("Initial problem creation".to_string()));
        assert_eq!(loaded.refs, vec!["m1".to_string()]);

        // List by entity_id
        let events = list_events(conn, Some("p1"), None, 100).expect("Failed to list");
        assert_eq!(events.len(), 1);

        // List by event_type
        let events = list_events(conn, None, Some("problem_created"), 100).expect("Failed to list");
        assert_eq!(events.len(), 1);

        // List by both
        let events = list_events(conn, Some("p1"), Some("problem_created"), 100).expect("Failed to list");
        assert_eq!(events.len(), 1);

        // List with no matches
        let events = list_events(conn, Some("p999"), None, 100).expect("Failed to list");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_timeline_events() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create a problem event
        let problem_event = Event::new(
            EventType::ProblemCreated,
            "p1".to_string(),
            "alice".to_string(),
        );
        insert_event(conn, &problem_event).expect("Failed to insert problem event");

        // Create a solution event that references the problem
        let mut extra = EventExtra::default();
        extra.problem = Some("p1".to_string());

        let solution_event = Event::new(
            EventType::SolutionCreated,
            "s1".to_string(),
            "bob".to_string(),
        )
        .with_refs(vec!["p1".to_string()])
        .with_extra(extra);
        insert_event(conn, &solution_event).expect("Failed to insert solution event");

        // Get timeline for p1 - should return both events
        let timeline = list_events_for_timeline(conn, "p1").expect("Failed to get timeline");
        assert_eq!(timeline.len(), 2);

        // First event should be the problem (chronological order)
        assert_eq!(timeline[0].entity, "p1");
        assert_eq!(timeline[0].event_type, EventType::ProblemCreated);

        // Second event should be the solution
        assert_eq!(timeline[1].entity, "s1");
        assert_eq!(timeline[1].event_type, EventType::SolutionCreated);

        // Get timeline for s1 - should only return the solution event
        let timeline = list_events_for_timeline(conn, "s1").expect("Failed to get timeline");
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].entity, "s1");
    }

    #[test]
    fn test_clear_events() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert some events
        let event1 = Event::new(EventType::ProblemCreated, "p1".to_string(), "alice".to_string());
        let event2 = Event::new(EventType::SolutionCreated, "s1".to_string(), "bob".to_string());

        insert_event(conn, &event1).expect("Failed to insert");
        insert_event(conn, &event2).expect("Failed to insert");

        // Verify events exist
        let events = list_events(conn, None, None, 100).expect("Failed to list");
        assert_eq!(events.len(), 2);

        // Clear all events
        clear_events(conn).expect("Failed to clear events");

        // Verify no events remain
        let events = list_events(conn, None, None, 100).expect("Failed to list");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_event_extra_serialization() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create a critique event with extra fields
        let mut extra = EventExtra::default();
        extra.target = Some("s1".to_string());
        extra.severity = Some("high".to_string());
        extra.title = Some("Test critique".to_string());

        let event = Event::new(
            EventType::CritiqueRaised,
            "c1".to_string(),
            "charlie".to_string(),
        )
        .with_extra(extra);

        insert_event(conn, &event).expect("Failed to insert");

        // Retrieve and verify extra fields
        let events = list_events(conn, Some("c1"), None, 100).expect("Failed to list");
        assert_eq!(events.len(), 1);

        let loaded = &events[0];
        assert_eq!(loaded.extra.target, Some("s1".to_string()));
        assert_eq!(loaded.extra.severity, Some("high".to_string()));
        assert_eq!(loaded.extra.title, Some("Test critique".to_string()));
    }

    #[test]
    fn test_list_events_limit() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert 5 events
        for i in 1..=5 {
            let event = Event::new(
                EventType::ProblemCreated,
                format!("p{}", i),
                "alice".to_string(),
            );
            insert_event(conn, &event).expect("Failed to insert");
        }

        // Verify limit works
        let events = list_events(conn, None, None, 3).expect("Failed to list");
        assert_eq!(events.len(), 3);

        let events = list_events(conn, None, None, 10).expect("Failed to list");
        assert_eq!(events.len(), 5);
    }
}
