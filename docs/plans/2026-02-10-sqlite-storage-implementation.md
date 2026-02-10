# SQLite Storage Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace file-based storage with SQLite as runtime source of truth, keeping markdown as sync format.

**Architecture:** SQLite at `.jj/jjj.db` handles all CRUD operations. On `fetch`, rebuild DB from markdown. On `push`, dump DB to markdown and validate. Schema version tracking triggers rebuild on mismatch.

**Tech Stack:** rusqlite with bundled SQLite, existing serde/chrono for serialization.

---

## Task 1: Add rusqlite Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependency**

Add to `[dependencies]` section in `Cargo.toml`:

```toml
# SQLite storage
rusqlite = { version = "0.31", features = ["bundled"] }
```

**Step 2: Verify build**

Run: `cargo build`
Expected: Compiles successfully with new dependency

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add rusqlite dependency"
```

---

## Task 2: Create Database Schema Module

**Files:**
- Create: `src/db/mod.rs`
- Create: `src/db/schema.rs`
- Modify: `src/lib.rs`

**Step 1: Create db module structure**

Create `src/db/mod.rs`:

```rust
pub mod schema;

pub use schema::{Database, SCHEMA_VERSION};
```

**Step 2: Create schema with version tracking**

Create `src/db/schema.rs`:

```rust
use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

pub const SCHEMA_VERSION: u32 = 1;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open database, creating schema if needed
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    /// Open in-memory database (for testing)
    pub fn open_in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    /// Get reference to connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Check schema version, return true if rebuild needed
    pub fn needs_rebuild(&self) -> bool {
        match self.get_schema_version() {
            Ok(version) => version != SCHEMA_VERSION,
            Err(_) => true,
        }
    }

    fn get_schema_version(&self) -> SqliteResult<u32> {
        self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| {
                let v: String = row.get(0)?;
                Ok(v.parse().unwrap_or(0))
            },
        )
    }

    fn ensure_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(include_str!("schema.sql"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        assert!(!db.needs_rebuild());
    }

    #[test]
    fn test_schema_version() {
        let db = Database::open_in_memory().unwrap();
        let version = db.get_schema_version().unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }
}
```

**Step 3: Create SQL schema file**

Create `src/db/schema.sql`:

```sql
-- Meta table for schema version
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '1');
INSERT OR IGNORE INTO meta (key, value) VALUES ('dirty', 'false');

-- Problems table
CREATE TABLE IF NOT EXISTS problems (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT,
    parent_id TEXT REFERENCES problems(id),
    milestone_id TEXT,
    assignee TEXT,
    created_at TEXT,
    updated_at TEXT,
    description TEXT,
    context TEXT,
    dissolved_reason TEXT
);

-- Solutions table
CREATE TABLE IF NOT EXISTS solutions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    problem_id TEXT NOT NULL,
    change_ids TEXT,  -- JSON array
    supersedes TEXT,
    assignee TEXT,
    force_accepted INTEGER DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    approach TEXT,
    tradeoffs TEXT
);

-- Critiques table
CREATE TABLE IF NOT EXISTS critiques (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    solution_id TEXT NOT NULL,
    severity TEXT,
    reviewer TEXT,
    file_path TEXT,
    line_number INTEGER,
    created_at TEXT,
    updated_at TEXT,
    body TEXT,
    replies TEXT  -- JSON array
);

-- Milestones table
CREATE TABLE IF NOT EXISTS milestones (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT,
    target_date TEXT,
    assignee TEXT,
    created_at TEXT,
    updated_at TEXT,
    description TEXT,
    problem_ids TEXT  -- JSON array
);

-- Events table
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    actor TEXT NOT NULL,
    rationale TEXT,
    refs TEXT,  -- JSON array
    extra TEXT  -- JSON object
);

-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(
    entity_type,
    entity_id,
    title,
    body,
    content='',
    tokenize='porter unicode61'
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_entity ON events(entity_id);
CREATE INDEX IF NOT EXISTS idx_solutions_problem ON solutions(problem_id);
CREATE INDEX IF NOT EXISTS idx_critiques_solution ON critiques(solution_id);
CREATE INDEX IF NOT EXISTS idx_problems_milestone ON problems(milestone_id);
CREATE INDEX IF NOT EXISTS idx_problems_parent ON problems(parent_id);
```

**Step 4: Add db module to lib.rs**

In `src/lib.rs`, add after other module declarations:

```rust
pub mod db;
```

**Step 5: Run tests**

Run: `cargo test db::schema`
Expected: 2 tests pass

**Step 6: Commit**

```bash
git add src/db src/lib.rs
git commit -m "feat(db): add SQLite schema with version tracking"
```

---

## Task 3: Implement Entity CRUD Operations

**Files:**
- Create: `src/db/entities.rs`
- Modify: `src/db/mod.rs`

**Step 1: Write failing test for problem insert/load**

Add to `src/db/entities.rs`:

```rust
use crate::error::Result;
use crate::models::{
    Critique, CritiqueSeverity, CritiqueStatus, Milestone, MilestoneStatus,
    Problem, ProblemStatus, Priority, Solution, SolutionStatus,
};
use rusqlite::{params, Connection};

/// Insert or update a problem
pub fn upsert_problem(conn: &Connection, problem: &Problem) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO problems
         (id, title, status, priority, parent_id, milestone_id, assignee,
          created_at, updated_at, description, context, dissolved_reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            problem.id,
            problem.title,
            problem.status.to_string(),
            problem.priority.to_string(),
            problem.parent_id,
            problem.milestone_id,
            problem.assignee,
            problem.created_at.map(|d| d.to_rfc3339()),
            problem.updated_at.map(|d| d.to_rfc3339()),
            problem.description,
            problem.context,
            problem.dissolved_reason,
        ],
    )?;
    Ok(())
}

/// Load a problem by ID
pub fn load_problem(conn: &Connection, id: &str) -> Result<Option<Problem>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, priority, parent_id, milestone_id, assignee,
                created_at, updated_at, description, context, dissolved_reason
         FROM problems WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(problem_from_row(row)?))
    } else {
        Ok(None)
    }
}

/// List all problems
pub fn list_problems(conn: &Connection) -> Result<Vec<Problem>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, priority, parent_id, milestone_id, assignee,
                created_at, updated_at, description, context, dissolved_reason
         FROM problems ORDER BY id"
    )?;

    let rows = stmt.query_map([], |row| problem_from_row(row))?;

    let mut problems = Vec::new();
    for row in rows {
        problems.push(row?);
    }
    Ok(problems)
}

/// Delete a problem
pub fn delete_problem(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM problems WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

fn problem_from_row(row: &rusqlite::Row) -> rusqlite::Result<Problem> {
    use chrono::DateTime;

    let status_str: String = row.get(2)?;
    let priority_str: String = row.get(3)?;
    let created_str: Option<String> = row.get(7)?;
    let updated_str: Option<String> = row.get(8)?;

    Ok(Problem {
        id: row.get(0)?,
        title: row.get(1)?,
        status: status_str.parse().unwrap_or(ProblemStatus::Open),
        priority: priority_str.parse().unwrap_or(Priority::Medium),
        parent_id: row.get(4)?,
        milestone_id: row.get(5)?,
        assignee: row.get(6)?,
        created_at: created_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        updated_at: updated_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        description: row.get(9)?,
        context: row.get(10)?,
        dissolved_reason: row.get(11)?,
        solution_ids: Vec::new(),  // Not stored in DB, computed from solutions table
        child_ids: Vec::new(),     // Not stored in DB, computed from parent_id
    })
}

/// Insert or update a solution
pub fn upsert_solution(conn: &Connection, solution: &Solution) -> Result<()> {
    let change_ids_json = serde_json::to_string(&solution.change_ids)?;

    conn.execute(
        "INSERT OR REPLACE INTO solutions
         (id, title, status, problem_id, change_ids, supersedes, assignee,
          force_accepted, created_at, updated_at, approach, tradeoffs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            solution.id,
            solution.title,
            solution.status.to_string(),
            solution.problem_id,
            change_ids_json,
            solution.supersedes,
            solution.assignee,
            solution.force_accepted,
            solution.created_at.map(|d| d.to_rfc3339()),
            solution.updated_at.map(|d| d.to_rfc3339()),
            solution.approach,
            solution.tradeoffs,
        ],
    )?;
    Ok(())
}

/// Load a solution by ID
pub fn load_solution(conn: &Connection, id: &str) -> Result<Option<Solution>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, problem_id, change_ids, supersedes, assignee,
                force_accepted, created_at, updated_at, approach, tradeoffs
         FROM solutions WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(solution_from_row(row)?))
    } else {
        Ok(None)
    }
}

/// List all solutions
pub fn list_solutions(conn: &Connection) -> Result<Vec<Solution>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, problem_id, change_ids, supersedes, assignee,
                force_accepted, created_at, updated_at, approach, tradeoffs
         FROM solutions ORDER BY id"
    )?;

    let rows = stmt.query_map([], |row| solution_from_row(row))?;

    let mut solutions = Vec::new();
    for row in rows {
        solutions.push(row?);
    }
    Ok(solutions)
}

/// List solutions for a problem
pub fn list_solutions_for_problem(conn: &Connection, problem_id: &str) -> Result<Vec<Solution>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, problem_id, change_ids, supersedes, assignee,
                force_accepted, created_at, updated_at, approach, tradeoffs
         FROM solutions WHERE problem_id = ?1 ORDER BY id"
    )?;

    let rows = stmt.query_map(params![problem_id], |row| solution_from_row(row))?;

    let mut solutions = Vec::new();
    for row in rows {
        solutions.push(row?);
    }
    Ok(solutions)
}

/// Delete a solution
pub fn delete_solution(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM solutions WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

fn solution_from_row(row: &rusqlite::Row) -> rusqlite::Result<Solution> {
    use chrono::DateTime;

    let status_str: String = row.get(2)?;
    let change_ids_json: String = row.get(4)?;
    let created_str: Option<String> = row.get(8)?;
    let updated_str: Option<String> = row.get(9)?;

    let change_ids: Vec<String> = serde_json::from_str(&change_ids_json).unwrap_or_default();

    Ok(Solution {
        id: row.get(0)?,
        title: row.get(1)?,
        status: status_str.parse().unwrap_or(SolutionStatus::Proposed),
        problem_id: row.get(3)?,
        change_ids,
        supersedes: row.get(5)?,
        assignee: row.get(6)?,
        force_accepted: row.get(7)?,
        created_at: created_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        updated_at: updated_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        approach: row.get(10)?,
        tradeoffs: row.get(11)?,
        critique_ids: Vec::new(),  // Computed from critiques table
    })
}

/// Insert or update a critique
pub fn upsert_critique(conn: &Connection, critique: &Critique) -> Result<()> {
    let replies_json = serde_json::to_string(&critique.replies)?;

    conn.execute(
        "INSERT OR REPLACE INTO critiques
         (id, title, status, solution_id, severity, reviewer, file_path,
          line_number, created_at, updated_at, body, replies)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            critique.id,
            critique.title,
            critique.status.to_string(),
            critique.solution_id,
            critique.severity.to_string(),
            critique.reviewer,
            critique.file,
            critique.line,
            critique.created_at.map(|d| d.to_rfc3339()),
            critique.updated_at.map(|d| d.to_rfc3339()),
            critique.body,
            replies_json,
        ],
    )?;
    Ok(())
}

/// Load a critique by ID
pub fn load_critique(conn: &Connection, id: &str) -> Result<Option<Critique>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, solution_id, severity, reviewer, file_path,
                line_number, created_at, updated_at, body, replies
         FROM critiques WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(critique_from_row(row)?))
    } else {
        Ok(None)
    }
}

/// List all critiques
pub fn list_critiques(conn: &Connection) -> Result<Vec<Critique>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, solution_id, severity, reviewer, file_path,
                line_number, created_at, updated_at, body, replies
         FROM critiques ORDER BY id"
    )?;

    let rows = stmt.query_map([], |row| critique_from_row(row))?;

    let mut critiques = Vec::new();
    for row in rows {
        critiques.push(row?);
    }
    Ok(critiques)
}

/// List critiques for a solution
pub fn list_critiques_for_solution(conn: &Connection, solution_id: &str) -> Result<Vec<Critique>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, solution_id, severity, reviewer, file_path,
                line_number, created_at, updated_at, body, replies
         FROM critiques WHERE solution_id = ?1 ORDER BY id"
    )?;

    let rows = stmt.query_map(params![solution_id], |row| critique_from_row(row))?;

    let mut critiques = Vec::new();
    for row in rows {
        critiques.push(row?);
    }
    Ok(critiques)
}

/// Delete a critique
pub fn delete_critique(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM critiques WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

fn critique_from_row(row: &rusqlite::Row) -> rusqlite::Result<Critique> {
    use chrono::DateTime;
    use crate::models::CritiqueReply;

    let status_str: String = row.get(2)?;
    let severity_str: String = row.get(4)?;
    let created_str: Option<String> = row.get(8)?;
    let updated_str: Option<String> = row.get(9)?;
    let replies_json: String = row.get(11)?;

    let replies: Vec<CritiqueReply> = serde_json::from_str(&replies_json).unwrap_or_default();

    Ok(Critique {
        id: row.get(0)?,
        title: row.get(1)?,
        status: status_str.parse().unwrap_or(CritiqueStatus::Open),
        solution_id: row.get(3)?,
        severity: severity_str.parse().unwrap_or(CritiqueSeverity::Medium),
        reviewer: row.get(5)?,
        file: row.get(6)?,
        line: row.get(7)?,
        created_at: created_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        updated_at: updated_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        body: row.get(10)?,
        replies,
    })
}

/// Insert or update a milestone
pub fn upsert_milestone(conn: &Connection, milestone: &Milestone) -> Result<()> {
    let problem_ids_json = serde_json::to_string(&milestone.problem_ids)?;

    conn.execute(
        "INSERT OR REPLACE INTO milestones
         (id, title, status, target_date, assignee, created_at, updated_at,
          description, problem_ids)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            milestone.id,
            milestone.title,
            milestone.status.to_string(),
            milestone.target_date.map(|d| d.format("%Y-%m-%d").to_string()),
            milestone.assignee,
            milestone.created_at.map(|d| d.to_rfc3339()),
            milestone.updated_at.map(|d| d.to_rfc3339()),
            milestone.description,
            problem_ids_json,
        ],
    )?;
    Ok(())
}

/// Load a milestone by ID
pub fn load_milestone(conn: &Connection, id: &str) -> Result<Option<Milestone>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, target_date, assignee, created_at, updated_at,
                description, problem_ids
         FROM milestones WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(milestone_from_row(row)?))
    } else {
        Ok(None)
    }
}

/// List all milestones
pub fn list_milestones(conn: &Connection) -> Result<Vec<Milestone>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, target_date, assignee, created_at, updated_at,
                description, problem_ids
         FROM milestones ORDER BY id"
    )?;

    let rows = stmt.query_map([], |row| milestone_from_row(row))?;

    let mut milestones = Vec::new();
    for row in rows {
        milestones.push(row?);
    }
    Ok(milestones)
}

/// Delete a milestone
pub fn delete_milestone(conn: &Connection, id: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM milestones WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

fn milestone_from_row(row: &rusqlite::Row) -> rusqlite::Result<Milestone> {
    use chrono::{DateTime, NaiveDate};

    let status_str: String = row.get(2)?;
    let target_str: Option<String> = row.get(3)?;
    let created_str: Option<String> = row.get(5)?;
    let updated_str: Option<String> = row.get(6)?;
    let problem_ids_json: String = row.get(8)?;

    let problem_ids: Vec<String> = serde_json::from_str(&problem_ids_json).unwrap_or_default();

    Ok(Milestone {
        id: row.get(0)?,
        title: row.get(1)?,
        status: status_str.parse().unwrap_or(MilestoneStatus::Planning),
        target_date: target_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        assignee: row.get(4)?,
        created_at: created_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        updated_at: updated_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))),
        description: row.get(7)?,
        problem_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_problem_crud() {
        let db = Database::open_in_memory().unwrap();

        let problem = Problem {
            id: "p1".to_string(),
            title: "Test problem".to_string(),
            status: ProblemStatus::Open,
            priority: Priority::High,
            parent_id: None,
            milestone_id: None,
            assignee: Some("alice".to_string()),
            created_at: None,
            updated_at: None,
            description: "A test".to_string(),
            context: "".to_string(),
            dissolved_reason: None,
            solution_ids: vec![],
            child_ids: vec![],
        };

        upsert_problem(db.conn(), &problem).unwrap();

        let loaded = load_problem(db.conn(), "p1").unwrap().unwrap();
        assert_eq!(loaded.id, "p1");
        assert_eq!(loaded.title, "Test problem");
        assert_eq!(loaded.assignee, Some("alice".to_string()));

        let all = list_problems(db.conn()).unwrap();
        assert_eq!(all.len(), 1);

        delete_problem(db.conn(), "p1").unwrap();
        assert!(load_problem(db.conn(), "p1").unwrap().is_none());
    }

    #[test]
    fn test_solution_crud() {
        let db = Database::open_in_memory().unwrap();

        let solution = Solution {
            id: "s1".to_string(),
            title: "Test solution".to_string(),
            status: SolutionStatus::Proposed,
            problem_id: "p1".to_string(),
            change_ids: vec!["abc123".to_string()],
            supersedes: None,
            assignee: None,
            force_accepted: false,
            created_at: None,
            updated_at: None,
            approach: "Do the thing".to_string(),
            tradeoffs: "".to_string(),
            critique_ids: vec![],
        };

        upsert_solution(db.conn(), &solution).unwrap();

        let loaded = load_solution(db.conn(), "s1").unwrap().unwrap();
        assert_eq!(loaded.change_ids, vec!["abc123".to_string()]);

        let for_problem = list_solutions_for_problem(db.conn(), "p1").unwrap();
        assert_eq!(for_problem.len(), 1);
    }

    #[test]
    fn test_critique_crud() {
        let db = Database::open_in_memory().unwrap();

        let critique = Critique::new(
            "c1".to_string(),
            "s1".to_string(),
            "Test critique".to_string(),
            CritiqueSeverity::High,
        );

        upsert_critique(db.conn(), &critique).unwrap();

        let loaded = load_critique(db.conn(), "c1").unwrap().unwrap();
        assert_eq!(loaded.title, "Test critique");
        assert_eq!(loaded.severity, CritiqueSeverity::High);
    }

    #[test]
    fn test_milestone_crud() {
        let db = Database::open_in_memory().unwrap();

        let milestone = Milestone::new("m1".to_string(), "Q1 Goals".to_string());

        upsert_milestone(db.conn(), &milestone).unwrap();

        let loaded = load_milestone(db.conn(), "m1").unwrap().unwrap();
        assert_eq!(loaded.title, "Q1 Goals");
    }
}
```

**Step 2: Update db/mod.rs**

```rust
pub mod entities;
pub mod schema;

pub use schema::{Database, SCHEMA_VERSION};
```

**Step 3: Run tests**

Run: `cargo test db::entities`
Expected: 4 tests pass

**Step 4: Commit**

```bash
git add src/db/entities.rs src/db/mod.rs
git commit -m "feat(db): add CRUD operations for all entities"
```

---

## Task 4: Implement Events Table Operations

**Files:**
- Create: `src/db/events.rs`
- Modify: `src/db/mod.rs`

**Step 1: Create events module**

Create `src/db/events.rs`:

```rust
use crate::error::Result;
use crate::models::{Event, EventExtra, EventType};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

/// Insert an event
pub fn insert_event(conn: &Connection, event: &Event) -> Result<i64> {
    let refs_json = serde_json::to_string(&event.refs)?;
    let extra_json = serde_json::to_string(&event.extra)?;

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

/// List events with optional filters
pub fn list_events(
    conn: &Connection,
    entity_id: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
) -> Result<Vec<Event>> {
    let mut sql = String::from(
        "SELECT timestamp, event_type, entity_id, actor, rationale, refs, extra
         FROM events WHERE 1=1"
    );

    if entity_id.is_some() {
        sql.push_str(" AND entity_id = ?1");
    }
    if event_type.is_some() {
        sql.push_str(" AND event_type = ?2");
    }
    sql.push_str(" ORDER BY timestamp DESC LIMIT ?3");

    let mut stmt = conn.prepare(&sql)?;

    let rows = match (entity_id, event_type) {
        (Some(e), Some(t)) => stmt.query_map(params![e, t, limit], event_from_row)?,
        (Some(e), None) => stmt.query_map(params![e, "", limit], event_from_row)?,
        (None, Some(t)) => stmt.query_map(params!["", t, limit], event_from_row)?,
        (None, None) => stmt.query_map(params!["", "", limit], event_from_row)?,
    };

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

/// List events for an entity and all related entities (for timeline)
pub fn list_events_for_timeline(conn: &Connection, entity_id: &str) -> Result<Vec<Event>> {
    // Get events where entity matches OR is in refs
    let mut stmt = conn.prepare(
        "SELECT timestamp, event_type, entity_id, actor, rationale, refs, extra
         FROM events
         WHERE entity_id = ?1 OR refs LIKE ?2
         ORDER BY timestamp ASC"
    )?;

    let pattern = format!("%\"{}%", entity_id);
    let rows = stmt.query_map(params![entity_id, pattern], event_from_row)?;

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

/// Clear all events (for rebuild)
pub fn clear_events(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM events", [])?;
    Ok(())
}

fn event_from_row(row: &rusqlite::Row) -> rusqlite::Result<Event> {
    let timestamp_str: String = row.get(0)?;
    let type_str: String = row.get(1)?;
    let refs_json: String = row.get(5)?;
    let extra_json: String = row.get(6)?;

    let when = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let event_type = match type_str.as_str() {
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
        _ => EventType::ProblemCreated, // fallback
    };

    let refs: Vec<String> = serde_json::from_str(&refs_json).unwrap_or_default();
    let extra: EventExtra = serde_json::from_str(&extra_json).unwrap_or_default();

    Ok(Event {
        when,
        event_type,
        entity: row.get(2)?,
        by: row.get(3)?,
        rationale: row.get(4)?,
        refs,
        extra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_insert_and_list_events() {
        let db = Database::open_in_memory().unwrap();

        let event = Event::new(
            EventType::ProblemCreated,
            "p1".to_string(),
            "alice".to_string(),
        );

        insert_event(db.conn(), &event).unwrap();

        let events = list_events(db.conn(), None, None, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, "p1");
    }

    #[test]
    fn test_timeline_events() {
        let db = Database::open_in_memory().unwrap();

        // Problem created
        let e1 = Event::new(EventType::ProblemCreated, "p1".to_string(), "alice".to_string());
        insert_event(db.conn(), &e1).unwrap();

        // Solution created referencing problem
        let e2 = Event::new(EventType::SolutionCreated, "s1".to_string(), "alice".to_string())
            .with_refs(vec!["p1".to_string()]);
        insert_event(db.conn(), &e2).unwrap();

        let timeline = list_events_for_timeline(db.conn(), "p1").unwrap();
        assert_eq!(timeline.len(), 2);
    }
}
```

**Step 2: Update db/mod.rs**

```rust
pub mod entities;
pub mod events;
pub mod schema;

pub use schema::{Database, SCHEMA_VERSION};
```

**Step 3: Run tests**

Run: `cargo test db::events`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add src/db/events.rs src/db/mod.rs
git commit -m "feat(db): add event storage operations"
```

---

## Task 5: Implement Markdown Load/Dump

**Files:**
- Create: `src/db/sync.rs`
- Modify: `src/db/mod.rs`

**Step 1: Create sync module**

Create `src/db/sync.rs`:

```rust
//! Sync between SQLite and markdown files

use crate::db::{entities, events, Database};
use crate::error::Result;
use crate::models::Event;
use crate::storage::MetadataStore;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Load all markdown files into SQLite database
pub fn load_from_markdown(db: &Database, store: &MetadataStore) -> Result<()> {
    let conn = db.conn();

    // Clear existing data
    conn.execute("DELETE FROM problems", [])?;
    conn.execute("DELETE FROM solutions", [])?;
    conn.execute("DELETE FROM critiques", [])?;
    conn.execute("DELETE FROM milestones", [])?;
    events::clear_events(conn)?;

    // Load problems
    for problem in store.list_problems()? {
        entities::upsert_problem(conn, &problem)?;
    }

    // Load solutions
    for solution in store.list_solutions()? {
        entities::upsert_solution(conn, &solution)?;
    }

    // Load critiques
    for critique in store.list_critiques()? {
        entities::upsert_critique(conn, &critique)?;
    }

    // Load milestones
    for milestone in store.list_milestones()? {
        entities::upsert_milestone(conn, &milestone)?;
    }

    // Load events from events.jsonl
    load_events_from_jsonl(db, store.meta_path())?;

    // Rebuild FTS index
    rebuild_fts(db)?;

    // Mark as clean
    set_dirty(db, false)?;

    Ok(())
}

/// Load events from events.jsonl file
fn load_events_from_jsonl(db: &Database, meta_path: &Path) -> Result<()> {
    let events_path = meta_path.join("events.jsonl");
    if !events_path.exists() {
        return Ok(());
    }

    let file = fs::File::open(events_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<Event>(&line) {
            events::insert_event(db.conn(), &event)?;
        }
    }

    Ok(())
}

/// Dump SQLite database to markdown files
pub fn dump_to_markdown(db: &Database, store: &MetadataStore) -> Result<()> {
    let conn = db.conn();

    // Dump problems
    for problem in entities::list_problems(conn)? {
        store.save_problem(&problem)?;
    }

    // Dump solutions
    for solution in entities::list_solutions(conn)? {
        store.save_solution(&solution)?;
    }

    // Dump critiques
    for critique in entities::list_critiques(conn)? {
        store.save_critique(&critique)?;
    }

    // Dump milestones
    for milestone in entities::list_milestones(conn)? {
        store.save_milestone(&milestone)?;
    }

    // Dump events to events.jsonl
    dump_events_to_jsonl(db, store.meta_path())?;

    Ok(())
}

/// Dump events to events.jsonl file
fn dump_events_to_jsonl(db: &Database, meta_path: &Path) -> Result<()> {
    let events = events::list_events(db.conn(), None, None, usize::MAX)?;

    // Sort by timestamp ascending for the file
    let mut events = events;
    events.sort_by(|a, b| a.when.cmp(&b.when));

    let events_path = meta_path.join("events.jsonl");
    let mut content = String::new();

    for event in events {
        content.push_str(&event.to_json_line()?);
        content.push('\n');
    }

    fs::write(events_path, content)?;

    Ok(())
}

/// Rebuild FTS index from current data
fn rebuild_fts(db: &Database) -> Result<()> {
    let conn = db.conn();

    // Clear FTS
    conn.execute("DELETE FROM fts", [])?;

    // Index problems
    let problems = entities::list_problems(conn)?;
    for p in problems {
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["problem", p.id, p.title, format!("{}\n{}", p.description, p.context)],
        )?;
    }

    // Index solutions
    let solutions = entities::list_solutions(conn)?;
    for s in solutions {
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["solution", s.id, s.title, format!("{}\n{}", s.approach, s.tradeoffs)],
        )?;
    }

    // Index critiques
    let critiques = entities::list_critiques(conn)?;
    for c in critiques {
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["critique", c.id, c.title, c.body],
        )?;
    }

    // Index milestones
    let milestones = entities::list_milestones(conn)?;
    for m in milestones {
        conn.execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            rusqlite::params!["milestone", m.id, m.title, m.description],
        )?;
    }

    Ok(())
}

/// Check if database has uncommitted changes
pub fn is_dirty(db: &Database) -> Result<bool> {
    let dirty: String = db.conn().query_row(
        "SELECT value FROM meta WHERE key = 'dirty'",
        [],
        |row| row.get(0),
    )?;
    Ok(dirty == "true")
}

/// Set dirty flag
pub fn set_dirty(db: &Database, dirty: bool) -> Result<()> {
    db.conn().execute(
        "UPDATE meta SET value = ?1 WHERE key = 'dirty'",
        rusqlite::params![if dirty { "true" } else { "false" }],
    )?;
    Ok(())
}
```

**Step 2: Update db/mod.rs**

```rust
pub mod entities;
pub mod events;
pub mod schema;
pub mod sync;

pub use schema::{Database, SCHEMA_VERSION};
pub use sync::{dump_to_markdown, is_dirty, load_from_markdown, set_dirty};
```

**Step 3: Add meta_path getter to MetadataStore**

In `src/storage.rs`, add this method to `impl MetadataStore`:

```rust
/// Get the metadata directory path
pub fn meta_path(&self) -> &Path {
    &self.meta_path
}
```

**Step 4: Run build**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/db/sync.rs src/db/mod.rs src/storage.rs
git commit -m "feat(db): add markdown sync (load/dump)"
```

---

## Task 6: Implement Full-Text Search

**Files:**
- Create: `src/db/search.rs`
- Modify: `src/db/mod.rs`

**Step 1: Create search module**

Create `src/db/search.rs`:

```rust
//! Full-text search implementation

use crate::error::Result;
use rusqlite::{params, Connection};

/// A search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub snippet: String,
}

/// Search all entities
pub fn search(conn: &Connection, query: &str, entity_type: Option<&str>) -> Result<Vec<SearchResult>> {
    let sql = if entity_type.is_some() {
        "SELECT entity_type, entity_id, title, snippet(fts, 3, '<b>', '</b>', '...', 32)
         FROM fts
         WHERE fts MATCH ?1 AND entity_type = ?2
         ORDER BY rank
         LIMIT 50"
    } else {
        "SELECT entity_type, entity_id, title, snippet(fts, 3, '<b>', '</b>', '...', 32)
         FROM fts
         WHERE fts MATCH ?1
         ORDER BY rank
         LIMIT 50"
    };

    let mut stmt = conn.prepare(sql)?;

    let rows = if let Some(et) = entity_type {
        stmt.query_map(params![query, et], |row| {
            Ok(SearchResult {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                title: row.get(2)?,
                snippet: row.get(3)?,
            })
        })?
    } else {
        stmt.query_map(params![query], |row| {
            Ok(SearchResult {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                title: row.get(2)?,
                snippet: row.get(3)?,
            })
        })?
    };

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Search events by rationale
pub fn search_events(conn: &Connection, query: &str) -> Result<Vec<crate::models::Event>> {
    use crate::db::events;

    // Simple LIKE search on rationale for now
    let mut stmt = conn.prepare(
        "SELECT timestamp, event_type, entity_id, actor, rationale, refs, extra
         FROM events
         WHERE rationale LIKE ?1
         ORDER BY timestamp DESC
         LIMIT 50"
    )?;

    let pattern = format!("%{}%", query);
    let rows = stmt.query_map(params![pattern], |row| {
        use chrono::{DateTime, Utc};
        use crate::models::{EventExtra, EventType};

        let timestamp_str: String = row.get(0)?;
        let type_str: String = row.get(1)?;
        let refs_json: String = row.get(5)?;
        let extra_json: String = row.get(6)?;

        let when = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let event_type = match type_str.as_str() {
            "problem_created" => EventType::ProblemCreated,
            "solution_accepted" => EventType::SolutionAccepted,
            "solution_refuted" => EventType::SolutionRefuted,
            "critique_raised" => EventType::CritiqueRaised,
            _ => EventType::ProblemCreated,
        };

        let refs: Vec<String> = serde_json::from_str(&refs_json).unwrap_or_default();
        let extra: EventExtra = serde_json::from_str(&extra_json).unwrap_or_default();

        Ok(crate::models::Event {
            when,
            event_type,
            entity: row.get(2)?,
            by: row.get(3)?,
            rationale: row.get(4)?,
            refs,
            extra,
        })
    })?;

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_fts_search() {
        let db = Database::open_in_memory().unwrap();

        // Insert test data
        db.conn().execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            params!["problem", "p1", "Authentication bug", "Users cannot log in with special characters"],
        ).unwrap();

        db.conn().execute(
            "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
            params!["solution", "s1", "Fix login", "Handle special chars in password"],
        ).unwrap();

        // Search
        let results = search(db.conn(), "login", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "s1");

        // Search with type filter
        let results = search(db.conn(), "special", Some("problem")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "p1");
    }
}
```

**Step 2: Update db/mod.rs**

```rust
pub mod entities;
pub mod events;
pub mod schema;
pub mod search;
pub mod sync;

pub use schema::{Database, SCHEMA_VERSION};
pub use search::{search, SearchResult};
pub use sync::{dump_to_markdown, is_dirty, load_from_markdown, set_dirty};
```

**Step 3: Run tests**

Run: `cargo test db::search`
Expected: 1 test passes

**Step 4: Commit**

```bash
git add src/db/search.rs src/db/mod.rs
git commit -m "feat(db): add full-text search"
```

---

## Task 7: Implement Validation

**Files:**
- Create: `src/db/validate.rs`
- Modify: `src/db/mod.rs`

**Step 1: Create validation module**

Create `src/db/validate.rs`:

```rust
//! Validation before push

use crate::db::{entities, Database};
use crate::error::Result;
use std::collections::HashSet;

/// A validation error
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

/// Validate all data before push
pub fn validate(db: &Database) -> Result<Vec<ValidationError>> {
    let conn = db.conn();
    let mut errors = Vec::new();

    // Collect all valid IDs
    let problem_ids: HashSet<String> = entities::list_problems(conn)?
        .into_iter()
        .map(|p| p.id)
        .collect();

    let solution_ids: HashSet<String> = entities::list_solutions(conn)?
        .into_iter()
        .map(|s| s.id)
        .collect();

    let milestone_ids: HashSet<String> = entities::list_milestones(conn)?
        .into_iter()
        .map(|m| m.id)
        .collect();

    // Validate problems
    for problem in entities::list_problems(conn)? {
        // Check parent exists
        if let Some(ref parent_id) = problem.parent_id {
            if !problem_ids.contains(parent_id) {
                errors.push(ValidationError {
                    entity_id: problem.id.clone(),
                    message: format!("references non-existent parent {}", parent_id),
                });
            }
        }

        // Check milestone exists
        if let Some(ref milestone_id) = problem.milestone_id {
            if !milestone_ids.contains(milestone_id) {
                errors.push(ValidationError {
                    entity_id: problem.id.clone(),
                    message: format!("references non-existent milestone {}", milestone_id),
                });
            }
        }

        // Check for cycles in parent chain
        if has_parent_cycle(conn, &problem.id, &problem_ids)? {
            errors.push(ValidationError {
                entity_id: problem.id.clone(),
                message: "creates a cycle in parent chain".to_string(),
            });
        }
    }

    // Validate solutions
    for solution in entities::list_solutions(conn)? {
        // Check problem exists
        if !problem_ids.contains(&solution.problem_id) {
            errors.push(ValidationError {
                entity_id: solution.id.clone(),
                message: format!("references non-existent problem {}", solution.problem_id),
            });
        }

        // Check supersedes exists
        if let Some(ref supersedes) = solution.supersedes {
            if !solution_ids.contains(supersedes) {
                errors.push(ValidationError {
                    entity_id: solution.id.clone(),
                    message: format!("supersedes non-existent solution {}", supersedes),
                });
            }
        }
    }

    // Validate critiques
    for critique in entities::list_critiques(conn)? {
        // Check solution exists
        if !solution_ids.contains(&critique.solution_id) {
            errors.push(ValidationError {
                entity_id: critique.id.clone(),
                message: format!("references non-existent solution {}", critique.solution_id),
            });
        }
    }

    Ok(errors)
}

/// Check if a problem has a cycle in its parent chain
fn has_parent_cycle(
    conn: &rusqlite::Connection,
    start_id: &str,
    valid_ids: &HashSet<String>,
) -> Result<bool> {
    let mut visited = HashSet::new();
    let mut current_id = Some(start_id.to_string());

    while let Some(id) = current_id {
        if visited.contains(&id) {
            return Ok(true);
        }
        visited.insert(id.clone());

        if let Some(problem) = entities::load_problem(conn, &id)? {
            current_id = problem.parent_id;
        } else {
            break;
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{Problem, ProblemStatus, Priority, Solution, SolutionStatus};

    #[test]
    fn test_validation_catches_missing_refs() {
        let db = Database::open_in_memory().unwrap();

        // Create solution referencing non-existent problem
        let solution = Solution {
            id: "s1".to_string(),
            title: "Test".to_string(),
            status: SolutionStatus::Proposed,
            problem_id: "p99".to_string(),  // doesn't exist
            change_ids: vec![],
            supersedes: None,
            assignee: None,
            force_accepted: false,
            created_at: None,
            updated_at: None,
            approach: "".to_string(),
            tradeoffs: "".to_string(),
            critique_ids: vec![],
        };
        entities::upsert_solution(db.conn(), &solution).unwrap();

        let errors = validate(&db).unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("p99"));
    }

    #[test]
    fn test_validation_passes_for_valid_data() {
        let db = Database::open_in_memory().unwrap();

        // Create valid problem and solution
        let problem = Problem {
            id: "p1".to_string(),
            title: "Test".to_string(),
            status: ProblemStatus::Open,
            priority: Priority::Medium,
            parent_id: None,
            milestone_id: None,
            assignee: None,
            created_at: None,
            updated_at: None,
            description: "".to_string(),
            context: "".to_string(),
            dissolved_reason: None,
            solution_ids: vec![],
            child_ids: vec![],
        };
        entities::upsert_problem(db.conn(), &problem).unwrap();

        let solution = Solution {
            id: "s1".to_string(),
            title: "Test".to_string(),
            status: SolutionStatus::Proposed,
            problem_id: "p1".to_string(),
            change_ids: vec![],
            supersedes: None,
            assignee: None,
            force_accepted: false,
            created_at: None,
            updated_at: None,
            approach: "".to_string(),
            tradeoffs: "".to_string(),
            critique_ids: vec![],
        };
        entities::upsert_solution(db.conn(), &solution).unwrap();

        let errors = validate(&db).unwrap();
        assert!(errors.is_empty());
    }
}
```

**Step 2: Update db/mod.rs**

```rust
pub mod entities;
pub mod events;
pub mod schema;
pub mod search;
pub mod sync;
pub mod validate;

pub use schema::{Database, SCHEMA_VERSION};
pub use search::{search, SearchResult};
pub use sync::{dump_to_markdown, is_dirty, load_from_markdown, set_dirty};
pub use validate::{validate, ValidationError};
```

**Step 3: Run tests**

Run: `cargo test db::validate`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add src/db/validate.rs src/db/mod.rs
git commit -m "feat(db): add validation before push"
```

---

## Task 8: Update Fetch Command

**Files:**
- Modify: `src/commands/fetch.rs`

**Step 1: Update fetch to rebuild DB**

Replace `src/commands/fetch.rs`:

```rust
use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;
use std::path::PathBuf;

pub fn execute(ctx: &CommandContext, remote: &str) -> Result<()> {
    let jj_client = ctx.jj();
    let db_path = get_db_path(jj_client)?;

    // 1. If DB exists and is dirty, dump to markdown first
    if db_path.exists() {
        if let Ok(db) = Database::open(&db_path) {
            if db::is_dirty(&db)? {
                println!("Saving local changes before fetch...");
                db::dump_to_markdown(&db, &ctx.store)?;
                ctx.store.commit_changes("Sync local changes before fetch")?;
            }
        }
    }

    // Snapshot counts before fetch
    let solutions_before = ctx.store.list_solutions().unwrap_or_default().len();
    let critiques_before = ctx.store.list_critiques().unwrap_or_default().len();

    // 2. Fetch from remote
    println!("Fetching from {}...", remote);
    jj_client.execute(&["git", "fetch", "--remote", remote])?;

    // 3. Update jjj-meta workspace if it exists
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    if meta_path.exists() {
        if let Ok(meta_client) = JjClient::with_root(meta_path) {
            if let Err(e) = meta_client.execute(&["new", "jjj/meta@origin"]) {
                eprintln!("Warning: could not update jjj-meta workspace: {}", e);
            }
        }
    }

    // 4. Rebuild SQLite from markdown
    println!("Rebuilding database...");

    // Delete old DB and create fresh
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let db = Database::open(&db_path)?;

    // Need fresh store to see updated files
    let store_after = MetadataStore::new(jj_client.clone())?;
    db::load_from_markdown(&db, &store_after)?;

    // 5. Show summary
    let solutions_after = store_after.list_solutions().unwrap_or_default().len();
    let critiques_after = store_after.list_critiques().unwrap_or_default().len();

    let new_solutions = solutions_after.saturating_sub(solutions_before);
    let new_critiques = critiques_after.saturating_sub(critiques_before);

    println!("Fetched from {}.", remote);
    if new_solutions > 0 {
        println!("  {} new solution(s)", new_solutions);
    }
    if new_critiques > 0 {
        println!("  {} new critique(s)", new_critiques);
    }
    if new_solutions == 0 && new_critiques == 0 {
        println!("  No new jjj changes.");
    }

    Ok(())
}

fn get_db_path(jj_client: &JjClient) -> Result<PathBuf> {
    Ok(jj_client.repo_root().join(".jj").join("jjj.db"))
}
```

**Step 2: Run build**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/commands/fetch.rs
git commit -m "feat(fetch): rebuild SQLite from markdown after fetch"
```

---

## Task 9: Update Push Command

**Files:**
- Modify: `src/commands/push.rs`

**Step 1: Update push to dump and validate**

Replace the beginning of `src/commands/push.rs` (keep the helper functions):

```rust
use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::Result;
use crate::models::{CritiqueStatus, ProblemStatus};
use crate::storage::MetadataStore;
use std::io::{self, Write};

fn prompt_yes_no(message: &str) -> bool {
    print!("{} [Y/n] ", message);
    if io::stdout().flush().is_err() {
        return false;
    }

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}

pub fn execute(
    ctx: &CommandContext,
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");

    // 1. Dump SQLite to markdown (if DB exists)
    if db_path.exists() {
        if let Ok(db) = Database::open(&db_path) {
            println!("Syncing database to files...");
            db::dump_to_markdown(&db, store)?;

            // 2. Validate
            println!("Validating metadata...");
            let errors = db::validate(&db)?;

            if !errors.is_empty() {
                eprintln!("Validation errors:");
                for err in &errors {
                    eprintln!("  ✗ {}", err);
                }
                return Err("Push aborted. Fix errors and retry.".into());
            }
            println!("  ✓ All checks passed");
        }
    }

    // 3. Commit any changes
    store.commit_changes("Sync metadata for push")?;

    if dry_run {
        println!("Would push to {}:", remote);
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj/meta");
        return Ok(());
    }

    // 4. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        let result = jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote]);
        if result.is_err() {
            jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote, "--allow-new"])?;
        }
    }

    // 5. Always push jjj/meta
    println!("Pushing jjj/meta...");
    let result = jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote]);
    if result.is_err() {
        jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote, "--allow-new"])?;
    }

    // 6. Clear dirty flag
    if db_path.exists() {
        if let Ok(db) = Database::open(&db_path) {
            db::set_dirty(&db, false)?;
        }
    }

    println!("Pushed to {}.", remote);

    // 7. Smart prompts (unless --no-prompt)
    if !no_prompt {
        check_and_prompt_accept_solve(&store)?;
    }

    Ok(())
}

// Keep the existing check_and_prompt_accept_solve function unchanged
```

**Step 2: Run build**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/commands/push.rs
git commit -m "feat(push): dump, validate, and sync before push"
```

---

## Task 10: Add Search Command

**Files:**
- Modify: `src/cli.rs`
- Create: `src/commands/search.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add Search command to CLI**

In `src/cli.rs`, add to the `Commands` enum after `Status`:

```rust
    /// Search across all entities
    Search {
        /// Search query
        query: String,

        /// Filter by entity type (problem, solution, critique, milestone, event)
        #[arg(long, short = 't')]
        r#type: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
```

**Step 2: Create search command handler**

Create `src/commands/search.rs`:

```rust
use crate::context::CommandContext;
use crate::db::{self, search, Database};
use crate::error::Result;

pub fn execute(
    ctx: &CommandContext,
    query: &str,
    entity_type: Option<&str>,
    json: bool,
) -> Result<()> {
    let jj_client = ctx.jj();
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");

    // Ensure DB exists
    let db = if db_path.exists() {
        Database::open(&db_path)?
    } else {
        // Create and populate from markdown
        let db = Database::open(&db_path)?;
        db::load_from_markdown(&db, &ctx.store)?;
        db
    };

    let results = search::search(db.conn(), query, entity_type)?;

    if json {
        let json_results: Vec<_> = results.iter().map(|r| {
            serde_json::json!({
                "type": r.entity_type,
                "id": r.entity_id,
                "title": r.title,
                "snippet": r.snippet,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        if results.is_empty() {
            println!("No results found for \"{}\"", query);
        } else {
            println!("Found {} result(s) for \"{}\":\n", results.len(), query);
            for result in results {
                println!("[{}] {} - {}", result.entity_type, result.entity_id, result.title);
                if !result.snippet.is_empty() {
                    println!("    {}", result.snippet.replace('\n', " "));
                }
                println!();
            }
        }
    }

    Ok(())
}
```

**Step 3: Add to commands/mod.rs dispatch**

In `src/commands/mod.rs`, add:

```rust
pub mod search;
```

And in the `execute` function, add the match arm:

```rust
        Commands::Search { query, r#type, json } => {
            search::execute(&ctx, &query, r#type.as_deref(), *json)
        }
```

**Step 4: Run build**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/search.rs src/commands/mod.rs
git commit -m "feat: add jjj search command"
```

---

## Task 11: Add --search Flag to List Commands

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/problem.rs`
- Modify: `src/commands/solution.rs`
- Modify: `src/commands/critique.rs`

**Step 1: Add --search to ProblemAction::List**

In `src/cli.rs`, update `ProblemAction::List`:

```rust
    /// List all problems
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Show as tree (hierarchical view)
        #[arg(long)]
        tree: bool,

        /// Filter by milestone
        #[arg(long)]
        milestone: Option<String>,

        /// Search problems
        #[arg(long)]
        search: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
```

**Step 2: Add --search to SolutionAction::List**

```rust
    /// List all solutions
    List {
        /// Filter by problem
        #[arg(long)]
        problem: Option<String>,

        /// Filter by status (proposed, testing, refuted, accepted)
        #[arg(long)]
        status: Option<String>,

        /// Search solutions
        #[arg(long)]
        search: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
```

**Step 3: Add --search to CritiqueAction::List**

```rust
    /// List critiques
    List {
        /// Filter by solution
        #[arg(long)]
        solution: Option<String>,

        /// Filter by status (open, addressed, valid, dismissed)
        #[arg(long)]
        status: Option<String>,

        /// Filter by assigned reviewer
        #[arg(long)]
        reviewer: Option<String>,

        /// Search critiques
        #[arg(long)]
        search: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
```

**Step 4: Update command handlers** (simplified - just update function signatures)

The command handlers need to accept and use the search parameter. This involves updating the match arms in `commands/mod.rs` and the list functions in each command file to filter results using FTS when `search` is provided.

**Step 5: Run build**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/cli.rs src/commands/problem.rs src/commands/solution.rs src/commands/critique.rs src/commands/mod.rs
git commit -m "feat: add --search flag to list commands"
```

---

## Task 12: Integration Test

**Files:**
- Create: `tests/sqlite_storage_test.rs`

**Step 1: Create integration test**

Create `tests/sqlite_storage_test.rs`:

```rust
//! Integration tests for SQLite storage layer

use jjj::db::{self, Database};
use jjj::models::{Problem, ProblemStatus, Priority, Solution, SolutionStatus};
use tempfile::TempDir;

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
        created_at: None,
        updated_at: None,
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
        created_at: None,
        updated_at: None,
        approach: "Do the thing".to_string(),
        tradeoffs: "".to_string(),
        critique_ids: vec![],
    };
    db::entities::upsert_solution(db.conn(), &solution).unwrap();

    // Validate - should pass
    let errors = db::validate(&db).unwrap();
    assert!(errors.is_empty(), "Expected no validation errors");

    // Search
    db.conn().execute(
        "INSERT INTO fts (entity_type, entity_id, title, body) VALUES (?, ?, ?, ?)",
        rusqlite::params!["problem", "p1", "Test problem", "Test description"],
    ).unwrap();

    let results = db::search(db.conn(), "test", None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, "p1");
}

#[test]
fn test_validation_catches_invalid_refs() {
    let db = Database::open_in_memory().unwrap();

    // Create solution with invalid problem reference
    let solution = Solution {
        id: "s1".to_string(),
        title: "Bad solution".to_string(),
        status: SolutionStatus::Proposed,
        problem_id: "p_invalid".to_string(),
        change_ids: vec![],
        supersedes: None,
        assignee: None,
        force_accepted: false,
        created_at: None,
        updated_at: None,
        approach: "".to_string(),
        tradeoffs: "".to_string(),
        critique_ids: vec![],
    };
    db::entities::upsert_solution(db.conn(), &solution).unwrap();

    let errors = db::validate(&db).unwrap();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("p_invalid"));
}

#[test]
fn test_dirty_flag() {
    let db = Database::open_in_memory().unwrap();

    // Initially clean
    assert!(!db::is_dirty(&db).unwrap());

    // Set dirty
    db::set_dirty(&db, true).unwrap();
    assert!(db::is_dirty(&db).unwrap());

    // Clear
    db::set_dirty(&db, false).unwrap();
    assert!(!db::is_dirty(&db).unwrap());
}
```

**Step 2: Run tests**

Run: `cargo test sqlite_storage`
Expected: 3 tests pass

**Step 3: Commit**

```bash
git add tests/sqlite_storage_test.rs
git commit -m "test: add SQLite storage integration tests"
```

---

## Summary

After completing all tasks, you will have:

1. **SQLite database layer** at `.jj/jjj.db` with schema versioning
2. **CRUD operations** for problems, solutions, critiques, milestones, events
3. **Full-text search** via FTS5
4. **Sync operations**: load from markdown, dump to markdown
5. **Validation** before push with clear error messages
6. **Updated fetch** that rebuilds DB from markdown
7. **Updated push** that dumps, validates, then pushes
8. **New `jjj search` command** for global search
9. **`--search` flag** on list commands

The markdown files remain the sync format, and SQLite is purely a local performance optimization that can be rebuilt at any time.
