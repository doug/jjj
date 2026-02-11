//! Integration tests for embedding search functionality.
//!
//! Note: These tests don't require a real embedding service - they test
//! the search infrastructure with manually inserted embeddings.

use jjj::db::entities::upsert_problem;
use jjj::db::search::{find_similar, merge_with_rrf, SearchResult, SimilarityResult};
use jjj::db::{embeddings, Database};
use jjj::models::Problem;
use jjj::resolve::parse_entity_reference;

#[test]
fn test_parse_entity_reference() {
    assert_eq!(
        parse_entity_reference("p/abc123"),
        Some(("problem", "abc123"))
    );
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
    let results = find_similar(conn, "problem", "p1", None, 10).expect("Failed to search");

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

    let semantic = vec![
        SimilarityResult {
            entity_type: "problem".to_string(),
            entity_id: "p2".to_string(),
            title: "Second".to_string(),
            similarity: 0.95,
        },
        SimilarityResult {
            entity_type: "problem".to_string(),
            entity_id: "p3".to_string(),
            title: "Third".to_string(),
            similarity: 0.85,
        },
    ];

    let merged = merge_with_rrf(fts, semantic, 60);

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
