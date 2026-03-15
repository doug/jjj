mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_insights_basic() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create some data to generate events
    run_jjj_success(&dir, &["problem", "new", "Test Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Test Problem",
        ],
    );

    let stdout = run_jjj_success(&dir, &["insights"]);
    assert!(
        stdout.contains("Approval rate") || stdout.contains("approval rate"),
        "Expected approval rate in output: {}",
        stdout
    );
    assert!(
        stdout.contains("Total events") || stdout.contains("total_events"),
        "Expected total events in output: {}",
        stdout
    );
}

#[test]
fn test_insights_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );

    let stdout = run_jjj_success(&dir, &["insights", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    assert!(
        json.get("total_events").is_some(),
        "Expected total_events field: {}",
        stdout
    );
    assert!(
        json.get("approval_rate").is_some(),
        "Expected approval_rate field"
    );
    assert!(
        json.get("top_contributors").is_some(),
        "Expected top_contributors field"
    );
}

#[test]
fn test_insights_with_data() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create problem, solution, approve it
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["solution", "submit", "Solution"]);
    run_jjj_success(&dir, &["solution", "approve", "Solution", "--no-rationale"]);

    let stdout = run_jjj_success(&dir, &["insights", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    let approved = json["solutions_approved"].as_i64().unwrap_or(0);
    assert!(approved >= 1, "Expected at least 1 approved solution");

    // Approval rate should be present and non-null
    assert!(
        json["approval_rate"].is_string(),
        "Expected approval_rate to be a string: {:?}",
        json["approval_rate"]
    );
}

#[test]
fn test_insights_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["insights"]);
    assert!(
        stdout.contains("No events recorded"),
        "Expected 'No events recorded' message: {}",
        stdout
    );
}
