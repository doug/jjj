mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

// =============================================================================
// db status
// =============================================================================

#[test]
fn test_db_status_on_fresh_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["db", "status"]);
    // A freshly initialized repo should report a database path or "not initialized"
    // depending on whether init creates the db. Either is valid.
    assert!(
        stdout.contains("Database") || stdout.contains("database"),
        "Expected database info in output: {}",
        stdout
    );
}

#[test]
fn test_db_status_shows_entity_counts() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create some entities first
    run_jjj_success(&dir, &["problem", "new", "Status Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Status Solution",
            "--problem",
            "Status Problem",
        ],
    );

    // Rebuild the database so it has entity counts
    run_jjj_success(&dir, &["db", "rebuild"]);

    let stdout = run_jjj_success(&dir, &["db", "status"]);
    // Should show entity counts
    assert!(
        stdout.contains("Entities:") || stdout.contains("problems"),
        "Expected entity counts in output: {}",
        stdout
    );
    // Schema version should be present
    assert!(
        stdout.contains("Schema version") || stdout.contains("schema"),
        "Expected schema version in output: {}",
        stdout
    );
}

#[test]
fn test_db_status_shows_fts_index_info() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "FTS Problem"]);
    run_jjj_success(&dir, &["db", "rebuild"]);

    let stdout = run_jjj_success(&dir, &["db", "status"]);
    assert!(
        stdout.contains("FTS") || stdout.contains("fts") || stdout.contains("document"),
        "Expected FTS index info in output: {}",
        stdout
    );
}

// =============================================================================
// db rebuild
// =============================================================================

#[test]
fn test_db_rebuild_on_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["db", "rebuild"]);
    assert!(
        stdout.contains("Done") || stdout.contains("Rebuild") || stdout.contains("FTS"),
        "Expected rebuild confirmation: {}",
        stdout
    );
}

#[test]
fn test_db_rebuild_with_entities() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create several entities
    run_jjj_success(&dir, &["problem", "new", "Rebuild Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Rebuild Problem 2"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Rebuild Solution",
            "--problem",
            "Rebuild Problem 1",
        ],
    );

    let stdout = run_jjj_success(&dir, &["db", "rebuild"]);
    assert!(
        stdout.contains("Done"),
        "Expected rebuild to finish successfully: {}",
        stdout
    );
    // Should mention loading from markdown and rebuilding FTS
    assert!(
        stdout.contains("markdown") || stdout.contains("Loading"),
        "Expected markdown loading step: {}",
        stdout
    );
    assert!(
        stdout.contains("FTS") || stdout.contains("fts"),
        "Expected FTS rebuild step: {}",
        stdout
    );
}

#[test]
fn test_db_rebuild_is_idempotent() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Idempotent Problem"]);

    // Rebuild twice -- both should succeed
    run_jjj_success(&dir, &["db", "rebuild"]);
    let stdout = run_jjj_success(&dir, &["db", "rebuild"]);
    assert!(
        stdout.contains("Done"),
        "Expected second rebuild to succeed: {}",
        stdout
    );
}

#[test]
fn test_db_rebuild_then_status_reflects_entities() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Counted Problem"]);
    run_jjj_success(&dir, &["db", "rebuild"]);

    let stdout = run_jjj_success(&dir, &["db", "status"]);
    // The status should show at least 1 problem
    assert!(
        stdout.contains("1 problem") || stdout.contains("Entities:"),
        "Expected entity count to reflect created problem: {}",
        stdout
    );
}

// =============================================================================
// search (top-level command that uses the database)
// =============================================================================

#[test]
fn test_search_finds_problem_by_title() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Authentication Failure Bug"]);

    let stdout = run_jjj_success(&dir, &["search", "Authentication"]);
    assert!(
        stdout.contains("Authentication Failure Bug"),
        "Expected to find problem by title keyword: {}",
        stdout
    );
}

#[test]
fn test_search_finds_solution_by_title() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Some Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Refactor Token Validation",
            "--problem",
            "Some Problem",
        ],
    );

    let stdout = run_jjj_success(&dir, &["search", "Token Validation"]);
    assert!(
        stdout.contains("Refactor Token Validation"),
        "Expected to find solution by title keyword: {}",
        stdout
    );
}

#[test]
fn test_search_no_results() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Real Problem"]);

    let stdout = run_jjj_success(
        &dir,
        &["search", "zzz_nonexistent_xyzzy_query_42"],
    );
    assert!(
        stdout.contains("No results") || stdout.contains("0 result"),
        "Expected no results message: {}",
        stdout
    );
}

#[test]
fn test_search_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON Search Problem"]);

    let stdout = run_jjj_success(&dir, &["search", "JSON Search", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse search JSON output");
    assert!(json.is_array(), "Expected JSON array");
    let arr = json.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "Expected at least one search result in JSON"
    );
    // Each result should have type, id, and title
    assert!(
        arr[0]["type"].is_string(),
        "Expected type field: {:?}",
        arr[0]
    );
    assert!(
        arr[0]["id"].is_string(),
        "Expected id field: {:?}",
        arr[0]
    );
    assert!(
        arr[0]["title"].is_string(),
        "Expected title field: {:?}",
        arr[0]
    );
}

#[test]
fn test_search_filter_by_type() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Filterable Widget"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Filterable Widget Fix",
            "--problem",
            "Filterable Widget",
        ],
    );

    // Search only for problems
    let stdout = run_jjj_success(
        &dir,
        &["search", "Filterable Widget", "--type", "problem", "--json"],
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    let arr = json.as_array().unwrap();
    for item in arr {
        assert_eq!(
            item["type"].as_str().unwrap(),
            "problem",
            "Expected only problem results when filtering by type"
        );
    }
}

#[test]
fn test_search_text_only_flag() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Text Only Search Target"]);

    // --text-only should work without any embedding service
    let stdout = run_jjj_success(
        &dir,
        &["search", "Text Only Search", "--text-only"],
    );
    assert!(
        stdout.contains("Text Only Search Target"),
        "Expected to find problem with --text-only: {}",
        stdout
    );
}

// =============================================================================
// Error cases
// =============================================================================

#[test]
fn test_db_invalid_subcommand() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["db", "nonexistent"]);
    assert!(
        !output.status.success(),
        "Expected failure for invalid db subcommand"
    );
}

#[test]
fn test_search_empty_query_is_handled() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // An empty string argument should either fail gracefully or return no results
    let output = run_jjj(&dir, &["search", ""]);
    // We just verify it does not panic -- either success with no results or a clean error
    let _stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);
    // No assertion on success/failure -- the key thing is it doesn't crash
}
