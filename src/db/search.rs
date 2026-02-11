//! Full-text search operations for jjj entities.
//!
//! This module provides search functions using the FTS5 virtual table.
//! It supports searching across problems, solutions, critiques, and milestones.
//!
//! Note: The FTS5 table is contentless (`content=''`), which means we can use
//! it for matching but cannot retrieve stored column values. The search uses
//! EXISTS queries with dynamic entity_id matching to correlate FTS results
//! with entity tables.

use rusqlite::{params, Connection, Result as SqliteResult};

use crate::models::Event;

/// A search result from the FTS5 index.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Entity type: "problem", "solution", "critique", or "milestone"
    pub entity_type: String,
    /// Entity ID (e.g., "p1", "s1", "c1", "m1")
    pub entity_id: String,
    /// Entity title
    pub title: String,
    /// Snippet with match context and highlighting
    pub snippet: String,
}

/// Search entities using full-text search.
///
/// Uses FTS5 MATCH query with ranking by relevance. Since the FTS table is
/// contentless, we query each entity type with EXISTS subqueries that
/// correlate entity IDs in the FTS match expression.
///
/// Returns up to 50 results with snippets showing match context.
///
/// # Arguments
/// * `conn` - Database connection
/// * `query` - FTS5 search query (supports AND, OR, NOT, phrase matching)
/// * `entity_type` - Optional filter to limit search to a specific entity type
///
/// # Example
/// ```ignore
/// let results = search(conn, "login authentication", Some("problem"))?;
/// ```
pub fn search(
    conn: &Connection,
    query: &str,
    entity_type: Option<&str>,
) -> SqliteResult<Vec<SearchResult>> {
    let mut results = Vec::new();

    // Helper to check if we should search this entity type
    let should_search = |et: &str| entity_type.is_none() || entity_type == Some(et);

    // Search problems
    if should_search("problem") {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.title, p.description, p.context
             FROM problems p
             WHERE EXISTS (
                 SELECT 1 FROM fts
                 WHERE fts MATCH 'entity_type:problem AND entity_id:' || p.id || ' AND (' || ?1 || ')'
             )
             LIMIT 50",
        )?;

        let rows = stmt.query_map(params![query], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let context: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();

            // Create snippet from description or context
            let snippet = create_snippet(&description, &context, &title, query);

            Ok(SearchResult {
                entity_type: "problem".to_string(),
                entity_id: id,
                title,
                snippet,
            })
        })?;

        for result in rows {
            results.push(result?);
        }
    }

    // Search solutions
    if should_search("solution") {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.title, s.approach, s.tradeoffs
             FROM solutions s
             WHERE EXISTS (
                 SELECT 1 FROM fts
                 WHERE fts MATCH 'entity_type:solution AND entity_id:' || s.id || ' AND (' || ?1 || ')'
             )
             LIMIT 50",
        )?;

        let rows = stmt.query_map(params![query], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let approach: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tradeoffs: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();

            let snippet = create_snippet(&approach, &tradeoffs, &title, query);

            Ok(SearchResult {
                entity_type: "solution".to_string(),
                entity_id: id,
                title,
                snippet,
            })
        })?;

        for result in rows {
            results.push(result?);
        }
    }

    // Search critiques
    if should_search("critique") {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.body
             FROM critiques c
             WHERE EXISTS (
                 SELECT 1 FROM fts
                 WHERE fts MATCH 'entity_type:critique AND entity_id:' || c.id || ' AND (' || ?1 || ')'
             )
             LIMIT 50",
        )?;

        let rows = stmt.query_map(params![query], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let body: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();

            let snippet = create_snippet(&body, "", &title, query);

            Ok(SearchResult {
                entity_type: "critique".to_string(),
                entity_id: id,
                title,
                snippet,
            })
        })?;

        for result in rows {
            results.push(result?);
        }
    }

    // Search milestones
    if should_search("milestone") {
        let mut stmt = conn.prepare(
            "SELECT m.id, m.title, m.description
             FROM milestones m
             WHERE EXISTS (
                 SELECT 1 FROM fts
                 WHERE fts MATCH 'entity_type:milestone AND entity_id:' || m.id || ' AND (' || ?1 || ')'
             )
             LIMIT 50",
        )?;

        let rows = stmt.query_map(params![query], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();

            let snippet = create_snippet(&description, "", &title, query);

            Ok(SearchResult {
                entity_type: "milestone".to_string(),
                entity_id: id,
                title,
                snippet,
            })
        })?;

        for result in rows {
            results.push(result?);
        }
    }

    // Limit to 50 total results
    results.truncate(50);

    Ok(results)
}

/// Create a snippet from entity content, prioritizing fields that contain the query.
fn create_snippet(primary: &str, secondary: &str, fallback: &str, query: &str) -> String {
    // Extract first word for simple matching
    let first_word = query
        .split_whitespace()
        .next()
        .unwrap_or(query)
        .to_lowercase();

    let text = if primary.to_lowercase().contains(&first_word) {
        primary
    } else if secondary.to_lowercase().contains(&first_word) {
        secondary
    } else if fallback.to_lowercase().contains(&first_word) {
        fallback
    } else if !primary.is_empty() {
        primary
    } else if !secondary.is_empty() {
        secondary
    } else {
        fallback
    };

    // Truncate to reasonable length
    if text.len() > 200 {
        format!("{}...", &text[..197])
    } else {
        text.to_string()
    }
}

/// Search events by rationale text.
///
/// Uses LIKE pattern matching on the rationale column.
/// Returns up to 50 results ordered by timestamp (most recent first).
///
/// # Arguments
/// * `conn` - Database connection
/// * `query` - Search text (will be wrapped in % for LIKE matching)
pub fn search_events(conn: &Connection, query: &str) -> SqliteResult<Vec<Event>> {
    let pattern = format!("%{}%", query);

    let mut stmt = conn.prepare(
        "SELECT id, timestamp, event_type, entity_id, actor, rationale, refs, extra
         FROM events
         WHERE rationale LIKE ?1
         ORDER BY timestamp DESC
         LIMIT 50",
    )?;

    let rows = stmt.query_map(params![pattern], row_to_event)?;

    rows.collect()
}

/// Convert a database row to an Event.
/// This is a copy of the function from events.rs to avoid circular dependencies.
fn row_to_event(row: &rusqlite::Row) -> SqliteResult<Event> {
    use crate::models::EventExtra;
    use chrono::{DateTime, Utc};

    let timestamp_str: String = row.get(1)?;
    let event_type_str: String = row.get(2)?;
    let refs_json: String = row
        .get::<_, Option<String>>(6)?
        .unwrap_or_else(|| "[]".to_string());
    let extra_json: String = row
        .get::<_, Option<String>>(7)?
        .unwrap_or_else(|| "{}".to_string());

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
fn parse_event_type(s: &str) -> crate::models::EventType {
    use crate::models::EventType;

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
    use crate::db::entities::{upsert_problem, upsert_solution};
    use crate::db::sync::rebuild_fts;
    use crate::db::Database;
    use crate::models::{Problem, Solution};

    #[test]
    fn test_fts_search() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert test entities into actual tables
        let mut p1 = Problem::new("p1".to_string(), "Fix login authentication".to_string());
        p1.description =
            "Users cannot login when using OAuth. Need to fix the authentication flow.".to_string();
        upsert_problem(conn, &p1).expect("Failed to insert problem");

        let mut p2 = Problem::new(
            "p2".to_string(),
            "Performance issues on dashboard".to_string(),
        );
        p2.description = "The dashboard loads slowly due to N+1 queries.".to_string();
        upsert_problem(conn, &p2).expect("Failed to insert problem");

        let mut s1 = Solution::new(
            "s1".to_string(),
            "Implement OAuth2 login flow".to_string(),
            "p1".to_string(),
        );
        s1.approach = "Use the standard OAuth2 flow with refresh tokens.".to_string();
        upsert_solution(conn, &s1).expect("Failed to insert solution");

        // Rebuild FTS index from entities
        rebuild_fts(&db).expect("Failed to rebuild FTS");

        // Search for "login" - should find both problem and solution
        let results = search(conn, "login", None).expect("Failed to search");
        assert_eq!(
            results.len(),
            2,
            "Expected 2 results for 'login', got {}",
            results.len()
        );

        // Verify we found the right entities
        let entity_ids: Vec<&str> = results.iter().map(|r| r.entity_id.as_str()).collect();
        assert!(
            entity_ids.contains(&"p1"),
            "Expected p1 in results: {:?}",
            entity_ids
        );
        assert!(
            entity_ids.contains(&"s1"),
            "Expected s1 in results: {:?}",
            entity_ids
        );

        // Verify entity types
        let p1_result = results.iter().find(|r| r.entity_id == "p1").unwrap();
        assert_eq!(p1_result.entity_type, "problem");
        assert_eq!(p1_result.title, "Fix login authentication");

        let s1_result = results.iter().find(|r| r.entity_id == "s1").unwrap();
        assert_eq!(s1_result.entity_type, "solution");
        assert_eq!(s1_result.title, "Implement OAuth2 login flow");

        // Search with entity_type filter - only problems
        let results = search(conn, "login", Some("problem")).expect("Failed to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "p1");
        assert_eq!(results[0].entity_type, "problem");

        // Search for something that doesn't match
        let results = search(conn, "nonexistent", None).expect("Failed to search");
        assert_eq!(results.len(), 0);

        // Search for "dashboard" - should only find p2
        let results = search(conn, "dashboard", None).expect("Failed to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "p2");
        assert_eq!(results[0].title, "Performance issues on dashboard");
    }

    #[test]
    fn test_search_events() {
        use crate::db::events::insert_event;
        use crate::models::{Event, EventType};

        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert test events with rationale
        let event1 = Event::new(
            EventType::ProblemCreated,
            "p1".to_string(),
            "alice".to_string(),
        )
        .with_rationale("Identified login issue during security audit");
        insert_event(conn, &event1).expect("Failed to insert event");

        let event2 = Event::new(
            EventType::SolutionAccepted,
            "s1".to_string(),
            "bob".to_string(),
        )
        .with_rationale("This approach handles edge cases correctly");
        insert_event(conn, &event2).expect("Failed to insert event");

        let event3 = Event::new(
            EventType::ProblemCreated,
            "p2".to_string(),
            "charlie".to_string(),
        )
        .with_rationale("Performance regression after last deploy");
        insert_event(conn, &event3).expect("Failed to insert event");

        // Search for "login" in rationale
        let results = search_events(conn, "login").expect("Failed to search events");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "p1");
        assert!(results[0].rationale.as_ref().unwrap().contains("login"));

        // Search for "correctly" in rationale
        let results = search_events(conn, "correctly").expect("Failed to search events");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity, "s1");

        // Search for something that doesn't match
        let results = search_events(conn, "nonexistent").expect("Failed to search events");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_fts_snippet_truncation() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert a problem with very long description
        let mut problem = Problem::new("p1".to_string(), "Complex issue".to_string());
        problem.description = "A".repeat(500); // Long description
        problem.description.push_str(" authentication test ");
        problem.description.push_str(&"B".repeat(500));
        upsert_problem(conn, &problem).expect("Failed to insert problem");

        // Rebuild FTS index
        rebuild_fts(&db).expect("Failed to rebuild FTS");

        // Search and verify snippet is truncated
        let results = search(conn, "authentication", None).expect("Failed to search");
        assert_eq!(results.len(), 1);

        // The snippet should be truncated to around 200 chars
        assert!(
            results[0].snippet.len() <= 203,
            "Snippet should be truncated: len={}",
            results[0].snippet.len()
        );
    }
}
