//! Referential integrity validation for jjj SQLite storage.
//!
//! Before pushing to remote, we validate that all foreign key relationships
//! are intact: solutions reference existing problems, critiques reference
//! existing solutions, etc.

use rusqlite::{params, Result as SqliteResult};
use std::collections::HashSet;

use super::Database;

/// A validation error indicating a broken reference.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub entity_id: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.entity_id, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validate referential integrity of all entities in the database.
///
/// Checks:
/// 1. For each problem with parent_id, parent must exist in problems
/// 2. For each problem with milestone_id, milestone must exist in milestones
/// 3. For each solution, problem_id must exist in problems
/// 4. For each solution with supersedes, that solution must exist
/// 5. For each critique, solution_id must exist in solutions
/// 6. No cycles in problem parent chain
///
/// Returns a list of all validation errors found.
pub fn validate(db: &Database) -> SqliteResult<Vec<ValidationError>> {
    let conn = db.conn();
    let mut errors = Vec::new();

    // Collect all valid IDs for existence checks
    let problem_ids: HashSet<String> = conn
        .prepare("SELECT id FROM problems")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<_>, _>>()?;

    let solution_ids: HashSet<String> = conn
        .prepare("SELECT id FROM solutions")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<_>, _>>()?;

    let milestone_ids: HashSet<String> = conn
        .prepare("SELECT id FROM milestones")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<_>, _>>()?;

    // 1. Check problem parent_id references
    let mut stmt =
        conn.prepare("SELECT id, parent_id FROM problems WHERE parent_id IS NOT NULL")?;
    let parent_refs = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for result in parent_refs {
        let (problem_id, parent_id) = result?;
        if !problem_ids.contains(&parent_id) {
            errors.push(ValidationError {
                entity_id: problem_id,
                message: format!("references non-existent parent problem '{}'", parent_id),
            });
        }
    }

    // 2. Check problem milestone_id references
    let mut stmt =
        conn.prepare("SELECT id, milestone_id FROM problems WHERE milestone_id IS NOT NULL")?;
    let milestone_refs = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for result in milestone_refs {
        let (problem_id, milestone_id) = result?;
        if !milestone_ids.contains(&milestone_id) {
            errors.push(ValidationError {
                entity_id: problem_id,
                message: format!("references non-existent milestone '{}'", milestone_id),
            });
        }
    }

    // 3. Check solution problem_id references
    let mut stmt = conn.prepare("SELECT id, problem_id FROM solutions")?;
    let problem_refs = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for result in problem_refs {
        let (solution_id, problem_id) = result?;
        if !problem_ids.contains(&problem_id) {
            errors.push(ValidationError {
                entity_id: solution_id,
                message: format!("references non-existent problem '{}'", problem_id),
            });
        }
    }

    // 4. Check solution supersedes references
    let mut stmt =
        conn.prepare("SELECT id, supersedes FROM solutions WHERE supersedes IS NOT NULL")?;
    let supersedes_refs = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for result in supersedes_refs {
        let (solution_id, supersedes_id) = result?;
        if !solution_ids.contains(&supersedes_id) {
            errors.push(ValidationError {
                entity_id: solution_id,
                message: format!("supersedes non-existent solution '{}'", supersedes_id),
            });
        }
    }

    // 5. Check critique solution_id references
    let mut stmt = conn.prepare("SELECT id, solution_id FROM critiques")?;
    let solution_refs = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for result in solution_refs {
        let (critique_id, solution_id) = result?;
        if !solution_ids.contains(&solution_id) {
            errors.push(ValidationError {
                entity_id: critique_id,
                message: format!("references non-existent solution '{}'", solution_id),
            });
        }
    }

    // 6. Check for cycles in problem parent chain
    for problem_id in &problem_ids {
        if has_parent_cycle(conn, problem_id, &problem_ids)? {
            errors.push(ValidationError {
                entity_id: problem_id.clone(),
                message: "has a cycle in its parent chain".to_string(),
            });
        }
    }

    Ok(errors)
}

/// Check if a problem has a cycle in its parent chain.
///
/// Follows the parent_id chain from start_id, tracking visited IDs.
/// Returns true if we revisit an ID (cycle detected).
fn has_parent_cycle(
    conn: &rusqlite::Connection,
    start_id: &str,
    valid_ids: &HashSet<String>,
) -> SqliteResult<bool> {
    let mut visited = HashSet::new();
    let mut current_id = start_id.to_string();

    loop {
        // If we've seen this ID before, we have a cycle
        if !visited.insert(current_id.clone()) {
            return Ok(true);
        }

        // Get the parent_id for the current problem
        let parent_id: Option<String> = conn.query_row(
            "SELECT parent_id FROM problems WHERE id = ?1",
            params![current_id],
            |row| row.get(0),
        )?;

        match parent_id {
            Some(pid) => {
                // If the parent doesn't exist in valid_ids, stop (separate validation catches this)
                if !valid_ids.contains(&pid) {
                    return Ok(false);
                }
                current_id = pid;
            }
            None => {
                // No more parents, no cycle
                return Ok(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::entities::{upsert_problem, upsert_solution};
    use crate::models::{Problem, Solution};
    use chrono::Utc;
    use rusqlite::params;

    /// Helper to disable foreign key constraints for testing validation logic.
    /// This simulates data that might be loaded from corrupted markdown files.
    fn disable_foreign_keys(conn: &rusqlite::Connection) {
        conn.execute("PRAGMA foreign_keys = OFF", [])
            .expect("Failed to disable foreign keys");
    }

    /// Helper to insert a raw problem for testing (bypassing model validation).
    fn insert_raw_problem(
        conn: &rusqlite::Connection,
        id: &str,
        title: &str,
        parent_id: Option<&str>,
        milestone_id: Option<&str>,
    ) {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO problems (id, title, status, priority, parent_id, milestone_id, created_at, updated_at)
             VALUES (?1, ?2, 'open', 'medium', ?3, ?4, ?5, ?6)",
            params![id, title, parent_id, milestone_id, now, now],
        )
        .expect("Failed to insert raw problem");
    }

    /// Helper to insert a raw solution for testing.
    fn insert_raw_solution(
        conn: &rusqlite::Connection,
        id: &str,
        title: &str,
        problem_id: &str,
        supersedes: Option<&str>,
    ) {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO solutions (id, title, status, problem_id, supersedes, force_approved, created_at, updated_at)
             VALUES (?1, ?2, 'proposed', ?3, ?4, 0, ?5, ?6)",
            params![id, title, problem_id, supersedes, now, now],
        )
        .expect("Failed to insert raw solution");
    }

    /// Helper to insert a raw critique for testing.
    fn insert_raw_critique(conn: &rusqlite::Connection, id: &str, title: &str, solution_id: &str) {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO critiques (id, title, status, solution_id, severity, created_at, updated_at)
             VALUES (?1, ?2, 'open', ?3, 'medium', ?4, ?5)",
            params![id, title, solution_id, now, now],
        )
        .expect("Failed to insert raw critique");
    }

    #[test]
    fn test_validation_catches_missing_refs() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create a solution that references a non-existent problem
        insert_raw_solution(conn, "s1", "Orphan solution", "p999", None);

        let errors = validate(&db).expect("Validation failed");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].entity_id, "s1");
        assert!(errors[0].message.contains("p999"));
        assert!(errors[0].message.contains("non-existent problem"));
    }

    #[test]
    fn test_validation_passes_for_valid_data() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create a valid problem
        let problem = Problem::new("p1".to_string(), "Test problem".to_string());
        upsert_problem(conn, &problem).expect("Failed to insert problem");

        // Create a valid solution referencing the problem
        let solution = Solution::new(
            "s1".to_string(),
            "Valid solution".to_string(),
            "p1".to_string(),
        );
        upsert_solution(conn, &solution).expect("Failed to insert solution");

        let errors = validate(&db).expect("Validation failed");

        assert!(
            errors.is_empty(),
            "Expected no validation errors but got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_catches_missing_parent_problem() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create a problem with a non-existent parent
        insert_raw_problem(conn, "p1", "Child problem", Some("p999"), None);

        let errors = validate(&db).expect("Validation failed");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].entity_id, "p1");
        assert!(errors[0].message.contains("parent problem"));
        assert!(errors[0].message.contains("p999"));
    }

    #[test]
    fn test_validation_catches_missing_milestone() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create a problem with a non-existent milestone
        insert_raw_problem(conn, "p1", "Orphan problem", None, Some("m999"));

        let errors = validate(&db).expect("Validation failed");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].entity_id, "p1");
        assert!(errors[0].message.contains("milestone"));
        assert!(errors[0].message.contains("m999"));
    }

    #[test]
    fn test_validation_catches_missing_supersedes() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create a valid problem first
        insert_raw_problem(conn, "p1", "Test problem", None, None);

        // Create a solution that supersedes a non-existent solution
        insert_raw_solution(conn, "s1", "Superseding solution", "p1", Some("s999"));

        let errors = validate(&db).expect("Validation failed");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].entity_id, "s1");
        assert!(errors[0].message.contains("supersedes"));
        assert!(errors[0].message.contains("s999"));
    }

    #[test]
    fn test_validation_catches_missing_solution_for_critique() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create a critique that references a non-existent solution
        insert_raw_critique(conn, "c1", "Orphan critique", "s999");

        let errors = validate(&db).expect("Validation failed");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].entity_id, "c1");
        assert!(errors[0].message.contains("solution"));
        assert!(errors[0].message.contains("s999"));
    }

    #[test]
    fn test_validation_catches_parent_cycle() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create problems with a cycle: p1 -> p2 -> p3 -> p1
        // We need to insert them all first, then we can have the cycle
        insert_raw_problem(conn, "p1", "Problem 1", Some("p3"), None);
        insert_raw_problem(conn, "p2", "Problem 2", Some("p1"), None);
        insert_raw_problem(conn, "p3", "Problem 3", Some("p2"), None);

        let errors = validate(&db).expect("Validation failed");

        // All three problems are part of the cycle
        let cycle_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("cycle"))
            .collect();
        assert_eq!(
            cycle_errors.len(),
            3,
            "Expected 3 cycle errors, got: {:?}",
            cycle_errors
        );
    }

    #[test]
    fn test_validation_valid_parent_chain() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create a valid parent chain: p3 -> p2 -> p1 (no cycle)
        let p1 = Problem::new("p1".to_string(), "Root problem".to_string());
        let mut p2 = Problem::new("p2".to_string(), "Child problem".to_string());
        let mut p3 = Problem::new("p3".to_string(), "Grandchild problem".to_string());

        p2.parent_id = Some("p1".to_string());
        p3.parent_id = Some("p2".to_string());

        upsert_problem(conn, &p1).expect("Failed to insert p1");
        upsert_problem(conn, &p2).expect("Failed to insert p2");
        upsert_problem(conn, &p3).expect("Failed to insert p3");

        let errors = validate(&db).expect("Validation failed");

        assert!(
            errors.is_empty(),
            "Expected no errors but got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validation_multiple_errors() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();
        disable_foreign_keys(conn);

        // Create multiple validation errors
        insert_raw_problem(conn, "p1", "Problem", Some("p999"), Some("m999"));
        insert_raw_solution(conn, "s1", "Solution", "p888", None);

        let errors = validate(&db).expect("Validation failed");

        // Should have 3 errors: missing parent, missing milestone, missing problem for solution
        assert_eq!(errors.len(), 3, "Expected 3 errors but got: {:?}", errors);
    }
}
