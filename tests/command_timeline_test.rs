mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

/// Extract the problem UUID from `jjj problem new` output.
/// Output format: "Created problem <uuid> (<title>)"
fn extract_problem_id(output: &str) -> String {
    // Find the line containing "Created problem"
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("Created problem ") {
            // The UUID is the next token (up to the space before the parenthesized title)
            if let Some(id) = rest.split_whitespace().next() {
                return id.to_string();
            }
        }
    }
    panic!("Could not extract problem ID from output: {}", output);
}

/// Extract the solution UUID from `jjj solution new` output.
/// Output format: "Created solution <uuid> (<title>)"
fn extract_solution_id(output: &str) -> String {
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("Created solution ") {
            if let Some(id) = rest.split_whitespace().next() {
                return id.to_string();
            }
        }
    }
    panic!("Could not extract solution ID from output: {}", output);
}

#[test]
fn test_timeline_shows_creation_event() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let create_output = run_jjj_success(&dir, &["problem", "new", "Timeline Test Problem"]);
    let problem_id = extract_problem_id(&create_output);

    let stdout = run_jjj_success(&dir, &["timeline", &problem_id]);
    assert!(
        stdout.contains("Timeline Test Problem"),
        "Expected problem title in timeline output: {}",
        stdout
    );
    assert!(
        stdout.contains("problem created"),
        "Expected 'problem created' event in timeline: {}",
        stdout
    );
}

#[test]
fn test_timeline_with_solutions() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let create_output = run_jjj_success(&dir, &["problem", "new", "Problem With Solutions"]);
    let problem_id = extract_problem_id(&create_output);

    // Create a solution attached to the problem
    let sol_output = run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "First Solution",
            "--problem",
            "Problem With Solutions",
        ],
    );
    let solution_id = extract_solution_id(&sol_output);

    let stdout = run_jjj_success(&dir, &["timeline", &problem_id]);
    assert!(
        stdout.contains("Problem With Solutions"),
        "Expected problem title in timeline output: {}",
        stdout
    );
    assert!(
        stdout.contains("problem created"),
        "Expected 'problem created' event: {}",
        stdout
    );
    // The solution event should reference the solution's entity ID
    assert!(
        stdout.contains("proposed") || stdout.contains(&solution_id[..6.min(solution_id.len())]),
        "Expected solution proposed event in timeline: {}",
        stdout
    );
}

#[test]
fn test_timeline_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let create_output = run_jjj_success(&dir, &["problem", "new", "JSON Timeline Problem"]);
    let problem_id = extract_problem_id(&create_output);

    // Create a solution so there are multiple events
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "JSON Timeline Solution",
            "--problem",
            "JSON Timeline Problem",
        ],
    );

    let stdout = run_jjj_success(&dir, &["timeline", &problem_id, "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse timeline JSON output");
    assert!(json.is_array(), "Expected JSON array, got: {}", json);

    let arr = json.as_array().unwrap();
    assert!(
        arr.len() >= 2,
        "Expected at least 2 events (problem_created + solution_created), got {}",
        arr.len()
    );

    // Verify event structure
    for event in arr {
        assert!(
            event.get("when").is_some(),
            "Event missing 'when' field: {}",
            event
        );
        assert!(
            event.get("type").is_some(),
            "Event missing 'type' field: {}",
            event
        );
        assert!(
            event.get("entity").is_some(),
            "Event missing 'entity' field: {}",
            event
        );
        assert!(
            event.get("by").is_some(),
            "Event missing 'by' field: {}",
            event
        );
    }

    // First event should be problem_created
    let first_type = arr[0]["type"].as_str().unwrap_or("");
    assert_eq!(
        first_type, "problem_created",
        "Expected first event to be problem_created, got: {}",
        first_type
    );
}
