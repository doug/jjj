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
pub fn delete_embedding(conn: &Connection, entity_type: &str, entity_id: &str) -> SqliteResult<()> {
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
    let result = conn.query_row("SELECT model FROM embeddings LIMIT 1", [], |row| row.get(0));

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
