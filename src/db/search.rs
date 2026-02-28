//! Full-text search operations for jjj entities.
//!
//! This module provides search functions using the FTS5 virtual table.
//! It supports searching across problems, solutions, critiques, and milestones.
//!
//! Search queries are sanitized to prevent FTS5 syntax issues (e.g., hyphens
//! in UUIDs being interpreted as NOT operators). User input is quoted per-word
//! to ensure safe matching.

use rusqlite::{params, Connection, Result as SqliteResult};

use crate::models::Event;

/// A search result from the FTS5 index.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Entity type: "problem", "solution", "critique", or "milestone"
    pub entity_type: String,
    /// Entity ID (UUID7)
    pub entity_id: String,
    /// Entity title
    pub title: String,
    /// Snippet with match context and highlighting
    pub snippet: String,
}

/// Search entities using full-text search.
///
/// Uses FTS5 MATCH query with ranking by relevance. For each entity type,
/// matches FTS results by entity_id and joins with the entity table to
/// retrieve full details.
///
/// Returns up to 50 results total across all entity types, with snippets
/// showing match context. When no entity_type filter is given, results are
/// drawn from all types before truncation so no single type monopolises the
/// output.
///
/// # Arguments
/// * `conn` - Database connection
/// * `query` - Search query (words are automatically phrase-quoted for FTS5)
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

    // Sanitize user query for FTS5: escape double quotes and wrap in quotes
    // to prevent FTS5 syntax injection and handle special characters.
    let sanitized_query = sanitize_fts_query(query);

    // Helper to check if we should search this entity type
    let should_search = |et: &str| entity_type.is_none() || entity_type == Some(et);

    // Use a two-step approach: first search FTS for matching entity IDs,
    // then look up entity details. This avoids embedding UUIDs in MATCH
    // expressions (hyphens in UUIDs break FTS5 syntax).

    // Search problems
    if should_search("problem") {
        let fts_query = format!("entity_type:problem AND ({})", sanitized_query);
        let mut stmt = conn.prepare(
            "SELECT p.id, p.title, p.description, p.context
             FROM problems p
             WHERE p.id IN (
                 SELECT entity_id FROM fts WHERE fts MATCH ?1
             )",
        )?;

        let rows = stmt.query_map(params![fts_query], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let context: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();

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
        let fts_query = format!("entity_type:solution AND ({})", sanitized_query);
        let mut stmt = conn.prepare(
            "SELECT s.id, s.title, s.approach, s.tradeoffs
             FROM solutions s
             WHERE s.id IN (
                 SELECT entity_id FROM fts WHERE fts MATCH ?1
             )",
        )?;

        let rows = stmt.query_map(params![fts_query], |row| {
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
        let fts_query = format!("entity_type:critique AND ({})", sanitized_query);
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.body
             FROM critiques c
             WHERE c.id IN (
                 SELECT entity_id FROM fts WHERE fts MATCH ?1
             )",
        )?;

        let rows = stmt.query_map(params![fts_query], |row| {
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
        let fts_query = format!("entity_type:milestone AND ({})", sanitized_query);
        let mut stmt = conn.prepare(
            "SELECT m.id, m.title, m.description
             FROM milestones m
             WHERE m.id IN (
                 SELECT entity_id FROM fts WHERE fts MATCH ?1
             )",
        )?;

        let rows = stmt.query_map(params![fts_query], |row| {
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

/// Sanitize a user query for FTS5 MATCH syntax.
///
/// Each word is double-quoted as a phrase token so it is treated as a
/// literal string by FTS5. Without quoting, words like "AND", "OR", "NOT",
/// or tokens starting with "-" or "^" would be parsed as FTS5 operators.
/// Adjacent quoted tokens are implicitly AND-ed by FTS5.
fn sanitize_fts_query(query: &str) -> String {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| {
            let escaped = w.replace('"', "\"\"");
            format!("\"{}\"", escaped)
        })
        .collect();

    if words.is_empty() {
        "\"\"".to_string()
    } else {
        words.join(" ")
    }
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

/// A similarity search result with score.
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub similarity: f32,
}

/// Find entities similar to a given embedding.
///
/// Returns top-k results sorted by cosine similarity (descending).
pub fn similarity_search(
    conn: &Connection,
    query_embedding: &[f32],
    entity_type: Option<&str>,
    exclude_id: Option<&str>,
    top_k: usize,
) -> SqliteResult<Vec<SimilarityResult>> {
    use crate::db::embeddings::list_embeddings;
    use crate::embeddings::cosine_similarity;

    let embeddings = list_embeddings(conn, entity_type)?;

    let mut results: Vec<SimilarityResult> = embeddings
        .into_iter()
        .filter(|e| {
            // Exclude the source entity if specified
            exclude_id.is_none_or(|id| e.entity_id != id)
        })
        .map(|e| {
            let similarity = cosine_similarity(query_embedding, &e.embedding);
            SimilarityResult {
                entity_type: e.entity_type,
                entity_id: e.entity_id,
                title: String::new(), // Will be filled in below
                similarity,
            }
        })
        .collect();

    // Sort by similarity descending
    results.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top-k
    results.truncate(top_k);

    // Fill in titles
    for result in &mut results {
        result.title = get_entity_title(conn, &result.entity_type, &result.entity_id)?;
    }

    Ok(results)
}

/// Find entities similar to a given entity.
pub fn find_similar(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    filter_type: Option<&str>,
    top_k: usize,
) -> SqliteResult<Vec<SimilarityResult>> {
    use crate::db::embeddings::load_embedding;

    let embedding = load_embedding(conn, entity_type, entity_id)?;

    match embedding {
        Some(record) => {
            similarity_search(conn, &record.embedding, filter_type, Some(entity_id), top_k)
        }
        None => Ok(Vec::new()),
    }
}

/// Merge FTS and semantic results using Reciprocal Rank Fusion.
///
/// RRF score = Σ 1/(k + rank) for each result across both lists.
/// Higher scores indicate better combined relevance.
pub fn merge_with_rrf(
    fts_results: Vec<SearchResult>,
    semantic_results: Vec<SimilarityResult>,
    k: usize,
) -> Vec<SearchResult> {
    use std::collections::HashMap;

    let mut scores: HashMap<(String, String), f32> = HashMap::new();
    let mut titles: HashMap<(String, String), String> = HashMap::new();
    let mut snippets: HashMap<(String, String), String> = HashMap::new();

    // Add FTS scores (also capture snippets)
    for (rank, result) in fts_results.iter().enumerate() {
        let key = (result.entity_type.clone(), result.entity_id.clone());
        let rrf_score = 1.0 / (k as f32 + rank as f32 + 1.0);
        *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
        titles.insert(key.clone(), result.title.clone());
        if !result.snippet.is_empty() {
            snippets.entry(key).or_insert_with(|| result.snippet.clone());
        }
    }

    // Add semantic scores
    for (rank, result) in semantic_results.iter().enumerate() {
        let key = (result.entity_type.clone(), result.entity_id.clone());
        let rrf_score = 1.0 / (k as f32 + rank as f32 + 1.0);
        *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
        titles.entry(key).or_insert_with(|| result.title.clone());
    }

    // Sort by combined score
    let mut merged: Vec<_> = scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Convert to SearchResult, preserving FTS snippets where available
    merged
        .into_iter()
        .map(|((entity_type, entity_id), _score)| {
            let title = titles
                .get(&(entity_type.clone(), entity_id.clone()))
                .cloned()
                .unwrap_or_default();
            let snippet = snippets
                .get(&(entity_type.clone(), entity_id.clone()))
                .cloned()
                .unwrap_or_default();
            SearchResult {
                entity_type,
                entity_id,
                title,
                snippet,
            }
        })
        .collect()
}

/// Get the title of an entity by type and ID.
fn get_entity_title(conn: &Connection, entity_type: &str, entity_id: &str) -> SqliteResult<String> {
    let sql = match entity_type {
        "problem" => "SELECT title FROM problems WHERE id = ?1",
        "solution" => "SELECT title FROM solutions WHERE id = ?1",
        "critique" => "SELECT title FROM critiques WHERE id = ?1",
        "milestone" => "SELECT title FROM milestones WHERE id = ?1",
        _ => return Ok(String::new()),
    };

    conn.query_row(sql, params![entity_id], |row| row.get(0))
        .or(Ok(String::new()))
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
        "solution_reviewed" => EventType::SolutionReviewed,
        "solution_accepted" => EventType::SolutionAccepted,
        "solution_refuted" => EventType::SolutionRefuted,
        "critique_raised" => EventType::CritiqueRaised,
        "critique_addressed" => EventType::CritiqueAddressed,
        "critique_dismissed" => EventType::CritiqueDismissed,
        "critique_validated" => EventType::CritiqueValidated,
        "critique_replied" => EventType::CritiqueReplied,
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

    #[test]
    fn test_similarity_search() {
        use crate::db::embeddings::upsert_embedding;

        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert problems with embeddings
        let p1 = Problem::new("p1".to_string(), "Auth problem".to_string());
        upsert_problem(conn, &p1).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0, 0.0]).expect("Failed");

        let p2 = Problem::new("p2".to_string(), "Similar auth issue".to_string());
        upsert_problem(conn, &p2).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p2", "test", &[0.9, 0.1, 0.0]).expect("Failed");

        let p3 = Problem::new("p3".to_string(), "Unrelated problem".to_string());
        upsert_problem(conn, &p3).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p3", "test", &[0.0, 0.0, 1.0]).expect("Failed");

        // Search for similar to p1's embedding
        let results = similarity_search(conn, &[1.0, 0.0, 0.0], None, Some("p1"), 10)
            .expect("Failed to search");

        assert_eq!(results.len(), 2);
        // p2 should be more similar than p3
        assert_eq!(results[0].entity_id, "p2");
        assert_eq!(results[1].entity_id, "p3");
        assert!(results[0].similarity > results[1].similarity);
    }

    #[test]
    fn test_find_similar() {
        use crate::db::embeddings::upsert_embedding;

        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        let p1 = Problem::new("p1".to_string(), "Problem one".to_string());
        upsert_problem(conn, &p1).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0]).expect("Failed");

        let p2 = Problem::new("p2".to_string(), "Problem two".to_string());
        upsert_problem(conn, &p2).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p2", "test", &[0.8, 0.2]).expect("Failed");

        let results = find_similar(conn, "problem", "p1", None, 10).expect("Failed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "p2");
    }

    #[test]
    fn test_merge_with_rrf() {
        let fts_results = vec![
            SearchResult {
                entity_type: "problem".to_string(),
                entity_id: "p1".to_string(),
                title: "First".to_string(),
                snippet: "".to_string(),
            },
            SearchResult {
                entity_type: "problem".to_string(),
                entity_id: "p2".to_string(),
                title: "Second".to_string(),
                snippet: "".to_string(),
            },
        ];

        let semantic_results = vec![
            SimilarityResult {
                entity_type: "problem".to_string(),
                entity_id: "p2".to_string(),
                title: "Second".to_string(),
                similarity: 0.9,
            },
            SimilarityResult {
                entity_type: "problem".to_string(),
                entity_id: "p3".to_string(),
                title: "Third".to_string(),
                similarity: 0.8,
            },
        ];

        let merged = merge_with_rrf(fts_results, semantic_results, 60);

        // p2 appears in both, should rank highest
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].entity_id, "p2");
    }
}
