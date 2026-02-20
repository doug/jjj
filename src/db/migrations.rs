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
        Migration {
            version: 3,
            description: "Add argument, evidence, author columns to critiques",
            requires_rebuild: false,
            up: |conn| {
                conn.execute_batch(
                    "ALTER TABLE critiques ADD COLUMN argument TEXT DEFAULT '';
                     ALTER TABLE critiques ADD COLUMN evidence TEXT DEFAULT '';
                     ALTER TABLE critiques ADD COLUMN author TEXT;",
                )?;

                // Migrate existing data: split body into argument/evidence
                let mut stmt = conn.prepare("SELECT id, body FROM critiques WHERE body != ''")?;
                let rows: Vec<(String, String)> = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<Result<_, _>>()?;

                for (id, body) in rows {
                    let separator = "\n\n## Evidence\n\n";
                    let (argument, evidence) = if let Some(idx) = body.find(separator) {
                        (
                            body[..idx].to_string(),
                            body[idx + separator.len()..].to_string(),
                        )
                    } else {
                        (body, String::new())
                    };

                    conn.execute(
                        "UPDATE critiques SET argument = ?1, evidence = ?2 WHERE id = ?3",
                        rusqlite::params![argument, evidence, id],
                    )?;
                }

                // Copy reviewer to author for existing rows
                conn.execute_batch(
                    "UPDATE critiques SET author = reviewer WHERE author IS NULL AND reviewer IS NOT NULL;",
                )?;

                Ok(())
            },
        },
        Migration {
            version: 4,
            description: "Recreate FTS table without contentless mode (enables SELECT from FTS)",
            requires_rebuild: true,
            up: |_conn| Ok(()),
        },
        Migration {
            version: 5,
            description: "Add GitHub sync columns",
            requires_rebuild: false,
            up: |conn| {
                conn.execute_batch(
                    "ALTER TABLE problems ADD COLUMN github_issue INTEGER;
                     ALTER TABLE solutions ADD COLUMN github_pr INTEGER;
                     ALTER TABLE solutions ADD COLUMN github_branch TEXT;
                     ALTER TABLE critiques ADD COLUMN github_review_id INTEGER;
                     CREATE INDEX IF NOT EXISTS idx_problems_github_issue ON problems(github_issue);
                     CREATE INDEX IF NOT EXISTS idx_solutions_github_pr ON solutions(github_pr);
                     CREATE INDEX IF NOT EXISTS idx_critiques_github_review_id ON critiques(github_review_id);"
                )?;
                Ok(())
            },
        },
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
    fn test_migrations_registered() {
        let migrations = all_migrations();
        assert!(!migrations.is_empty());
        assert_eq!(migrations[0].version, 3);
        assert!(!migrations[0].requires_rebuild);
    }
}
