//! Integration tests for SQLite storage layer
//!
//! These tests verify the full workflow of the SQLite storage layer including:
//! - Entity CRUD operations
//! - Full-text search indexing and querying
//! - Referential integrity validation
//! - Dirty flag management for sync operations

use chrono::Utc;
use jjj::db::{self, Database};
use jjj::models::{Problem, ProblemStatus, Priority, Solution, SolutionStatus};

/// Test a full sync cycle: insert entities, validate, and search.
#[test]
fn test_full_sync_cycle() {
    let db = Database::open_in_memory().unwrap();

    // Insert a problem
    let problem = Problem {
        id: "p1".to_string(),
        title: "Test problem".to_string(),
        status: ProblemStatus::Open,
        priority: Priority::High,
        parent_id: None,
        milestone_id: None,
        assignee: Some("alice".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        description: "Test description".to_string(),
        context: "".to_string(),
        dissolved_reason: None,
        solution_ids: vec![],
        child_ids: vec![],
    };
    db::entities::upsert_problem(db.conn(), &problem).unwrap();

    // Insert a solution
    let solution = Solution {
        id: "s1".to_string(),
        title: "Test solution".to_string(),
        status: SolutionStatus::Proposed,
        problem_id: "p1".to_string(),
        change_ids: vec!["abc123".to_string()],
        supersedes: None,
        assignee: None,
        force_accepted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        approach: "Do the thing".to_string(),
        tradeoffs: "".to_string(),
        critique_ids: vec![],
    };
    db::entities::upsert_solution(db.conn(), &solution).unwrap();

    // Validate - should pass since all references are valid
    let errors = db::validate(&db).unwrap();
    assert!(errors.is_empty(), "Expected no validation errors, got: {:?}", errors);

    // Manually populate FTS for search testing
    // (In production, this would be done via rebuild_fts after load_from_markdown)
    db.conn()
        .execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["problem", "p1", "Test problem", "Test description"],
        )
        .unwrap();

    // Search for the problem
    let results = db::search(db.conn(), "test", None).unwrap();
    assert_eq!(results.len(), 1, "Expected 1 search result, got {}", results.len());
    assert_eq!(results[0].entity_id, "p1");
    assert_eq!(results[0].entity_type, "problem");
}

/// Test that validation catches invalid foreign key references.
#[test]
fn test_validation_catches_invalid_refs() {
    let db = Database::open_in_memory().unwrap();

    // Disable foreign key constraints so we can insert invalid data
    // (simulates data loaded from corrupted markdown files)
    db.conn()
        .execute("PRAGMA foreign_keys = OFF", [])
        .unwrap();

    // Create solution with invalid problem reference
    let solution = Solution {
        id: "s1".to_string(),
        title: "Bad solution".to_string(),
        status: SolutionStatus::Proposed,
        problem_id: "p_invalid".to_string(), // This problem doesn't exist
        change_ids: vec![],
        supersedes: None,
        assignee: None,
        force_accepted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        approach: "".to_string(),
        tradeoffs: "".to_string(),
        critique_ids: vec![],
    };
    db::entities::upsert_solution(db.conn(), &solution).unwrap();

    // Validation should catch the invalid reference
    let errors = db::validate(&db).unwrap();
    assert_eq!(errors.len(), 1, "Expected 1 validation error, got {:?}", errors);
    assert!(
        errors[0].message.contains("p_invalid"),
        "Error message should mention the invalid reference: {}",
        errors[0].message
    );
    assert!(
        errors[0].message.contains("non-existent problem"),
        "Error message should indicate non-existent problem: {}",
        errors[0].message
    );
}

/// Test that the dirty flag correctly tracks uncommitted changes.
#[test]
fn test_dirty_flag() {
    let db = Database::open_in_memory().unwrap();

    // Initially clean
    assert!(
        !db::is_dirty(&db).unwrap(),
        "Database should not be dirty initially"
    );

    // Set dirty
    db::set_dirty(&db, true).unwrap();
    assert!(
        db::is_dirty(&db).unwrap(),
        "Database should be dirty after setting flag"
    );

    // Clear dirty
    db::set_dirty(&db, false).unwrap();
    assert!(
        !db::is_dirty(&db).unwrap(),
        "Database should not be dirty after clearing flag"
    );
}

/// Test loading and querying a problem with all fields populated.
#[test]
fn test_problem_roundtrip() {
    let db = Database::open_in_memory().unwrap();
    let now = Utc::now();

    let problem = Problem {
        id: "p1".to_string(),
        title: "Complex problem".to_string(),
        status: ProblemStatus::InProgress,
        priority: Priority::Critical,
        parent_id: None,
        milestone_id: None,
        assignee: Some("bob".to_string()),
        created_at: now,
        updated_at: now,
        description: "Detailed description of the problem".to_string(),
        context: "Why this is difficult".to_string(),
        dissolved_reason: None,
        solution_ids: vec!["s1".to_string(), "s2".to_string()],
        child_ids: vec!["p2".to_string()],
    };

    db::entities::upsert_problem(db.conn(), &problem).unwrap();

    let loaded = db::entities::load_problem(db.conn(), "p1")
        .unwrap()
        .expect("Problem should exist");

    assert_eq!(loaded.id, "p1");
    assert_eq!(loaded.title, "Complex problem");
    assert_eq!(loaded.status, ProblemStatus::InProgress);
    assert_eq!(loaded.priority, Priority::Critical);
    assert_eq!(loaded.assignee, Some("bob".to_string()));
    assert_eq!(loaded.description, "Detailed description of the problem");
    assert_eq!(loaded.context, "Why this is difficult");
}

/// Test that search respects entity type filter.
#[test]
fn test_search_with_entity_type_filter() {
    let db = Database::open_in_memory().unwrap();

    // Insert a problem and a solution
    let problem = Problem::new("p1".to_string(), "Authentication bug".to_string());
    db::entities::upsert_problem(db.conn(), &problem).unwrap();

    let solution = Solution::new(
        "s1".to_string(),
        "Fix authentication flow".to_string(),
        "p1".to_string(),
    );
    db::entities::upsert_solution(db.conn(), &solution).unwrap();

    // Manually populate FTS
    db.conn()
        .execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["problem", "p1", "Authentication bug", ""],
        )
        .unwrap();
    db.conn()
        .execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["solution", "s1", "Fix authentication flow", ""],
        )
        .unwrap();

    // Search all - should find both
    let results = db::search(db.conn(), "authentication", None).unwrap();
    assert_eq!(results.len(), 2, "Expected 2 results without filter");

    // Search only problems
    let results = db::search(db.conn(), "authentication", Some("problem")).unwrap();
    assert_eq!(results.len(), 1, "Expected 1 result with problem filter");
    assert_eq!(results[0].entity_type, "problem");

    // Search only solutions
    let results = db::search(db.conn(), "authentication", Some("solution")).unwrap();
    assert_eq!(results.len(), 1, "Expected 1 result with solution filter");
    assert_eq!(results[0].entity_type, "solution");
}

/// Test validation with valid parent-child problem hierarchy.
#[test]
fn test_valid_problem_hierarchy() {
    let db = Database::open_in_memory().unwrap();

    // Create parent problem
    let parent = Problem::new("p1".to_string(), "Parent problem".to_string());
    db::entities::upsert_problem(db.conn(), &parent).unwrap();

    // Create child problem with valid parent reference
    let mut child = Problem::new("p2".to_string(), "Child problem".to_string());
    child.parent_id = Some("p1".to_string());
    db::entities::upsert_problem(db.conn(), &child).unwrap();

    // Validation should pass
    let errors = db::validate(&db).unwrap();
    assert!(
        errors.is_empty(),
        "Expected no validation errors for valid hierarchy, got: {:?}",
        errors
    );
}

/// Test that deleting an entity works correctly.
#[test]
fn test_entity_deletion() {
    let db = Database::open_in_memory().unwrap();

    // Insert a problem
    let problem = Problem::new("p1".to_string(), "To be deleted".to_string());
    db::entities::upsert_problem(db.conn(), &problem).unwrap();

    // Verify it exists
    let loaded = db::entities::load_problem(db.conn(), "p1").unwrap();
    assert!(loaded.is_some(), "Problem should exist before deletion");

    // Delete it
    let deleted = db::entities::delete_problem(db.conn(), "p1").unwrap();
    assert!(deleted, "Delete should return true when row was deleted");

    // Verify it's gone
    let loaded = db::entities::load_problem(db.conn(), "p1").unwrap();
    assert!(loaded.is_none(), "Problem should not exist after deletion");

    // Deleting again should return false
    let deleted = db::entities::delete_problem(db.conn(), "p1").unwrap();
    assert!(!deleted, "Delete should return false when no row exists");
}

/// Test listing solutions for a specific problem.
#[test]
fn test_list_solutions_for_problem() {
    let db = Database::open_in_memory().unwrap();

    // Create two problems
    let p1 = Problem::new("p1".to_string(), "Problem 1".to_string());
    let p2 = Problem::new("p2".to_string(), "Problem 2".to_string());
    db::entities::upsert_problem(db.conn(), &p1).unwrap();
    db::entities::upsert_problem(db.conn(), &p2).unwrap();

    // Create solutions for p1
    let s1 = Solution::new("s1".to_string(), "Solution 1".to_string(), "p1".to_string());
    let s2 = Solution::new("s2".to_string(), "Solution 2".to_string(), "p1".to_string());
    db::entities::upsert_solution(db.conn(), &s1).unwrap();
    db::entities::upsert_solution(db.conn(), &s2).unwrap();

    // Create solution for p2
    let s3 = Solution::new("s3".to_string(), "Solution 3".to_string(), "p2".to_string());
    db::entities::upsert_solution(db.conn(), &s3).unwrap();

    // List solutions for p1 - should get s1 and s2
    let solutions = db::entities::list_solutions_for_problem(db.conn(), "p1").unwrap();
    assert_eq!(solutions.len(), 2, "Expected 2 solutions for p1");

    // List solutions for p2 - should get s3
    let solutions = db::entities::list_solutions_for_problem(db.conn(), "p2").unwrap();
    assert_eq!(solutions.len(), 1, "Expected 1 solution for p2");
    assert_eq!(solutions[0].id, "s3");
}
