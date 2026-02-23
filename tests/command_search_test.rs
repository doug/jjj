mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_search_problem_by_title_text() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create a few problems with distinct titles
    run_jjj_success(&dir, &["problem", "new", "Authentication timeout bug"]);
    run_jjj_success(&dir, &["problem", "new", "Database connection pooling"]);
    run_jjj_success(&dir, &["problem", "new", "Frontend rendering glitch"]);

    // Search for a problem by partial title
    let stdout = run_jjj_success(&dir, &["search", "Authentication", "--text-only"]);
    assert!(
        stdout.contains("Authentication timeout bug"),
        "Expected to find 'Authentication timeout bug' in search results: {}",
        stdout
    );
}

#[test]
fn test_search_with_type_filter() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create a problem and a solution
    run_jjj_success(&dir, &["problem", "new", "Widget alignment issue"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Fix widget CSS",
            "--problem",
            "Widget alignment issue",
        ],
    );

    // Search with --type problem should only return problems
    let stdout = run_jjj_success(
        &dir,
        &["search", "widget", "--type", "problem", "--text-only"],
    );
    assert!(
        stdout.contains("Widget alignment issue"),
        "Expected problem in type-filtered results: {}",
        stdout
    );

    // Search with --type solution should only return solutions
    let stdout = run_jjj_success(
        &dir,
        &["search", "widget", "--type", "solution", "--text-only"],
    );
    assert!(
        stdout.contains("Fix widget CSS"),
        "Expected solution in type-filtered results: {}",
        stdout
    );
}

#[test]
fn test_search_no_results() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Some real problem"]);

    // Search for something that does not exist
    let stdout = run_jjj_success(
        &dir,
        &["search", "xyzzyzzynonexistent", "--text-only"],
    );
    assert!(
        stdout.contains("No results found"),
        "Expected 'No results found' message: {}",
        stdout
    );
}

#[test]
fn test_search_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON searchable problem"]);

    let stdout = run_jjj_success(
        &dir,
        &["search", "JSON searchable", "--json", "--text-only"],
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse search JSON output");
    assert!(json.is_array(), "Expected JSON array, got: {}", json);

    let arr = json.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "Expected at least one result in JSON output"
    );

    // Verify the result has expected fields
    let first = &arr[0];
    assert!(
        first.get("type").is_some(),
        "Expected 'type' field in result: {}",
        first
    );
    assert!(
        first.get("id").is_some(),
        "Expected 'id' field in result: {}",
        first
    );
    assert!(
        first.get("title").is_some(),
        "Expected 'title' field in result: {}",
        first
    );
    assert_eq!(
        first["title"], "JSON searchable problem",
        "Expected matching title in JSON result"
    );
}
