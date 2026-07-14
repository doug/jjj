//! CRUD operations for jjj entities in SQLite.
//!
//! All row parsing is defensive: invalid datetime strings or unrecognised enum
//! values emit a `Warning:` to stderr and fall back to a safe default rather
//! than failing.
//!
//! Derived back-references (`solution_ids`, `critique_ids`, `problem_ids`) are
//! left empty by the `row_to_*` functions; the storage wrappers attach them from
//! forward references at read time (Pillar 4), not this layer.
//!
//! This module provides functions to store, retrieve, and delete
//! Problems, Solutions, Critiques, and Milestones from the SQLite database.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};

use crate::models::{Confidence, Critique, Milestone, Priority, Problem, Solution};

// ============================================================================
// Row parsing helpers
// ============================================================================

fn parse_datetime(s: &str, field: &str, entity: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|e| {
            eprintln!(
                "Warning: invalid {} '{}' for {} row: {}",
                field, s, entity, e
            );
            Utc::now()
        })
}

fn parse_json_vec<T: serde::de::DeserializeOwned>(json: &str, field: &str) -> Vec<T> {
    serde_json::from_str(json).unwrap_or_else(|e| {
        eprintln!("Warning: invalid {} JSON: {}", field, e);
        Vec::new()
    })
}

fn parse_enum<T: std::str::FromStr + Default>(s: &str, kind: &str, default_name: &str) -> T {
    s.parse().unwrap_or_else(|_| {
        eprintln!(
            "Warning: invalid {} '{}', defaulting to {}",
            kind, s, default_name
        );
        T::default()
    })
}

// ============================================================================
// Generic CRUD scaffolding
// ============================================================================

/// Per-table data needed to drive the generic `load_one` / `list_all` /
/// `delete_one` helpers below, removing the four-way `load_*`/`list_*`/`delete_*`
/// boilerplate. The per-type `upsert_*` and `row_to_*` stay, since their SQL is
/// irreducibly schema-specific.
///
/// `COLUMNS` MUST list columns in the exact order `from_row` decodes them.
trait DbEntity: Sized {
    const TABLE: &'static str;
    const COLUMNS: &'static str;
    fn from_row(row: &rusqlite::Row) -> SqliteResult<Self>;
}

/// Load a single entity by id (`None` if absent).
fn load_one<T: DbEntity>(conn: &Connection, id: &str) -> SqliteResult<Option<T>> {
    let sql = format!("SELECT {} FROM {} WHERE id = ?1", T::COLUMNS, T::TABLE);
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(T::from_row(row)?)),
        None => Ok(None),
    }
}

/// List all entities of a type, newest first.
fn list_all<T: DbEntity>(conn: &Connection) -> SqliteResult<Vec<T>> {
    let sql = format!(
        "SELECT {} FROM {} ORDER BY created_at DESC",
        T::COLUMNS,
        T::TABLE
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], T::from_row)?;
    rows.collect()
}

/// Delete an entity by id. Returns true if a row was removed.
fn delete_one<T: DbEntity>(conn: &Connection, id: &str) -> SqliteResult<bool> {
    let sql = format!("DELETE FROM {} WHERE id = ?1", T::TABLE);
    Ok(conn.execute(&sql, params![id])? > 0)
}

impl DbEntity for Problem {
    const TABLE: &'static str = "problems";
    const COLUMNS: &'static str = "id, title, status, priority, confidence, parent_id, \
        milestone_id, assignee, created_at, updated_at, description, dissolved_reason, \
        github_issue, tags";
    fn from_row(row: &rusqlite::Row) -> SqliteResult<Self> {
        row_to_problem(row)
    }
}

impl DbEntity for Solution {
    const TABLE: &'static str = "solutions";
    const COLUMNS: &'static str = "id, title, status, problem_id, change_ids, supersedes, \
        assignee, force_approved, created_at, updated_at, approach, github_pr, github_branch, tags";
    fn from_row(row: &rusqlite::Row) -> SqliteResult<Self> {
        row_to_solution(row)
    }
}

impl DbEntity for Critique {
    const TABLE: &'static str = "critiques";
    const COLUMNS: &'static str = "id, title, status, solution_id, severity, reviewer, author, \
        file_path, line_number, created_at, updated_at, argument, replies, github_review_id";
    fn from_row(row: &rusqlite::Row) -> SqliteResult<Self> {
        row_to_critique(row)
    }
}

impl DbEntity for Milestone {
    const TABLE: &'static str = "milestones";
    const COLUMNS: &'static str =
        "id, title, status, target_date, assignee, created_at, updated_at, description, problem_ids";
    fn from_row(row: &rusqlite::Row) -> SqliteResult<Self> {
        row_to_milestone(row)
    }
}

// ============================================================================
// Problems
// ============================================================================

/// Insert or update a problem in the database.
pub fn upsert_problem(conn: &Connection, problem: &Problem) -> SqliteResult<()> {
    let tags_json = serde_json::to_string(&problem.tags).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO problems (
            id, title, status, priority, confidence, parent_id, milestone_id, assignee,
            created_at, updated_at, description, dissolved_reason, github_issue, tags
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            problem.id,
            problem.title,
            problem.status.to_string(),
            match problem.priority {
                Priority::Low => "low",
                Priority::Medium => "medium",
                Priority::High => "high",
                Priority::Critical => "critical",
            },
            match problem.confidence {
                Confidence::Unknown => "unknown",
                Confidence::Red => "red",
                Confidence::Amber => "amber",
                Confidence::Green => "green",
            },
            problem.parent_id,
            problem.milestone_id,
            problem.assignee,
            problem.created_at.to_rfc3339(),
            problem.updated_at.to_rfc3339(),
            problem.description,
            problem.dissolved_reason,
            problem.github_issue.map(|n| n as i64),
            tags_json,
        ],
    )?;
    Ok(())
}

/// Load a problem by ID.
pub fn load_problem(conn: &Connection, id: &str) -> SqliteResult<Option<Problem>> {
    load_one(conn, id)
}

/// List all problems, newest first.
pub fn list_problems(conn: &Connection) -> SqliteResult<Vec<Problem>> {
    list_all(conn)
}

/// Delete a problem by ID. Returns true if a row was deleted.
pub fn delete_problem(conn: &Connection, id: &str) -> SqliteResult<bool> {
    delete_one::<Problem>(conn, id)
}

fn row_to_problem(row: &rusqlite::Row) -> SqliteResult<Problem> {
    // Column order: id(0), title(1), status(2), priority(3), confidence(4),
    //   parent_id(5), milestone_id(6), assignee(7),
    //   created_at(8), updated_at(9), description(10),
    //   dissolved_reason(11), github_issue(12), tags(13)
    let status_str: String = row.get(2)?;
    let priority_str: String = row.get(3)?;
    let confidence_str: String = row
        .get::<_, Option<String>>(4)?
        .unwrap_or_else(|| "unknown".to_string());
    let created_at_str: String = row.get(8)?;
    let updated_at_str: String = row.get(9)?;
    let tags_json: String = row
        .get::<_, Option<String>>(13)?
        .unwrap_or_else(|| "[]".to_string());

    Ok(Problem {
        id: row.get(0)?,
        title: row.get(1)?,
        status: parse_enum(&status_str, "problem status", "Open"),
        priority: parse_enum(&priority_str, "priority", "Medium"),
        confidence: parse_enum(&confidence_str, "confidence", "Unknown"),
        parent_id: row.get(5)?,
        milestone_id: row.get(6)?,
        assignee: row.get(7)?,
        created_at: parse_datetime(&created_at_str, "created_at", "problem"),
        updated_at: parse_datetime(&updated_at_str, "updated_at", "problem"),
        description: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
        dissolved_reason: row.get(11)?,
        github_issue: row.get::<_, Option<i64>>(12)?.map(|n| n as u64),
        tags: parse_json_vec(&tags_json, "tags"),
        // Derived back-reference — left empty; attached by the storage wrapper.
        solution_ids: Vec::new(),
    })
}

// ============================================================================
// Solutions
// ============================================================================

/// Insert or update a solution in the database.
pub fn upsert_solution(conn: &Connection, solution: &Solution) -> SqliteResult<()> {
    let change_ids_json =
        serde_json::to_string(&solution.change_ids).unwrap_or_else(|_| "[]".to_string());
    let tags_json = serde_json::to_string(&solution.tags).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO solutions (
            id, title, status, problem_id, change_ids, supersedes, assignee,
            force_approved, created_at, updated_at, approach,
            github_pr, github_branch, tags
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            solution.id,
            solution.title,
            solution.status.to_string(),
            solution.problem_id,
            change_ids_json,
            solution.supersedes,
            solution.assignee,
            solution.force_approved,
            solution.created_at.to_rfc3339(),
            solution.updated_at.to_rfc3339(),
            solution.approach,
            solution.github_pr.map(|n| n as i64),
            solution.github_branch,
            tags_json,
        ],
    )?;
    Ok(())
}

/// Load a solution by ID.
pub fn load_solution(conn: &Connection, id: &str) -> SqliteResult<Option<Solution>> {
    load_one(conn, id)
}

/// List all solutions, newest first.
pub fn list_solutions(conn: &Connection) -> SqliteResult<Vec<Solution>> {
    list_all(conn)
}

/// List solutions for a specific problem.
pub fn list_solutions_for_problem(
    conn: &Connection,
    problem_id: &str,
) -> SqliteResult<Vec<Solution>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, problem_id, change_ids, supersedes, assignee,
                force_approved, created_at, updated_at, approach,
                github_pr, github_branch, tags
         FROM solutions WHERE problem_id = ?1 ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map(params![problem_id], row_to_solution)?;

    rows.collect()
}

/// Delete a solution by ID. Returns true if a row was deleted.
pub fn delete_solution(conn: &Connection, id: &str) -> SqliteResult<bool> {
    delete_one::<Solution>(conn, id)
}

fn row_to_solution(row: &rusqlite::Row) -> SqliteResult<Solution> {
    // Column order: id(0), title(1), status(2), problem_id(3), change_ids(4),
    //   supersedes(5), assignee(6), force_approved(7),
    //   created_at(8), updated_at(9), approach(10),
    //   github_pr(11), github_branch(12), tags(13)
    let status_str: String = row.get(2)?;
    let change_ids_json: String = row
        .get::<_, Option<String>>(4)?
        .unwrap_or_else(|| "[]".to_string());
    let created_at_str: String = row.get(8)?;
    let updated_at_str: String = row.get(9)?;
    let tags_json: String = row
        .get::<_, Option<String>>(13)?
        .unwrap_or_else(|| "[]".to_string());

    Ok(Solution {
        id: row.get(0)?,
        title: row.get(1)?,
        status: parse_enum(&status_str, "solution status", "Proposed"),
        problem_id: row.get(3)?,
        change_ids: parse_json_vec(&change_ids_json, "change_ids"),
        supersedes: row.get(5)?,
        assignee: row.get(6)?,
        force_approved: row.get(7)?,
        created_at: parse_datetime(&created_at_str, "created_at", "solution"),
        updated_at: parse_datetime(&updated_at_str, "updated_at", "solution"),
        approach: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
        github_pr: row.get::<_, Option<i64>>(11)?.map(|n| n as u64),
        github_branch: row.get(12)?,
        tags: parse_json_vec(&tags_json, "tags"),
        // Derived back-reference — left empty; attached by the storage wrapper.
        critique_ids: Vec::new(),
    })
}

// ============================================================================
// Critiques
// ============================================================================

/// Insert or update a critique in the database.
pub fn upsert_critique(conn: &Connection, critique: &Critique) -> SqliteResult<()> {
    let replies_json =
        serde_json::to_string(&critique.replies).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO critiques (
            id, title, status, solution_id, severity, reviewer, author, file_path,
            line_number, created_at, updated_at, argument, replies,
            github_review_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            critique.id,
            critique.title,
            critique.status.to_string(),
            critique.solution_id,
            critique.severity.to_string(),
            critique.reviewer,
            critique.author,
            critique.file_path,
            critique.line_start.map(|n| n as i64),
            critique.created_at.to_rfc3339(),
            critique.updated_at.to_rfc3339(),
            critique.argument,
            replies_json,
            critique.github_review_id.map(|n| n as i64),
        ],
    )?;
    Ok(())
}

/// Load a critique by ID.
pub fn load_critique(conn: &Connection, id: &str) -> SqliteResult<Option<Critique>> {
    load_one(conn, id)
}

/// List all critiques, newest first.
pub fn list_critiques(conn: &Connection) -> SqliteResult<Vec<Critique>> {
    list_all(conn)
}

/// List critiques for a specific solution.
pub fn list_critiques_for_solution(
    conn: &Connection,
    solution_id: &str,
) -> SqliteResult<Vec<Critique>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, solution_id, severity, reviewer, author, file_path,
                line_number, created_at, updated_at, argument, replies,
                github_review_id
         FROM critiques WHERE solution_id = ?1 ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map(params![solution_id], row_to_critique)?;

    rows.collect()
}

/// Delete a critique by ID. Returns true if a row was deleted.
pub fn delete_critique(conn: &Connection, id: &str) -> SqliteResult<bool> {
    delete_one::<Critique>(conn, id)
}

fn row_to_critique(row: &rusqlite::Row) -> SqliteResult<Critique> {
    // Column order: id(0), title(1), status(2), solution_id(3), severity(4),
    //   reviewer(5), author(6), file_path(7), line_number(8),
    //   created_at(9), updated_at(10), argument(11), replies(12),
    //   github_review_id(13)
    let status_str: String = row.get(2)?;
    let severity_str: String = row.get(4)?;
    let created_at_str: String = row.get(9)?;
    let updated_at_str: String = row.get(10)?;
    let replies_json: String = row
        .get::<_, Option<String>>(12)?
        .unwrap_or_else(|| "[]".to_string());

    Ok(Critique {
        id: row.get(0)?,
        title: row.get(1)?,
        status: parse_enum(&status_str, "critique status", "Open"),
        solution_id: row.get(3)?,
        severity: parse_enum(&severity_str, "critique severity", "Medium"),
        reviewer: row.get(5)?,
        author: row.get(6)?,
        file_path: row.get(7)?,
        line_start: row.get::<_, Option<i64>>(8)?.map(|n| n as usize),
        line_end: row.get::<_, Option<i64>>(8)?.map(|n| n as usize), // DB only stores one line number
        created_at: parse_datetime(&created_at_str, "created_at", "critique"),
        updated_at: parse_datetime(&updated_at_str, "updated_at", "critique"),
        argument: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
        code_context: Vec::new(),   // Not stored in DB
        context_before: Vec::new(), // Not stored in DB
        context_after: Vec::new(),  // Not stored in DB
        replies: parse_json_vec(&replies_json, "replies"),
        github_review_id: row.get::<_, Option<i64>>(13)?.map(|n| n as u64),
    })
}

// ============================================================================
// Milestones
// ============================================================================

/// Insert or update a milestone in the database.
pub fn upsert_milestone(conn: &Connection, milestone: &Milestone) -> SqliteResult<()> {
    let problem_ids_json =
        serde_json::to_string(&milestone.problem_ids).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO milestones (
            id, title, status, target_date, assignee, created_at, updated_at,
            description, problem_ids
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            milestone.id,
            milestone.title,
            milestone.status.to_string(),
            milestone.target_date.map(|dt| dt.to_rfc3339()),
            milestone.assignee,
            milestone.created_at.to_rfc3339(),
            milestone.updated_at.to_rfc3339(),
            milestone.description.clone(),
            problem_ids_json,
        ],
    )?;
    Ok(())
}

/// Load a milestone by ID.
pub fn load_milestone(conn: &Connection, id: &str) -> SqliteResult<Option<Milestone>> {
    load_one(conn, id)
}

/// List all milestones, newest first.
pub fn list_milestones(conn: &Connection) -> SqliteResult<Vec<Milestone>> {
    list_all(conn)
}

/// Delete a milestone by ID. Returns true if a row was deleted.
pub fn delete_milestone(conn: &Connection, id: &str) -> SqliteResult<bool> {
    delete_one::<Milestone>(conn, id)
}

fn row_to_milestone(row: &rusqlite::Row) -> SqliteResult<Milestone> {
    let status_str: String = row.get(2)?;
    let target_date_str: Option<String> = row.get(3)?;
    let created_at_str: String = row.get(5)?;
    let updated_at_str: String = row.get(6)?;
    let description: String = row.get::<_, Option<String>>(7)?.unwrap_or_default();
    let problem_ids_json: String = row
        .get::<_, Option<String>>(8)?
        .unwrap_or_else(|| "[]".to_string());

    Ok(Milestone {
        id: row.get(0)?,
        title: row.get(1)?,
        status: parse_enum(&status_str, "milestone status", "Planning"),
        target_date: target_date_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
        assignee: row.get(4)?,
        created_at: parse_datetime(&created_at_str, "created_at", "milestone"),
        updated_at: parse_datetime(&updated_at_str, "updated_at", "milestone"),
        description,
        problem_ids: parse_json_vec(&problem_ids_json, "problem_ids"),
    })
}

// Derived back-references (`solution_ids`, `critique_ids`, `problem_ids`) are no
// longer populated here: the storage wrappers (`list_problems`/`load_problem`
// etc.) attach them uniformly for both the DB and filesystem read paths via
// `MetadataStore::reverse_ids_batch` / `reverse_ids_for` (Pillar 4).

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{
        CritiqueSeverity, CritiqueStatus, MilestoneStatus, ProblemStatus, SolutionStatus,
    };

    #[test]
    fn test_problem_crud() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create
        let mut problem = Problem::new("p1".to_string(), "Test problem".to_string());
        problem.description = "A test description".to_string();
        problem.priority = Priority::High;
        problem.assignee = Some("alice".to_string());

        upsert_problem(conn, &problem).expect("Failed to upsert problem");

        // Read
        let loaded = load_problem(conn, "p1")
            .expect("Failed to load problem")
            .expect("Problem not found");

        assert_eq!(loaded.id, "p1");
        assert_eq!(loaded.title, "Test problem");
        assert_eq!(loaded.description, "A test description");
        assert_eq!(loaded.priority, Priority::High);
        assert_eq!(loaded.assignee, Some("alice".to_string()));
        assert_eq!(loaded.status, ProblemStatus::Open);

        // Update
        problem.title = "Updated title".to_string();
        problem.set_status(ProblemStatus::InProgress);
        upsert_problem(conn, &problem).expect("Failed to update problem");

        let loaded = load_problem(conn, "p1")
            .expect("Failed to load")
            .expect("Not found");
        assert_eq!(loaded.title, "Updated title");
        assert_eq!(loaded.status, ProblemStatus::InProgress);

        // List
        let problem2 = Problem::new("p2".to_string(), "Another problem".to_string());
        upsert_problem(conn, &problem2).expect("Failed to upsert");

        let problems = list_problems(conn).expect("Failed to list");
        assert_eq!(problems.len(), 2);

        // Delete
        let deleted = delete_problem(conn, "p1").expect("Failed to delete");
        assert!(deleted);

        let deleted_again = delete_problem(conn, "p1").expect("Failed to delete");
        assert!(!deleted_again);

        let loaded = load_problem(conn, "p1").expect("Failed to load");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_solution_crud() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // First create a problem (foreign key)
        let problem = Problem::new("p1".to_string(), "Test problem".to_string());
        upsert_problem(conn, &problem).expect("Failed to upsert problem");

        // Create solution
        let mut solution = Solution::new(
            "s1".to_string(),
            "Test solution".to_string(),
            "p1".to_string(),
        );
        solution.approach = "Use this approach".to_string();
        solution.change_ids = vec!["abc123".to_string(), "def456".to_string()];
        solution.assignee = Some("bob".to_string());

        upsert_solution(conn, &solution).expect("Failed to upsert solution");

        // Read
        let loaded = load_solution(conn, "s1")
            .expect("Failed to load")
            .expect("Not found");

        assert_eq!(loaded.id, "s1");
        assert_eq!(loaded.title, "Test solution");
        assert_eq!(loaded.problem_id, "p1");
        assert_eq!(loaded.approach, "Use this approach");
        assert_eq!(
            loaded.change_ids,
            vec!["abc123".to_string(), "def456".to_string()]
        );
        assert_eq!(loaded.assignee, Some("bob".to_string()));
        assert_eq!(loaded.status, SolutionStatus::Proposed);

        // Update
        solution.set_status(SolutionStatus::Submitted);
        solution.attach_change("ghi789".to_string());
        upsert_solution(conn, &solution).expect("Failed to update");

        let loaded = load_solution(conn, "s1")
            .expect("Failed to load")
            .expect("Not found");
        assert_eq!(loaded.status, SolutionStatus::Submitted);
        assert_eq!(loaded.change_ids.len(), 3);

        // List for problem
        let solution2 = Solution::new(
            "s2".to_string(),
            "Another solution".to_string(),
            "p1".to_string(),
        );
        upsert_solution(conn, &solution2).expect("Failed to upsert");

        let solutions = list_solutions_for_problem(conn, "p1").expect("Failed to list");
        assert_eq!(solutions.len(), 2);

        // List all
        let all_solutions = list_solutions(conn).expect("Failed to list");
        assert_eq!(all_solutions.len(), 2);

        // Delete
        let deleted = delete_solution(conn, "s1").expect("Failed to delete");
        assert!(deleted);

        let loaded = load_solution(conn, "s1").expect("Failed to load");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_critique_crud() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create problem and solution (foreign keys)
        let problem = Problem::new("p1".to_string(), "Test problem".to_string());
        upsert_problem(conn, &problem).expect("Failed to upsert problem");

        let solution = Solution::new(
            "s1".to_string(),
            "Test solution".to_string(),
            "p1".to_string(),
        );
        upsert_solution(conn, &solution).expect("Failed to upsert solution");

        // Create critique
        let mut critique = Critique::new(
            "c1".to_string(),
            "Test critique".to_string(),
            "s1".to_string(),
        );
        critique.argument = "This is problematic".to_string();
        critique.severity = CritiqueSeverity::High;
        critique.author = Some("charlie".to_string());
        critique.reviewer = Some("charlie".to_string());
        critique.file_path = Some("src/main.rs".to_string());
        critique.line_start = Some(42);

        upsert_critique(conn, &critique).expect("Failed to upsert critique");

        // Read
        let loaded = load_critique(conn, "c1")
            .expect("Failed to load")
            .expect("Not found");

        assert_eq!(loaded.id, "c1");
        assert_eq!(loaded.title, "Test critique");
        assert_eq!(loaded.solution_id, "s1");
        assert_eq!(loaded.argument, "This is problematic");
        assert_eq!(loaded.severity, CritiqueSeverity::High);
        assert_eq!(loaded.reviewer, Some("charlie".to_string()));
        assert_eq!(loaded.file_path, Some("src/main.rs".to_string()));
        assert_eq!(loaded.line_start, Some(42));
        assert_eq!(loaded.status, CritiqueStatus::Open);

        // Update with reply
        critique.add_reply("dave".to_string(), "I disagree".to_string());
        critique.set_status(CritiqueStatus::Addressed);
        upsert_critique(conn, &critique).expect("Failed to update");

        let loaded = load_critique(conn, "c1")
            .expect("Failed to load")
            .expect("Not found");
        assert_eq!(loaded.status, CritiqueStatus::Addressed);
        assert_eq!(loaded.replies.len(), 1);
        assert_eq!(loaded.replies[0].author, "dave");
        assert_eq!(loaded.replies[0].body, "I disagree");

        // List for solution
        let critique2 = Critique::new(
            "c2".to_string(),
            "Another critique".to_string(),
            "s1".to_string(),
        );
        upsert_critique(conn, &critique2).expect("Failed to upsert");

        let critiques = list_critiques_for_solution(conn, "s1").expect("Failed to list");
        assert_eq!(critiques.len(), 2);

        // List all
        let all_critiques = list_critiques(conn).expect("Failed to list");
        assert_eq!(all_critiques.len(), 2);

        // Delete
        let deleted = delete_critique(conn, "c1").expect("Failed to delete");
        assert!(deleted);

        let loaded = load_critique(conn, "c1").expect("Failed to load");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_milestone_crud() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Create
        let mut milestone = Milestone::new("m1".to_string(), "v1.0 Release".to_string());
        milestone.description = "Ship the product. All tests pass.".to_string();
        milestone.assignee = Some("eve".to_string());
        milestone.target_date = Some(Utc::now());
        milestone.problem_ids = vec!["p1".to_string(), "p2".to_string()];

        upsert_milestone(conn, &milestone).expect("Failed to upsert milestone");

        // Read
        let loaded = load_milestone(conn, "m1")
            .expect("Failed to load")
            .expect("Not found");

        assert_eq!(loaded.id, "m1");
        assert_eq!(loaded.title, "v1.0 Release");
        assert_eq!(loaded.description, "Ship the product. All tests pass.");
        assert_eq!(loaded.assignee, Some("eve".to_string()));
        assert!(loaded.target_date.is_some());
        assert_eq!(loaded.problem_ids, vec!["p1".to_string(), "p2".to_string()]);
        assert_eq!(loaded.status, MilestoneStatus::Planning);

        // Update
        milestone.set_status(MilestoneStatus::Active);
        milestone.add_problem("p3".to_string());
        upsert_milestone(conn, &milestone).expect("Failed to update");

        let loaded = load_milestone(conn, "m1")
            .expect("Failed to load")
            .expect("Not found");
        assert_eq!(loaded.status, MilestoneStatus::Active);
        assert_eq!(loaded.problem_ids.len(), 3);

        // List
        let milestone2 = Milestone::new("m2".to_string(), "v2.0 Release".to_string());
        upsert_milestone(conn, &milestone2).expect("Failed to upsert");

        let milestones = list_milestones(conn).expect("Failed to list");
        assert_eq!(milestones.len(), 2);

        // Delete
        let deleted = delete_milestone(conn, "m1").expect("Failed to delete");
        assert!(deleted);

        let loaded = load_milestone(conn, "m1").expect("Failed to load");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_problem_dissolved() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        let mut problem = Problem::new("p1".to_string(), "Test problem".to_string());
        problem.dissolve("No longer relevant".to_string());

        upsert_problem(conn, &problem).expect("Failed to upsert");

        let loaded = load_problem(conn, "p1")
            .expect("Failed to load")
            .expect("Not found");

        assert_eq!(loaded.status, ProblemStatus::Dissolved);
        assert_eq!(
            loaded.dissolved_reason,
            Some("No longer relevant".to_string())
        );
    }

    #[test]
    fn test_solution_supersedes() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        let problem = Problem::new("p1".to_string(), "Test problem".to_string());
        upsert_problem(conn, &problem).expect("Failed to upsert problem");

        let s1 = Solution::new(
            "s1".to_string(),
            "First solution".to_string(),
            "p1".to_string(),
        );
        upsert_solution(conn, &s1).expect("Failed to upsert");

        let mut s2 = Solution::new(
            "s2".to_string(),
            "Better solution".to_string(),
            "p1".to_string(),
        );
        s2.supersedes = Some("s1".to_string());
        upsert_solution(conn, &s2).expect("Failed to upsert");

        let loaded = load_solution(conn, "s2")
            .expect("Failed to load")
            .expect("Not found");

        assert_eq!(loaded.supersedes, Some("s1".to_string()));
    }
}
