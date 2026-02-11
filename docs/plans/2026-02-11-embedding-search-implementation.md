# Embedding Search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add semantic embedding support to jjj search with hybrid FTS+semantic ranking and entity similarity detection.

**Architecture:** OpenAI-compatible embedding API client (defaulting to local Ollama), SQLite storage for embeddings, cosine similarity in Rust, smart search that detects entity references vs text queries and combines FTS with semantic results using Reciprocal Rank Fusion.

**Tech Stack:** Rust, reqwest (HTTP client), rusqlite (SQLite), serde (JSON serialization)

---

## Task 1: Add reqwest Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add reqwest with blocking feature**

In `Cargo.toml`, add to the `[dependencies]` section after line 45 (after rusqlite):

```toml
# HTTP client for embedding API
reqwest = { version = "0.12", features = ["blocking", "json"] }
```

**Step 2: Verify build compiles**

Run: `cargo build`
Expected: Build succeeds with reqwest downloaded

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add reqwest for embedding API calls"
```

---

## Task 2: Create Local Config Module

**Files:**
- Create: `src/local_config.rs`
- Modify: `src/lib.rs`

**Step 1: Write test for LocalConfig loading**

Create `src/local_config.rs`:

```rust
//! Local configuration for jjj stored in .jj/jjj.toml
//!
//! This config is never synced - it's local to each machine.
//! Used for embedding service configuration and other local settings.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for the embedding service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    /// Explicitly enable embeddings (if true, warn on connection failure)
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Base URL for the embedding API (default: http://localhost:11434/v1)
    #[serde(default)]
    pub base_url: Option<String>,

    /// Model name (e.g., "qwen3-embedding:8b")
    #[serde(default)]
    pub model: Option<String>,

    /// Embedding dimensions
    #[serde(default)]
    pub dimensions: Option<usize>,

    /// API key for remote services
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Local configuration stored in .jj/jjj.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalConfig {
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
}

impl LocalConfig {
    /// Load config from .jj/jjj.toml, with env var overrides.
    /// Returns default config if file doesn't exist.
    pub fn load(repo_root: &Path) -> Self {
        let config_path = repo_root.join(".jj").join("jjj.toml");

        let mut config = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            LocalConfig::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();
        config
    }

    /// Apply environment variable overrides.
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_ENABLED") {
            self.embeddings.enabled = Some(val == "true" || val == "1");
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_BASE_URL") {
            self.embeddings.base_url = Some(val);
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_MODEL") {
            self.embeddings.model = Some(val);
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_DIMENSIONS") {
            if let Ok(dims) = val.parse() {
                self.embeddings.dimensions = Some(dims);
            }
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_API_KEY") {
            self.embeddings.api_key = Some(val);
        }
    }

    /// Check if embeddings are explicitly enabled in config.
    pub fn embeddings_explicitly_enabled(&self) -> bool {
        self.embeddings.enabled == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = LocalConfig::default();
        assert!(config.embeddings.enabled.is_none());
        assert!(config.embeddings.base_url.is_none());
        assert!(config.embeddings.model.is_none());
    }

    #[test]
    fn test_env_overrides() {
        // Set env vars
        env::set_var("JJJ_EMBEDDINGS_ENABLED", "true");
        env::set_var("JJJ_EMBEDDINGS_BASE_URL", "http://test:8080/v1");
        env::set_var("JJJ_EMBEDDINGS_MODEL", "test-model");
        env::set_var("JJJ_EMBEDDINGS_DIMENSIONS", "1024");

        let mut config = LocalConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.embeddings.enabled, Some(true));
        assert_eq!(
            config.embeddings.base_url,
            Some("http://test:8080/v1".to_string())
        );
        assert_eq!(config.embeddings.model, Some("test-model".to_string()));
        assert_eq!(config.embeddings.dimensions, Some(1024));

        // Clean up
        env::remove_var("JJJ_EMBEDDINGS_ENABLED");
        env::remove_var("JJJ_EMBEDDINGS_BASE_URL");
        env::remove_var("JJJ_EMBEDDINGS_MODEL");
        env::remove_var("JJJ_EMBEDDINGS_DIMENSIONS");
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
[embeddings]
enabled = true
base_url = "http://localhost:11434/v1"
model = "qwen3-embedding:8b"
dimensions = 4096
"#;

        let config: LocalConfig = toml::from_str(toml_str).expect("Failed to parse TOML");
        assert_eq!(config.embeddings.enabled, Some(true));
        assert_eq!(
            config.embeddings.base_url,
            Some("http://localhost:11434/v1".to_string())
        );
        assert_eq!(
            config.embeddings.model,
            Some("qwen3-embedding:8b".to_string())
        );
        assert_eq!(config.embeddings.dimensions, Some(4096));
    }
}
```

**Step 2: Run tests to verify**

Run: `cargo test local_config`
Expected: All 3 tests pass

**Step 3: Add module to lib.rs**

In `src/lib.rs`, add after line 10 (after `pub mod id;`):

```rust
pub mod local_config;
```

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/local_config.rs src/lib.rs
git commit -m "feat: add local config module for .jj/jjj.toml"
```

---

## Task 3: Create Embeddings Client Module

**Files:**
- Create: `src/embeddings.rs`
- Modify: `src/lib.rs`

**Step 1: Write the embeddings client with tests**

Create `src/embeddings.rs`:

```rust
//! Embedding client for computing vector embeddings via OpenAI-compatible APIs.
//!
//! Supports Ollama (default), OpenAI, and other compatible providers.

use crate::local_config::LocalConfig;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Default Ollama embedding endpoint
const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";
const DEFAULT_MODEL: &str = "nomic-embed-text";
const DEFAULT_DIMENSIONS: usize = 768;

/// Track if we've already warned about connection failure this session
static WARNED_THIS_SESSION: AtomicBool = AtomicBool::new(false);

/// Request body for the OpenAI-compatible embeddings API
#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
}

/// Response from the embeddings API
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Error type for embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API returned error: {0}")]
    Api(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Empty response from API")]
    EmptyResponse,
}

/// Client for computing embeddings via OpenAI-compatible API.
pub struct EmbeddingClient {
    base_url: String,
    model: String,
    dimensions: usize,
    api_key: Option<String>,
    http_client: reqwest::blocking::Client,
}

impl EmbeddingClient {
    /// Create a new embedding client from config.
    ///
    /// If `warn_on_error` is true and connection fails, logs a warning.
    /// Returns None if the service is unavailable.
    pub fn from_config(config: &LocalConfig, warn_on_error: bool) -> Option<Self> {
        let base_url = config
            .embeddings
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let model = config
            .embeddings
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let dimensions = config.embeddings.dimensions.unwrap_or(DEFAULT_DIMENSIONS);
        let api_key = config.embeddings.api_key.clone();

        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;

        let client = Self {
            base_url,
            model,
            dimensions,
            api_key,
            http_client,
        };

        // Test connection with a simple embed request
        match client.embed("test") {
            Ok(_) => Some(client),
            Err(e) => {
                if warn_on_error && !WARNED_THIS_SESSION.swap(true, Ordering::SeqCst) {
                    eprintln!(
                        "Warning: Embedding service unavailable at {}: {}",
                        client.base_url, e
                    );
                    eprintln!("Semantic search features will be disabled.");
                }
                None
            }
        }
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Compute embedding for a single text.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embeddings = self.embed_batch(&[text])?;
        embeddings
            .into_iter()
            .next()
            .ok_or(EmbeddingError::EmptyResponse)
    }

    /// Compute embeddings for multiple texts in a single API call.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/embeddings", self.base_url);
        let request = EmbeddingRequest {
            model: &self.model,
            input: texts.to_vec(),
        };

        let mut req = self.http_client.post(&url).json(&request);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(EmbeddingError::Api(format!("{}: {}", status, body)));
        }

        let response: EmbeddingResponse = response.json()?;

        // Validate dimensions
        for (i, data) in response.data.iter().enumerate() {
            if data.embedding.len() != self.dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: data.embedding.len(),
                });
            }
            // Only check first one in production for performance
            if i == 0 {
                break;
            }
        }

        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (ai, bi) in a.iter().zip(b.iter()) {
        dot_product += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot_product / denominator
    }
}

/// Prepare text for embedding from a problem.
pub fn prepare_problem_text(title: &str, description: &str, context: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, description, context)
        .trim()
        .to_string()
}

/// Prepare text for embedding from a solution.
pub fn prepare_solution_text(title: &str, approach: &str, tradeoffs: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, approach, tradeoffs)
        .trim()
        .to_string()
}

/// Prepare text for embedding from a critique.
pub fn prepare_critique_text(title: &str, argument: &str, evidence: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, argument, evidence)
        .trim()
        .to_string()
}

/// Prepare text for embedding from a milestone.
pub fn prepare_milestone_text(title: &str, goals: &str, success_criteria: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, goals, success_criteria)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_prepare_problem_text() {
        let text = prepare_problem_text("Title", "Description", "Context");
        assert_eq!(text, "Title\n\nDescription\n\nContext");
    }

    #[test]
    fn test_prepare_solution_text() {
        let text = prepare_solution_text("Title", "Approach", "Tradeoffs");
        assert_eq!(text, "Title\n\nApproach\n\nTradeoffs");
    }

    #[test]
    fn test_prepare_critique_text() {
        let text = prepare_critique_text("Title", "Argument", "Evidence");
        assert_eq!(text, "Title\n\nArgument\n\nEvidence");
    }

    #[test]
    fn test_prepare_milestone_text() {
        let text = prepare_milestone_text("Title", "Goals", "Criteria");
        assert_eq!(text, "Title\n\nGoals\n\nCriteria");
    }
}
```

**Step 2: Run tests**

Run: `cargo test embeddings`
Expected: All 9 tests pass

**Step 3: Add module to lib.rs**

In `src/lib.rs`, add after line 8 (after `pub mod error;`):

```rust
pub mod embeddings;
```

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/embeddings.rs src/lib.rs
git commit -m "feat: add embedding client with cosine similarity"
```

---

## Task 4: Add Embeddings Table to Schema

**Files:**
- Modify: `src/db/schema.sql`
- Modify: `src/db/schema.rs`

**Step 1: Add embeddings table to schema.sql**

In `src/db/schema.sql`, add after line 105 (after the last index), before the empty line at end:

```sql

-- Embeddings table for semantic search
CREATE TABLE IF NOT EXISTS embeddings (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    model TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_embeddings_model ON embeddings(model);
```

**Step 2: Bump schema version**

In `src/db/schema.rs`, change line 11:

From:
```rust
pub const SCHEMA_VERSION: u32 = 1;
```

To:
```rust
pub const SCHEMA_VERSION: u32 = 2;
```

**Step 3: Update drop_all_tables to include embeddings**

In `src/db/schema.rs`, in the `drop_all_tables` function (around line 111-125), update the SQL:

From:
```rust
    fn drop_all_tables(&mut self) -> SqliteResult<()> {
        // Drop in reverse order of dependencies
        self.conn.execute_batch(
            "
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
```

To:
```rust
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
```

**Step 4: Add test for embeddings table**

In `src/db/schema.rs`, add a new test after `test_fts_table_exists` (around line 222):

```rust
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
```

**Step 5: Run tests**

Run: `cargo test db::schema`
Expected: All 7 tests pass (including new test)

**Step 6: Commit**

```bash
git add src/db/schema.sql src/db/schema.rs
git commit -m "feat: add embeddings table to SQLite schema (v2)"
```

---

## Task 5: Add Embeddings Database Operations

**Files:**
- Create: `src/db/embeddings.rs`
- Modify: `src/db/mod.rs`

**Step 1: Create embeddings database module**

Create `src/db/embeddings.rs`:

```rust
//! Database operations for embedding storage and retrieval.

use rusqlite::{params, Connection, Result as SqliteResult};

/// An embedding record from the database.
#[derive(Debug, Clone)]
pub struct EmbeddingRecord {
    pub entity_type: String,
    pub entity_id: String,
    pub model: String,
    pub dimensions: usize,
    pub embedding: Vec<f32>,
    pub created_at: String,
}

/// Store or update an embedding for an entity.
pub fn upsert_embedding(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    model: &str,
    embedding: &[f32],
) -> SqliteResult<()> {
    let dimensions = embedding.len();
    let blob = embedding_to_blob(embedding);
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO embeddings (entity_type, entity_id, model, dimensions, embedding, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![entity_type, entity_id, model, dimensions, blob, now],
    )?;

    Ok(())
}

/// Load an embedding for a specific entity.
pub fn load_embedding(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> SqliteResult<Option<EmbeddingRecord>> {
    let result = conn.query_row(
        "SELECT entity_type, entity_id, model, dimensions, embedding, created_at
         FROM embeddings
         WHERE entity_type = ?1 AND entity_id = ?2",
        params![entity_type, entity_id],
        |row| {
            let blob: Vec<u8> = row.get(4)?;
            Ok(EmbeddingRecord {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                model: row.get(2)?,
                dimensions: row.get(3)?,
                embedding: blob_to_embedding(&blob),
                created_at: row.get(5)?,
            })
        },
    );

    match result {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Load all embeddings, optionally filtered by entity type.
pub fn list_embeddings(
    conn: &Connection,
    entity_type: Option<&str>,
) -> SqliteResult<Vec<EmbeddingRecord>> {
    let mut records = Vec::new();

    let sql = match entity_type {
        Some(_) => {
            "SELECT entity_type, entity_id, model, dimensions, embedding, created_at
             FROM embeddings
             WHERE entity_type = ?1"
        }
        None => {
            "SELECT entity_type, entity_id, model, dimensions, embedding, created_at
             FROM embeddings"
        }
    };

    let mut stmt = conn.prepare(sql)?;

    let rows = if let Some(et) = entity_type {
        stmt.query_map(params![et], row_to_record)?
    } else {
        stmt.query_map([], row_to_record)?
    };

    for row in rows {
        records.push(row?);
    }

    Ok(records)
}

/// Delete an embedding for an entity.
pub fn delete_embedding(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM embeddings WHERE entity_type = ?1 AND entity_id = ?2",
        params![entity_type, entity_id],
    )?;
    Ok(())
}

/// Clear all embeddings (used during rebuild).
pub fn clear_embeddings(conn: &Connection) -> SqliteResult<()> {
    conn.execute("DELETE FROM embeddings", [])?;
    Ok(())
}

/// Count embeddings, optionally by model.
pub fn count_embeddings(conn: &Connection, model: Option<&str>) -> SqliteResult<(usize, usize)> {
    let total: usize = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;

    let matching = if let Some(m) = model {
        conn.query_row(
            "SELECT COUNT(*) FROM embeddings WHERE model = ?1",
            params![m],
            |row| row.get(0),
        )?
    } else {
        total
    };

    Ok((matching, total))
}

/// Get the current embedding model (if any embeddings exist).
pub fn get_embedding_model(conn: &Connection) -> SqliteResult<Option<String>> {
    let result = conn.query_row(
        "SELECT model FROM embeddings LIMIT 1",
        [],
        |row| row.get(0),
    );

    match result {
        Ok(model) => Ok(Some(model)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// Helper to convert row to EmbeddingRecord
fn row_to_record(row: &rusqlite::Row) -> SqliteResult<EmbeddingRecord> {
    let blob: Vec<u8> = row.get(4)?;
    Ok(EmbeddingRecord {
        entity_type: row.get(0)?,
        entity_id: row.get(1)?,
        model: row.get(2)?,
        dimensions: row.get(3)?,
        embedding: blob_to_embedding(&blob),
        created_at: row.get(5)?,
    })
}

/// Convert f32 vector to blob for storage.
fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(embedding.len() * 4);
    for f in embedding {
        blob.extend_from_slice(&f.to_le_bytes());
    }
    blob
}

/// Convert blob back to f32 vector.
fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_embedding_blob_roundtrip() {
        let original = vec![1.0f32, 2.5, -3.7, 0.0, 1e-6];
        let blob = embedding_to_blob(&original);
        let recovered = blob_to_embedding(&blob);

        assert_eq!(original.len(), recovered.len());
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-10);
        }
    }

    #[test]
    fn test_upsert_and_load_embedding() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        upsert_embedding(conn, "problem", "p1", "test-model", &embedding)
            .expect("Failed to upsert");

        let record = load_embedding(conn, "problem", "p1")
            .expect("Failed to load")
            .expect("Should exist");

        assert_eq!(record.entity_type, "problem");
        assert_eq!(record.entity_id, "p1");
        assert_eq!(record.model, "test-model");
        assert_eq!(record.dimensions, 4);
        assert_eq!(record.embedding.len(), 4);
    }

    #[test]
    fn test_upsert_replaces_existing() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        let embedding1 = vec![0.1, 0.2, 0.3, 0.4];
        upsert_embedding(conn, "problem", "p1", "model-v1", &embedding1).expect("Failed to upsert");

        let embedding2 = vec![0.5, 0.6, 0.7, 0.8];
        upsert_embedding(conn, "problem", "p1", "model-v2", &embedding2).expect("Failed to upsert");

        let record = load_embedding(conn, "problem", "p1")
            .expect("Failed to load")
            .expect("Should exist");

        assert_eq!(record.model, "model-v2");
        assert!((record.embedding[0] - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_list_embeddings() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        upsert_embedding(conn, "problem", "p1", "model", &[0.1, 0.2]).expect("Failed to upsert");
        upsert_embedding(conn, "problem", "p2", "model", &[0.3, 0.4]).expect("Failed to upsert");
        upsert_embedding(conn, "solution", "s1", "model", &[0.5, 0.6]).expect("Failed to upsert");

        // List all
        let all = list_embeddings(conn, None).expect("Failed to list");
        assert_eq!(all.len(), 3);

        // List only problems
        let problems = list_embeddings(conn, Some("problem")).expect("Failed to list");
        assert_eq!(problems.len(), 2);
    }

    #[test]
    fn test_delete_embedding() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        upsert_embedding(conn, "problem", "p1", "model", &[0.1, 0.2]).expect("Failed to upsert");

        delete_embedding(conn, "problem", "p1").expect("Failed to delete");

        let record = load_embedding(conn, "problem", "p1").expect("Failed to load");
        assert!(record.is_none());
    }

    #[test]
    fn test_clear_embeddings() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        upsert_embedding(conn, "problem", "p1", "model", &[0.1]).expect("Failed to upsert");
        upsert_embedding(conn, "solution", "s1", "model", &[0.2]).expect("Failed to upsert");

        clear_embeddings(conn).expect("Failed to clear");

        let (matching, total) = count_embeddings(conn, None).expect("Failed to count");
        assert_eq!(total, 0);
        assert_eq!(matching, 0);
    }

    #[test]
    fn test_count_embeddings() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        upsert_embedding(conn, "problem", "p1", "model-a", &[0.1]).expect("Failed to upsert");
        upsert_embedding(conn, "problem", "p2", "model-a", &[0.2]).expect("Failed to upsert");
        upsert_embedding(conn, "solution", "s1", "model-b", &[0.3]).expect("Failed to upsert");

        let (matching, total) = count_embeddings(conn, Some("model-a")).expect("Failed to count");
        assert_eq!(total, 3);
        assert_eq!(matching, 2);
    }

    #[test]
    fn test_get_embedding_model() {
        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Empty database
        let model = get_embedding_model(conn).expect("Failed to get model");
        assert!(model.is_none());

        // After inserting
        upsert_embedding(conn, "problem", "p1", "test-model", &[0.1]).expect("Failed to upsert");
        let model = get_embedding_model(conn).expect("Failed to get model");
        assert_eq!(model, Some("test-model".to_string()));
    }
}
```

**Step 2: Add module to db/mod.rs**

In `src/db/mod.rs`, add a new line after the existing module declarations. First, read the current file to see its structure, then add:

```rust
pub mod embeddings;
```

And in the pub use section, add:

```rust
pub use embeddings::{
    clear_embeddings, count_embeddings, delete_embedding, get_embedding_model, list_embeddings,
    load_embedding, upsert_embedding, EmbeddingRecord,
};
```

**Step 3: Run tests**

Run: `cargo test db::embeddings`
Expected: All 8 tests pass

**Step 4: Commit**

```bash
git add src/db/embeddings.rs src/db/mod.rs
git commit -m "feat: add embedding storage CRUD operations"
```

---

## Task 6: Add Embedding Rebuild to Sync

**Files:**
- Modify: `src/db/sync.rs`

**Step 1: Add rebuild_embeddings function**

In `src/db/sync.rs`, add after the `rebuild_fts` function (around line 158):

```rust
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
        let text =
            prepare_solution_text(&solution.title, &solution.approach, &solution.tradeoffs);
        if let Ok(embedding) = client.embed(&text) {
            upsert_embedding(conn, "solution", &solution.id, model, &embedding)?;
        }
    }

    // Process critiques
    let critiques = list_critiques(conn)?;
    for critique in &critiques {
        let text =
            prepare_critique_text(&critique.title, &critique.argument, &critique.evidence);
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
```

**Step 2: Update clear_all_tables to include embeddings**

In `src/db/sync.rs`, in the `clear_all_tables` function (around line 175-184), add embeddings:

From:
```rust
fn clear_all_tables(conn: &Connection) -> Result<()> {
    // Clear in reverse order of dependencies
    conn.execute("DELETE FROM critiques", [])?;
    conn.execute("DELETE FROM solutions", [])?;
    conn.execute("DELETE FROM problems", [])?;
    conn.execute("DELETE FROM milestones", [])?;
    clear_events(conn)?;
    conn.execute("DELETE FROM fts", [])?;
    Ok(())
}
```

To:
```rust
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
```

**Step 3: Run tests**

Run: `cargo test db::sync`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/db/sync.rs
git commit -m "feat: add rebuild_embeddings to sync module"
```

---

## Task 7: Add Semantic Search Functions

**Files:**
- Modify: `src/db/search.rs`

**Step 1: Add semantic search functions**

In `src/db/search.rs`, add after the `search_events` function (around line 245), before the tests:

```rust
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
            exclude_id.map_or(true, |id| e.entity_id != id)
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
    results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

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

    // Add FTS scores
    for (rank, result) in fts_results.iter().enumerate() {
        let key = (result.entity_type.clone(), result.entity_id.clone());
        let rrf_score = 1.0 / (k as f32 + rank as f32 + 1.0);
        *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
        titles.insert(key, result.title.clone());
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

    // Convert to SearchResult
    merged
        .into_iter()
        .map(|((entity_type, entity_id), _score)| {
            let title = titles
                .get(&(entity_type.clone(), entity_id.clone()))
                .cloned()
                .unwrap_or_default();
            SearchResult {
                entity_type,
                entity_id,
                title,
                snippet: String::new(), // RRF results don't have snippets
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
```

**Step 2: Add tests for semantic search**

In `src/db/search.rs`, add new tests at the end of the `mod tests` block:

```rust
    #[test]
    fn test_similarity_search() {
        use crate::db::embeddings::upsert_embedding;

        let db = Database::open_in_memory().expect("Failed to open database");
        let conn = db.conn();

        // Insert problems with embeddings
        let mut p1 = Problem::new("p1".to_string(), "Auth problem".to_string());
        upsert_problem(conn, &p1).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0, 0.0]).expect("Failed");

        let mut p2 = Problem::new("p2".to_string(), "Similar auth issue".to_string());
        upsert_problem(conn, &p2).expect("Failed to insert");
        upsert_embedding(conn, "problem", "p2", "test", &[0.9, 0.1, 0.0]).expect("Failed");

        let mut p3 = Problem::new("p3".to_string(), "Unrelated problem".to_string());
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
```

**Step 3: Run tests**

Run: `cargo test db::search`
Expected: All tests pass (including 3 new ones)

**Step 4: Commit**

```bash
git add src/db/search.rs
git commit -m "feat: add semantic search and RRF merge functions"
```

---

## Task 8: Add Entity Reference Parser

**Files:**
- Modify: `src/resolve.rs`

**Step 1: Add entity reference parsing function**

Read the current `src/resolve.rs` file first, then add at the end (before tests):

```rust
/// Parse an entity reference like "p/01957d" or "s/abc123".
///
/// Returns (entity_type, id_prefix) if valid, None otherwise.
pub fn parse_entity_reference(input: &str) -> Option<(&str, &str)> {
    // Must be at least 3 chars: "p/" + 1 char
    if input.len() < 3 {
        return None;
    }

    // Check for type prefix followed by /
    let (type_char, rest) = input.split_at(1);
    if !rest.starts_with('/') {
        return None;
    }

    let id = &rest[1..];
    if id.is_empty() {
        return None;
    }

    let entity_type = match type_char {
        "p" => "problem",
        "s" => "solution",
        "c" => "critique",
        "m" => "milestone",
        _ => return None,
    };

    Some((entity_type, id))
}
```

**Step 2: Add tests**

Add to the tests module in `src/resolve.rs`:

```rust
    #[test]
    fn test_parse_entity_reference_valid() {
        assert_eq!(
            parse_entity_reference("p/01957d"),
            Some(("problem", "01957d"))
        );
        assert_eq!(
            parse_entity_reference("s/abc123"),
            Some(("solution", "abc123"))
        );
        assert_eq!(
            parse_entity_reference("c/xyz"),
            Some(("critique", "xyz"))
        );
        assert_eq!(
            parse_entity_reference("m/123"),
            Some(("milestone", "123"))
        );
    }

    #[test]
    fn test_parse_entity_reference_invalid() {
        assert_eq!(parse_entity_reference("p/"), None);
        assert_eq!(parse_entity_reference("x/123"), None);
        assert_eq!(parse_entity_reference("problem"), None);
        assert_eq!(parse_entity_reference("p123"), None);
        assert_eq!(parse_entity_reference(""), None);
        assert_eq!(parse_entity_reference("p"), None);
    }
```

**Step 3: Run tests**

Run: `cargo test resolve`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/resolve.rs
git commit -m "feat: add entity reference parser for smart search"
```

---

## Task 9: Enhance Search Command with Hybrid Search

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/search.rs`

**Step 1: Add --text-only flag to CLI**

In `src/cli.rs`, find the `Search` command (around line 75-87) and add the flag:

From:
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

To:
```rust
    /// Search across all entities
    Search {
        /// Search query (or entity reference like p/01957d for similarity search)
        query: String,

        /// Filter by entity type (problem, solution, critique, milestone, event)
        #[arg(long, short = 't')]
        r#type: Option<String>,

        /// Force text-only search (disable semantic/embedding features)
        #[arg(long)]
        text_only: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
```

**Step 2: Update command dispatch**

In `src/commands/mod.rs`, update the Search match arm (around line 86-90):

From:
```rust
        Commands::Search {
            query,
            r#type,
            json,
        } => search::execute(ctx, &query, r#type.as_deref(), json),
```

To:
```rust
        Commands::Search {
            query,
            r#type,
            text_only,
            json,
        } => search::execute(ctx, &query, r#type.as_deref(), text_only, json),
```

**Step 3: Rewrite search command with hybrid support**

Replace `src/commands/search.rs` with:

```rust
use crate::context::CommandContext;
use crate::db::{self, search, Database};
use crate::embeddings::EmbeddingClient;
use crate::error::Result;
use crate::local_config::LocalConfig;
use crate::resolve::parse_entity_reference;

pub fn execute(
    ctx: &CommandContext,
    query: &str,
    entity_type: Option<&str>,
    text_only: bool,
    json: bool,
) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    // Ensure DB exists
    let db = if db_path.exists() {
        Database::open(&db_path)?
    } else {
        let db = Database::open(&db_path)?;
        db::load_from_markdown(&db, &ctx.store)?;
        db
    };

    // Load local config and try to get embedding client
    let local_config = LocalConfig::load(repo_root);
    let embedding_client = if text_only {
        None
    } else {
        EmbeddingClient::from_config(&local_config, local_config.embeddings_explicitly_enabled())
    };

    // Check if query is an entity reference (e.g., "p/01957d")
    if let Some((ref_type, ref_id)) = parse_entity_reference(query) {
        return execute_similarity_search(
            &db,
            ref_type,
            ref_id,
            entity_type,
            json,
        );
    }

    // Hybrid text search
    execute_hybrid_search(&db, query, entity_type, embedding_client.as_ref(), json)
}

fn execute_similarity_search(
    db: &Database,
    entity_type: &str,
    entity_id_prefix: &str,
    filter_type: Option<&str>,
    json: bool,
) -> Result<()> {
    // Resolve the entity ID prefix to full ID
    let conn = db.conn();
    let full_id = resolve_entity_id(conn, entity_type, entity_id_prefix)?;

    let results = search::find_similar(conn, entity_type, &full_id, filter_type, 20)?;

    if json {
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "type": r.entity_type,
                    "id": r.entity_id,
                    "title": r.title,
                    "similarity": r.similarity,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else if results.is_empty() {
        println!("No similar entities found for {}/{}", entity_type.chars().next().unwrap(), entity_id_prefix);
        println!("\nNote: Embeddings may not be computed. Run 'jjj db rebuild' with an embedding service running.");
    } else {
        println!(
            "Entities similar to {}/{}:\n",
            entity_type.chars().next().unwrap(),
            &full_id[..6.min(full_id.len())]
        );
        for result in results {
            let short_id = &result.entity_id[..6.min(result.entity_id.len())];
            println!(
                "  {}/{}  [{:.2}]  \"{}\"",
                result.entity_type.chars().next().unwrap(),
                short_id,
                result.similarity,
                result.title
            );
        }
    }

    Ok(())
}

fn execute_hybrid_search(
    db: &Database,
    query: &str,
    entity_type: Option<&str>,
    embedding_client: Option<&EmbeddingClient>,
    json: bool,
) -> Result<()> {
    let conn = db.conn();

    // Always do FTS search
    let fts_results = search::search(conn, query, entity_type)?;

    // Try semantic search if client available
    let final_results = if let Some(client) = embedding_client {
        if let Ok(query_embedding) = client.embed(query) {
            let semantic_results =
                search::similarity_search(conn, &query_embedding, entity_type, None, 50)?;

            if !semantic_results.is_empty() {
                // Merge with RRF
                search::merge_with_rrf(fts_results, semantic_results, 60)
            } else {
                fts_results
            }
        } else {
            fts_results
        }
    } else {
        fts_results
    };

    if json {
        let json_results: Vec<_> = final_results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "type": r.entity_type,
                    "id": r.entity_id,
                    "title": r.title,
                    "snippet": r.snippet,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else if final_results.is_empty() {
        println!("No results found for \"{}\"", query);
    } else {
        let hybrid_note = if embedding_client.is_some() { " (hybrid)" } else { "" };
        println!(
            "Found {} result(s) for \"{}\"{}:\n",
            final_results.len(),
            query,
            hybrid_note
        );
        for result in final_results {
            println!(
                "[{}] {} - {}",
                result.entity_type, result.entity_id, result.title
            );
            if !result.snippet.is_empty() {
                println!("    {}", result.snippet.replace('\n', " "));
            }
            println!();
        }
    }

    Ok(())
}

/// Resolve an entity ID prefix to the full ID.
fn resolve_entity_id(
    conn: &rusqlite::Connection,
    entity_type: &str,
    prefix: &str,
) -> Result<String> {
    let table = match entity_type {
        "problem" => "problems",
        "solution" => "solutions",
        "critique" => "critiques",
        "milestone" => "milestones",
        _ => return Err(crate::error::JjjError::EntityNotFound(prefix.to_string())),
    };

    let sql = format!("SELECT id FROM {} WHERE id LIKE ?1 || '%'", table);
    let pattern = prefix;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([pattern])?;

    let mut matches = Vec::new();
    while let Some(row) = rows.next()? {
        matches.push(row.get::<_, String>(0)?);
    }

    match matches.len() {
        0 => Err(crate::error::JjjError::EntityNotFound(prefix.to_string())),
        1 => Ok(matches.remove(0)),
        _ => Err(crate::error::JjjError::AmbiguousId {
            prefix: prefix.to_string(),
            matches,
        }),
    }
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Run build to verify**

Run: `cargo build`
Expected: Build succeeds

**Step 6: Commit**

```bash
git add src/cli.rs src/commands/mod.rs src/commands/search.rs
git commit -m "feat: add hybrid search with entity reference detection"
```

---

## Task 10: Add db status Command for Embedding Info

**Files:**
- Modify: `src/cli.rs`
- Create: `src/commands/db.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add Db command to CLI**

In `src/cli.rs`, add after the `Search` command (around line 91):

```rust
    /// Database management commands
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
```

And add the DbAction enum after the existing command enums (before `#[derive(ValueEnum, Clone, Debug)]`):

```rust
// =============================================================================
// Db Commands
// =============================================================================

#[derive(Subcommand)]
pub enum DbAction {
    /// Show database status
    Status,

    /// Rebuild database from markdown (includes FTS and embeddings)
    Rebuild,
}
```

**Step 2: Create db command handler**

Create `src/commands/db.rs`:

```rust
use crate::cli::DbAction;
use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::embeddings::EmbeddingClient;
use crate::error::Result;
use crate::local_config::LocalConfig;

pub fn execute(ctx: &CommandContext, action: DbAction) -> Result<()> {
    match action {
        DbAction::Status => status(ctx),
        DbAction::Rebuild => rebuild(ctx),
    }
}

fn status(ctx: &CommandContext) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    if !db_path.exists() {
        println!("Database: not initialized");
        println!("Run any jjj command to initialize, or 'jjj db rebuild' to create.");
        return Ok(());
    }

    let db = Database::open(&db_path)?;
    let conn = db.conn();

    // Get schema version
    let version: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'schema_version'", [], |row| row.get(0))
        .unwrap_or_else(|_| "unknown".to_string());

    // Count entities
    let problems: i64 = conn.query_row("SELECT COUNT(*) FROM problems", [], |row| row.get(0))?;
    let solutions: i64 = conn.query_row("SELECT COUNT(*) FROM solutions", [], |row| row.get(0))?;
    let critiques: i64 = conn.query_row("SELECT COUNT(*) FROM critiques", [], |row| row.get(0))?;
    let milestones: i64 = conn.query_row("SELECT COUNT(*) FROM milestones", [], |row| row.get(0))?;

    // Count FTS documents
    let fts_count: i64 = conn.query_row("SELECT COUNT(*) FROM fts", [], |row| row.get(0))?;

    // Get embedding info
    let (embedding_count, embedding_model) = {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;
        let model: Option<String> = conn
            .query_row("SELECT model FROM embeddings LIMIT 1", [], |row| row.get(0))
            .ok();
        (count, model)
    };

    // Check dirty flag
    let dirty: bool = conn
        .query_row("SELECT value FROM meta WHERE key = 'dirty'", [], |row| {
            let v: String = row.get(0)?;
            Ok(v == "true" || v == "1")
        })
        .unwrap_or(false);

    // Print status
    println!("Database: {}", db_path.display());
    println!("Schema version: v{}", version);
    println!(
        "Entities: {} problems, {} solutions, {} critiques, {} milestones",
        problems, solutions, critiques, milestones
    );
    println!("FTS index: {} documents", fts_count);

    let total_entities = problems + solutions + critiques + milestones;
    if let Some(model) = embedding_model {
        println!(
            "Embeddings: {}/{} (model: {})",
            embedding_count, total_entities, model
        );
    } else {
        println!("Embeddings: none");
    }

    println!(
        "Sync status: {}",
        if dirty { "dirty (uncommitted changes)" } else { "clean" }
    );

    Ok(())
}

fn rebuild(ctx: &CommandContext) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    // Delete existing database to force full rebuild
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let db = Database::open(&db_path)?;

    println!("Loading from markdown...");
    db::load_from_markdown(&db, &ctx.store)?;

    println!("Rebuilding FTS index...");
    db::rebuild_fts(&db)?;

    // Try to rebuild embeddings
    let local_config = LocalConfig::load(repo_root);
    let embedding_client =
        EmbeddingClient::from_config(&local_config, local_config.embeddings_explicitly_enabled());

    if let Some(client) = embedding_client {
        println!("Rebuilding embeddings (model: {})...", client.model());
        db::rebuild_embeddings(&db, &client)?;
        let (count, _) = db::count_embeddings(db.conn(), Some(client.model()))?;
        println!("  {} embeddings computed", count);
    } else {
        println!("Embeddings: skipped (no embedding service available)");
    }

    println!("Done!");
    Ok(())
}
```

**Step 3: Add module and dispatch**

In `src/commands/mod.rs`, add after line 1:

```rust
pub mod db;
```

And add to the imports from cli (around line 16):

```rust
use crate::cli::{Cli, Commands, DbAction};
```

And add the dispatch case in `execute_with_context` (add after the Milestone case, around line 40):

```rust
        Commands::Db { action } => db::execute(ctx, action),
```

**Step 4: Export rebuild_embeddings and count_embeddings from db module**

In `src/db/mod.rs`, add to the pub use statements:

```rust
pub use sync::rebuild_embeddings;
pub use embeddings::count_embeddings;
```

**Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Run build**

Run: `cargo build`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add src/cli.rs src/commands/db.rs src/commands/mod.rs src/db/mod.rs
git commit -m "feat: add jjj db status and rebuild commands"
```

---

## Task 11: Integration Test

**Files:**
- Create: `tests/embedding_search_test.rs`

**Step 1: Create integration test**

Create `tests/embedding_search_test.rs`:

```rust
//! Integration tests for embedding search functionality.
//!
//! Note: These tests don't require a real embedding service - they test
//! the search infrastructure with manually inserted embeddings.

use jjj::db::{Database, embeddings, search};
use jjj::models::Problem;
use jjj::db::entities::upsert_problem;
use jjj::resolve::parse_entity_reference;

#[test]
fn test_parse_entity_reference() {
    assert_eq!(parse_entity_reference("p/abc123"), Some(("problem", "abc123")));
    assert_eq!(parse_entity_reference("s/xyz"), Some(("solution", "xyz")));
    assert_eq!(parse_entity_reference("invalid"), None);
    assert_eq!(parse_entity_reference(""), None);
}

#[test]
fn test_similarity_search_with_manual_embeddings() {
    let db = Database::open_in_memory().expect("Failed to open database");
    let conn = db.conn();

    // Create problems
    let p1 = Problem::new("p1".to_string(), "Authentication bug".to_string());
    let p2 = Problem::new("p2".to_string(), "Login issue".to_string());
    let p3 = Problem::new("p3".to_string(), "Database performance".to_string());

    upsert_problem(conn, &p1).expect("Failed to insert p1");
    upsert_problem(conn, &p2).expect("Failed to insert p2");
    upsert_problem(conn, &p3).expect("Failed to insert p3");

    // Add embeddings manually (simulating what the embedding client would do)
    // p1 and p2 are similar (both about auth), p3 is different
    embeddings::upsert_embedding(conn, "problem", "p1", "test-model", &[1.0, 0.0, 0.0])
        .expect("Failed to insert embedding");
    embeddings::upsert_embedding(conn, "problem", "p2", "test-model", &[0.9, 0.1, 0.0])
        .expect("Failed to insert embedding");
    embeddings::upsert_embedding(conn, "problem", "p3", "test-model", &[0.0, 0.0, 1.0])
        .expect("Failed to insert embedding");

    // Find similar to p1
    let results = search::find_similar(conn, "problem", "p1", None, 10)
        .expect("Failed to search");

    assert_eq!(results.len(), 2);
    // p2 should be most similar to p1
    assert_eq!(results[0].entity_id, "p2");
    assert!(results[0].similarity > 0.8);
    // p3 should be least similar
    assert_eq!(results[1].entity_id, "p3");
    assert!(results[1].similarity < 0.2);
}

#[test]
fn test_hybrid_search_rrf_merge() {
    let fts = vec![
        search::SearchResult {
            entity_type: "problem".to_string(),
            entity_id: "p1".to_string(),
            title: "First".to_string(),
            snippet: "".to_string(),
        },
        search::SearchResult {
            entity_type: "problem".to_string(),
            entity_id: "p2".to_string(),
            title: "Second".to_string(),
            snippet: "".to_string(),
        },
    ];

    let semantic = vec![
        search::SimilarityResult {
            entity_type: "problem".to_string(),
            entity_id: "p2".to_string(),
            title: "Second".to_string(),
            similarity: 0.95,
        },
        search::SimilarityResult {
            entity_type: "problem".to_string(),
            entity_id: "p3".to_string(),
            title: "Third".to_string(),
            similarity: 0.85,
        },
    ];

    let merged = search::merge_with_rrf(fts, semantic, 60);

    // p2 appears in both lists, should be ranked first
    assert_eq!(merged[0].entity_id, "p2");
    // All 3 unique results should be present
    assert_eq!(merged.len(), 3);
}

#[test]
fn test_embedding_storage_roundtrip() {
    let db = Database::open_in_memory().expect("Failed to open database");
    let conn = db.conn();

    let original = vec![0.1f32, 0.2, 0.3, -0.4, 1e-6];

    embeddings::upsert_embedding(conn, "problem", "test-id", "model", &original)
        .expect("Failed to upsert");

    let record = embeddings::load_embedding(conn, "problem", "test-id")
        .expect("Failed to load")
        .expect("Should exist");

    assert_eq!(record.embedding.len(), original.len());
    for (a, b) in original.iter().zip(record.embedding.iter()) {
        assert!((a - b).abs() < 1e-6, "Embedding values don't match");
    }
}
```

**Step 2: Run the integration tests**

Run: `cargo test embedding_search`
Expected: All 4 tests pass

**Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add tests/embedding_search_test.rs
git commit -m "test: add integration tests for embedding search"
```

---

## Task 12: Final Verification and Documentation

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No errors (warnings OK)

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues (or run `cargo fmt` to fix)

**Step 4: Build release**

Run: `cargo build --release`
Expected: Build succeeds

**Step 5: Manual verification**

Test the commands (these will work without embedding service, falling back to FTS):

```bash
# Initialize if needed
cargo run -- init 2>/dev/null || true

# Create test data
cargo run -- problem new "Test authentication bug"
cargo run -- problem new "Login page slow"

# Test search
cargo run -- search "auth"
cargo run -- search "auth" --text-only

# Test db status
cargo run -- db status
```

**Step 6: Final commit**

```bash
git add -A
git commit -m "chore: verify embedding search implementation"
```

---

## Summary

This implementation plan covers Phase 1 of the embedding search feature:

1. **Tasks 1-3**: Core infrastructure (reqwest dependency, local config, embedding client)
2. **Tasks 4-6**: Database layer (schema, CRUD operations, rebuild)
3. **Tasks 7-8**: Search layer (semantic search, entity reference parsing)
4. **Tasks 9-10**: CLI layer (hybrid search command, db status/rebuild)
5. **Tasks 11-12**: Testing and verification

The implementation follows TDD principles with tests written before or alongside each feature. Each task is atomic and commits frequently for easy rollback.

**Key features delivered:**
- Local config in `.jj/jjj.toml` with env var overrides
- OpenAI-compatible embedding API client (Ollama default)
- SQLite storage for embeddings
- Smart `jjj search` with hybrid FTS+semantic ranking
- Entity reference detection (`p/01957d`) for similarity search
- `jjj db status` showing embedding coverage
- `jjj db rebuild` computing embeddings
- Graceful degradation when embedding service unavailable
