//! Schema migration infrastructure for jjj SQLite database.
//!
//! Supports incremental migrations for additive schema changes,
//! falling back to full rebuild when necessary.

use rusqlite::{Connection, Result as SqliteResult};

/// A single schema migration.
pub struct Migration {
    /// Target schema version after this migration.
    pub version: u32,
    /// Human-readable description.
    pub description: &'static str,
    /// If true, this migration requires a full drop-and-recreate instead of incremental SQL.
    pub requires_rebuild: bool,
    /// The migration function to run (ignored if requires_rebuild is true).
    pub up: fn(&Connection) -> SqliteResult<()>,
}

/// Registry of all known migrations, ordered by version.
///
/// Each migration upgrades from `version - 1` to `version`.
/// Add new migrations to the end of this list.
pub fn all_migrations() -> Vec<Migration> {
    vec![
        // Future migrations go here. Example:
        // Migration {
        //     version: 3,
        //     description: "Add tags column to problems",
        //     requires_rebuild: false,
        //     up: |conn| {
        //         conn.execute_batch("ALTER TABLE problems ADD COLUMN tags TEXT DEFAULT '[]';")?;
        //         Ok(())
        //     },
        // },
    ]
}

/// Run pending migrations from `current_version` up to the latest.
///
/// Returns `Ok(true)` if all migrations were applied successfully.
/// Returns `Ok(false)` if a migration requires a full rebuild.
/// Returns `Err` on migration failure.
pub fn run_migrations(conn: &Connection, current_version: u32) -> SqliteResult<bool> {
    let migrations = all_migrations();

    for migration in &migrations {
        if migration.version <= current_version {
            continue;
        }

        if migration.requires_rebuild {
            return Ok(false);
        }

        (migration.up)(conn)?;

        // Update schema version after each successful migration
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
            [migration.version.to_string()],
        )?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_no_migrations_needed() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Fresh DB should have current version, no migrations needed
        let result = run_migrations(conn, crate::db::SCHEMA_VERSION).unwrap();
        assert!(result);
    }

    #[test]
    fn test_all_migrations_returns_empty() {
        let migrations = all_migrations();
        // Currently no migrations registered
        assert!(migrations.is_empty());
    }
}
