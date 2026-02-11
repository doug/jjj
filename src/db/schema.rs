//! Database schema and connection management for jjj SQLite storage.
//!
//! The database is stored at `.jj/jjj.db` and serves as a runtime cache
//! for fast queries and full-text search. The canonical data remains in
//! the shadow graph (markdown files in jjj/meta bookmark).

use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

/// Current schema version. Increment when schema changes require rebuild.
pub const SCHEMA_VERSION: u32 = 2;

/// Database wrapper providing connection management and schema initialization.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path.
    ///
    /// This will:
    /// 1. Create the database file if it doesn't exist
    /// 2. Check schema version and rebuild if needed
    /// 3. Ensure all tables and indexes exist
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let mut db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    /// Create an in-memory database for testing.
    pub fn open_in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.ensure_schema()?;
        Ok(db)
    }

    /// Get a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Check if the database needs a full rebuild.
    ///
    /// Returns true if:
    /// - Schema version is missing (new database)
    /// - Schema version doesn't match current version
    /// - Dirty flag is set (interrupted sync)
    pub fn needs_rebuild(&self) -> bool {
        match self.get_schema_version() {
            Ok(version) => version != SCHEMA_VERSION || self.is_dirty(),
            Err(_) => true,
        }
    }

    /// Get the current schema version from the database.
    fn get_schema_version(&self) -> SqliteResult<u32> {
        let version: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        version.parse().map_err(|_| rusqlite::Error::InvalidQuery)
    }

    /// Check if the dirty flag is set (indicating interrupted sync).
    fn is_dirty(&self) -> bool {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = 'dirty'", [], |row| {
                row.get::<_, String>(0)
            })
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }

    /// Set or clear the dirty flag.
    #[allow(dead_code)]
    pub fn set_dirty(&self, dirty: bool) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('dirty', ?1)",
            [if dirty { "true" } else { "false" }],
        )?;
        Ok(())
    }

    /// Ensure the schema is created and up to date.
    fn ensure_schema(&mut self) -> SqliteResult<()> {
        // Check if we need to rebuild
        let needs_rebuild = self.needs_rebuild();

        if needs_rebuild {
            // Drop existing tables if rebuilding
            self.drop_all_tables()?;
        }

        // Create schema
        self.conn.execute_batch(include_str!("schema.sql"))?;

        // Set schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
            [SCHEMA_VERSION.to_string()],
        )?;

        Ok(())
    }

    /// Drop all tables for a clean rebuild.
    fn drop_all_tables(&mut self) -> SqliteResult<()> {
        // Drop in reverse order of dependencies
        self.conn.execute_batch(
            "
            DROP TABLE IF EXISTS embeddings;
            DROP TABLE IF EXISTS fts;
            DROP TABLE IF EXISTS events;
            DROP TABLE IF EXISTS critiques;
            DROP TABLE IF EXISTS solutions;
            DROP TABLE IF EXISTS problems;
            DROP TABLE IF EXISTS milestones;
            DROP TABLE IF EXISTS meta;
            ",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().expect("Failed to open in-memory database");

        // Verify tables exist
        let tables: Vec<String> = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"meta".to_string()));
        assert!(tables.contains(&"problems".to_string()));
        assert!(tables.contains(&"solutions".to_string()));
        assert!(tables.contains(&"critiques".to_string()));
        assert!(tables.contains(&"milestones".to_string()));
        assert!(tables.contains(&"events".to_string()));
    }

    #[test]
    fn test_schema_version() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // Schema version should be set
        let version = db
            .get_schema_version()
            .expect("Failed to get schema version");
        assert_eq!(version, SCHEMA_VERSION);

        // Should not need rebuild on fresh database
        assert!(!db.needs_rebuild());
    }

    #[test]
    fn test_dirty_flag() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // Should not be dirty initially
        assert!(!db.is_dirty());

        // Set dirty
        db.set_dirty(true).expect("Failed to set dirty flag");
        assert!(db.is_dirty());

        // Clear dirty
        db.set_dirty(false).expect("Failed to clear dirty flag");
        assert!(!db.is_dirty());
    }

    #[test]
    fn test_indexes_created() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // Check that indexes exist
        let indexes: Vec<String> = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(indexes.contains(&"idx_events_timestamp".to_string()));
        assert!(indexes.contains(&"idx_events_event_type".to_string()));
        assert!(indexes.contains(&"idx_events_entity_id".to_string()));
        assert!(indexes.contains(&"idx_solutions_problem_id".to_string()));
        assert!(indexes.contains(&"idx_critiques_solution_id".to_string()));
        assert!(indexes.contains(&"idx_problems_milestone_id".to_string()));
        assert!(indexes.contains(&"idx_problems_parent_id".to_string()));
    }

    #[test]
    fn test_fts_table_exists() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // FTS virtual table should exist
        let fts_exists: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(fts_exists);
    }

    #[test]
    fn test_embeddings_table_exists() {
        let db = Database::open_in_memory().expect("Failed to open database");

        // Embeddings table should exist
        let embeddings_exists: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='embeddings'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(embeddings_exists);

        // Check index exists
        let index_exists: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='idx_embeddings_model'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(index_exists);
    }

    #[test]
    fn test_open_file_database() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        // Create database
        {
            let db = Database::open(&db_path).expect("Failed to create database");
            assert!(!db.needs_rebuild());
        }

        // Reopen database
        {
            let db = Database::open(&db_path).expect("Failed to reopen database");
            assert!(!db.needs_rebuild());
            assert_eq!(db.get_schema_version().unwrap(), SCHEMA_VERSION);
        }
    }
}
