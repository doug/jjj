//! Sync operations between SQLite database and markdown files.
//!
//! This module handles loading metadata from the shadow branch (markdown files)
//! into SQLite for fast queries, and dumping SQLite data back to markdown.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use rusqlite::{params, Connection};

use crate::db::entities::{
    list_critiques, list_milestones, list_problems, list_solutions,
    populate_problem_computed_fields, populate_solution_computed_fields, upsert_critique,
    upsert_milestone, upsert_problem, upsert_solution,
};
use crate::db::events::{clear_events, insert_event, list_events};
use crate::db::Database;
use crate::error::Result;
use crate::models::Event;
use crate::storage::MetadataStore;

const EVENTS_FILE: &str = "events.jsonl";

/// Load all metadata from markdown files into SQLite.
///
/// This clears all SQLite tables and reloads from the shadow branch.
/// Used during initial sync or when rebuilding the cache.
pub fn load_from_markdown(db: &Database, store: &MetadataStore) -> Result<()> {
    let conn = db.conn();

    // Set dirty flag before we start modifying data
    set_dirty_internal(conn, true)?;

    // Clear all tables
    clear_all_tables(conn)?;

    // Load milestones first (problems reference milestones via FK)
    let milestones = store.list_milestones()?;
    for milestone in &milestones {
        upsert_milestone(conn, milestone)?;
    }

    // Load problems (solutions reference problems via FK)
    let problems = store.list_problems()?;
    for problem in &problems {
        upsert_problem(conn, problem)?;
    }

    // Load solutions (critiques reference solutions via FK)
    let solutions = store.list_solutions()?;
    for solution in &solutions {
        upsert_solution(conn, solution)?;
    }

    // Load critiques
    let critiques = store.list_critiques()?;
    for critique in &critiques {
        upsert_critique(conn, critique)?;
    }

    // Load events from JSONL file
    load_events_from_jsonl(conn, store.meta_path())?;

    // Rebuild FTS index
    rebuild_fts(db)?;

    // Clear dirty flag on successful completion
    set_dirty_internal(conn, false)?;

    Ok(())
}

/// Dump all metadata from SQLite back to markdown files.
///
/// This writes all entities from SQLite to the shadow branch.
/// Used when syncing local changes back to the repository.
pub fn dump_to_markdown(db: &Database, store: &MetadataStore) -> Result<()> {
    let conn = db.conn();

    // Dump problems (with computed fields populated)
    let mut problems = list_problems(conn)?;
    populate_problem_computed_fields(conn, &mut problems)?;
    for problem in &problems {
        store.save_problem(problem)?;
    }

    // Dump solutions (with computed fields populated)
    let mut solutions = list_solutions(conn)?;
    populate_solution_computed_fields(conn, &mut solutions)?;
    for solution in &solutions {
        store.save_solution(solution)?;
    }

    // Dump critiques
    let critiques = list_critiques(conn)?;
    for critique in &critiques {
        store.save_critique(critique)?;
    }

    // Dump milestones
    let milestones = list_milestones(conn)?;
    for milestone in &milestones {
        store.save_milestone(milestone)?;
    }

    // Dump events to JSONL file
    dump_events_to_jsonl(conn, store.meta_path())?;

    Ok(())
}

/// Rebuild the full-text search index from all entities.
pub fn rebuild_fts(db: &Database) -> Result<()> {
    let conn = db.conn();

    // Clear existing FTS data
    conn.execute("DELETE FROM fts", [])?;

    // Index problems
    let problems = list_problems(conn)?;
    for problem in &problems {
        let body = format!("{}\n{}", problem.description, problem.context);
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
            params!["problem", &problem.id, &problem.title, &body],
        )?;
    }

    // Index solutions
    let solutions = list_solutions(conn)?;
    for solution in &solutions {
        let body = format!("{}\n{}", solution.approach, solution.tradeoffs);
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
            params!["solution", &solution.id, &solution.title, &body],
        )?;
    }

    // Index critiques
    let critiques = list_critiques(conn)?;
    for critique in &critiques {
        let body = format!("{}\n{}", critique.argument, critique.evidence);
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
            params!["critique", &critique.id, &critique.title, &body],
        )?;
    }

    // Index milestones
    let milestones = list_milestones(conn)?;
    for milestone in &milestones {
        let body = format!("{}\n{}", milestone.goals, milestone.success_criteria);
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
            params!["milestone", &milestone.id, &milestone.title, &body],
        )?;
    }

    Ok(())
}

/// Update a single entity's FTS entry (upsert).
///
/// Call this after saving an entity to keep FTS in sync incrementally,
/// avoiding a full rebuild.
pub fn update_fts_entry(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    title: &str,
    body: &str,
) -> std::result::Result<(), rusqlite::Error> {
    // Delete existing entry if any
    conn.execute(
        "DELETE FROM fts WHERE entity_type = ?1 AND entity_id = ?2",
        params![entity_type, entity_id],
    )?;
    // Insert updated entry
    conn.execute(
        "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?1, ?2, ?3, ?4)",
        params![entity_type, entity_id, title, body],
    )?;
    Ok(())
}

/// Remove a single entity's FTS entry.
pub fn remove_fts_entry(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> std::result::Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM fts WHERE entity_type = ?1 AND entity_id = ?2",
        params![entity_type, entity_id],
    )?;
    Ok(())
}

/// Rebuild all embeddings from entities.
///
/// This computes embeddings for all problems, solutions, critiques, and milestones.
/// Uses batch processing for efficiency.
pub fn rebuild_embeddings(
    db: &Database,
    client: &crate::embeddings::EmbeddingClient,
) -> Result<()> {
    use crate::db::embeddings::{clear_embeddings, upsert_embedding};
    use crate::embeddings::{
        prepare_critique_text, prepare_milestone_text, prepare_problem_text, prepare_solution_text,
    };

    let conn = db.conn();
    let model = client.model();

    // Clear existing embeddings
    clear_embeddings(conn)?;

    // Process problems
    let problems = list_problems(conn)?;
    for problem in &problems {
        let text = prepare_problem_text(&problem.title, &problem.description, &problem.context);
        if let Ok(embedding) = client.embed(&text) {
            upsert_embedding(conn, "problem", &problem.id, model, &embedding)?;
        }
    }

    // Process solutions
    let solutions = list_solutions(conn)?;
    for solution in &solutions {
        let text = prepare_solution_text(&solution.title, &solution.approach, &solution.tradeoffs);
        if let Ok(embedding) = client.embed(&text) {
            upsert_embedding(conn, "solution", &solution.id, model, &embedding)?;
        }
    }

    // Process critiques
    let critiques = list_critiques(conn)?;
    for critique in &critiques {
        let text = prepare_critique_text(&critique.title, &critique.argument, &critique.evidence);
        if let Ok(embedding) = client.embed(&text) {
            upsert_embedding(conn, "critique", &critique.id, model, &embedding)?;
        }
    }

    // Process milestones
    let milestones = list_milestones(conn)?;
    for milestone in &milestones {
        let text = prepare_milestone_text(
            &milestone.title,
            &milestone.goals,
            &milestone.success_criteria,
        );
        if let Ok(embedding) = client.embed(&text) {
            upsert_embedding(conn, "milestone", &milestone.id, model, &embedding)?;
        }
    }

    Ok(())
}

/// Check if the database has uncommitted changes (dirty flag is set).
pub fn is_dirty(db: &Database) -> Result<bool> {
    is_dirty_internal(db.conn())
}

/// Set the dirty flag indicating uncommitted changes.
pub fn set_dirty(db: &Database, dirty: bool) -> Result<()> {
    set_dirty_internal(db.conn(), dirty)
}

// ============================================================================
// Private helpers
// ============================================================================

/// Clear all entity tables (problems, solutions, critiques, milestones, events).
fn clear_all_tables(conn: &Connection) -> Result<()> {
    // Clear in reverse order of dependencies
    conn.execute("DELETE FROM embeddings", [])?;
    conn.execute("DELETE FROM critiques", [])?;
    conn.execute("DELETE FROM solutions", [])?;
    conn.execute("DELETE FROM problems", [])?;
    conn.execute("DELETE FROM milestones", [])?;
    clear_events(conn)?;
    conn.execute("DELETE FROM fts", [])?;
    Ok(())
}

/// Load events from the events.jsonl file.
fn load_events_from_jsonl(conn: &Connection, meta_path: &Path) -> Result<()> {
    let events_path = meta_path.join(EVENTS_FILE);

    if !events_path.exists() {
        return Ok(());
    }

    let file = fs::File::open(&events_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event: Event = serde_json::from_str(&line)?;
        insert_event(conn, &event)?;
    }

    Ok(())
}

/// Dump events to the events.jsonl file.
fn dump_events_to_jsonl(conn: &Connection, meta_path: &Path) -> Result<()> {
    let events_path = meta_path.join(EVENTS_FILE);

    // List all events (they come in DESC order, we want ASC for the file)
    // Use a reasonable limit instead of usize::MAX to avoid SQLite integer overflow
    let mut events = list_events(conn, None, None, 1_000_000)?;
    events.sort_by(|a, b| a.when.cmp(&b.when));

    let mut file = fs::File::create(&events_path)?;

    for event in &events {
        let json = event.to_json_line()?;
        writeln!(file, "{}", json)?;
    }

    Ok(())
}

/// Internal function to check dirty flag.
fn is_dirty_internal(conn: &Connection) -> Result<bool> {
    let result: std::result::Result<String, _> =
        conn.query_row("SELECT value FROM meta WHERE key = 'dirty'", [], |row| {
            row.get(0)
        });

    match result {
        Ok(value) => Ok(value == "true" || value == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Internal function to set dirty flag.
fn set_dirty_internal(conn: &Connection, dirty: bool) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('dirty', ?1)",
        [if dirty { "true" } else { "false" }],
    )?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_flag() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // Should not be dirty initially
        assert!(!is_dirty(&db).expect("Failed to check dirty"));

        // Set dirty
        set_dirty(&db, true).expect("Failed to set dirty");
        assert!(is_dirty(&db).expect("Failed to check dirty"));

        // Clear dirty
        set_dirty(&db, false).expect("Failed to clear dirty");
        assert!(!is_dirty(&db).expect("Failed to check dirty"));
    }

    #[test]
    fn test_rebuild_fts_empty() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // Should not fail on empty database
        rebuild_fts(&db).expect("Failed to rebuild FTS");

        // Verify FTS is empty
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fts", [], |row| row.get(0))
            .expect("Failed to count FTS rows");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_rebuild_fts_with_data() {
        use crate::db::entities::upsert_problem;
        use crate::models::Problem;

        let db = Database::open_in_memory().expect("Failed to open database");

        // Insert a problem
        let mut problem = Problem::new("p1".to_string(), "Test Problem".to_string());
        problem.description = "This is a test description".to_string();
        problem.context = "Some context here".to_string();
        upsert_problem(db.conn(), &problem).expect("Failed to insert problem");

        // Rebuild FTS
        rebuild_fts(&db).expect("Failed to rebuild FTS");

        // Verify FTS has the problem indexed
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fts", [], |row| row.get(0))
            .expect("Failed to count FTS rows");
        assert_eq!(count, 1);

        // Verify we can search for the problem using FTS
        // Note: FTS5 contentless tables don't store actual column values,
        // so we use highlight() or bm25() for ranking, not retrieving columns.
        // Here we just verify the match works by counting results.
        let match_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fts WHERE fts MATCH 'test'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to search FTS");
        assert_eq!(match_count, 1);
    }

    #[test]
    fn test_clear_all_tables() {
        use crate::db::entities::{upsert_problem, upsert_solution};
        use crate::models::{Problem, Solution};

        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert some data
        let problem = Problem::new("p1".to_string(), "Test Problem".to_string());
        upsert_problem(conn, &problem).expect("Failed to insert problem");

        let solution = Solution::new(
            "s1".to_string(),
            "Test Solution".to_string(),
            "p1".to_string(),
        );
        upsert_solution(conn, &solution).expect("Failed to insert solution");

        // Verify data exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM problems", [], |row| row.get(0))
            .expect("Failed to count problems");
        assert_eq!(count, 1);

        // Clear all tables
        clear_all_tables(conn).expect("Failed to clear tables");

        // Verify data is gone
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM problems", [], |row| row.get(0))
            .expect("Failed to count problems");
        assert_eq!(count, 0);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM solutions", [], |row| row.get(0))
            .expect("Failed to count solutions");
        assert_eq!(count, 0);
    }
}
