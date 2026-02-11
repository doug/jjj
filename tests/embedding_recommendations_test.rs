//! Integration tests for embedding recommendations (Phase 2+3).

use jjj::db::entities::upsert_problem;
use jjj::db::search::{find_similar, similarity_search};
use jjj::db::{embeddings, Database};
use jjj::local_config::LocalConfig;
use jjj::models::Problem;

#[test]
fn test_find_similar_filters_low_similarity() {
    let db = Database::open_in_memory().expect("Failed to open database");
    let conn = db.conn();

    // Create problems with embeddings
    let p1 = Problem::new("p1".to_string(), "Auth bug".to_string());
    let p2 = Problem::new("p2".to_string(), "Similar auth issue".to_string());
    let p3 = Problem::new("p3".to_string(), "Unrelated database issue".to_string());

    upsert_problem(conn, &p1).expect("insert p1");
    upsert_problem(conn, &p2).expect("insert p2");
    upsert_problem(conn, &p3).expect("insert p3");

    // p1 and p2 have similar embeddings, p3 is different
    embeddings::upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0, 0.0])
        .expect("embed p1");
    embeddings::upsert_embedding(conn, "problem", "p2", "test", &[0.95, 0.05, 0.0])
        .expect("embed p2");
    embeddings::upsert_embedding(conn, "problem", "p3", "test", &[0.0, 0.0, 1.0])
        .expect("embed p3");

    // Find similar to p1
    let results = find_similar(conn, "problem", "p1", None, 10).expect("search");

    // Should return 2 results (p2 and p3, excluding p1 itself)
    assert_eq!(results.len(), 2);

    // p2 should be first (most similar)
    assert_eq!(results[0].entity_id, "p2");
    assert!(
        results[0].similarity > 0.9,
        "p2 should have high similarity"
    );

    // p3 should be last (dissimilar)
    assert_eq!(results[1].entity_id, "p3");
    assert!(results[1].similarity < 0.1, "p3 should have low similarity");
}

#[test]
fn test_duplicate_threshold_config_defaults() {
    let config = LocalConfig::default();

    // Default threshold should be 0.85
    assert_eq!(config.duplicate_threshold(), 0.85);

    // Default duplicate checking should be enabled
    assert!(config.duplicate_check_enabled());
}

#[test]
fn test_duplicate_threshold_config_custom() {
    let toml_str = r#"
[embeddings]
duplicate_threshold = 0.9
duplicate_check_enabled = false
"#;

    let config: LocalConfig = toml::from_str(toml_str).expect("Failed to parse TOML");

    assert_eq!(config.duplicate_threshold(), 0.9);
    assert!(!config.duplicate_check_enabled());
}

#[test]
fn test_similarity_search_respects_entity_type_filter() {
    let db = Database::open_in_memory().expect("Failed to open database");
    let conn = db.conn();

    // Create a problem and a solution with similar embeddings
    let p1 = Problem::new("p1".to_string(), "Auth problem".to_string());
    upsert_problem(conn, &p1).expect("insert problem");

    // Insert embeddings for problem and a "solution" (simulated)
    embeddings::upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0, 0.0])
        .expect("embed problem");
    embeddings::upsert_embedding(conn, "solution", "s1", "test", &[0.98, 0.02, 0.0])
        .expect("embed solution");

    // Search only for problems - should not return the solution
    let results =
        similarity_search(conn, &[1.0, 0.0, 0.0], Some("problem"), None, 10).expect("search");

    // Should only find problems
    assert!(results.iter().all(|r| r.entity_type == "problem"));
}

#[test]
fn test_similarity_search_excludes_specified_id() {
    let db = Database::open_in_memory().expect("Failed to open database");
    let conn = db.conn();

    let p1 = Problem::new("p1".to_string(), "Problem 1".to_string());
    let p2 = Problem::new("p2".to_string(), "Problem 2".to_string());

    upsert_problem(conn, &p1).expect("insert p1");
    upsert_problem(conn, &p2).expect("insert p2");

    embeddings::upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0]).expect("embed p1");
    embeddings::upsert_embedding(conn, "problem", "p2", "test", &[0.9, 0.1]).expect("embed p2");

    // Search with p1's embedding but exclude p1 from results
    let results = similarity_search(conn, &[1.0, 0.0], None, Some("p1"), 10).expect("search");

    // Should not include p1 in results
    assert!(results.iter().all(|r| r.entity_id != "p1"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, "p2");
}
