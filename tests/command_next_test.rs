mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_next_basic() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);

    let stdout = run_jjj_success(&dir, &["next"]);
    assert!(
        stdout.contains("TODO") || stdout.contains("Problem"),
        "Expected next action: {}",
        stdout
    );
}

#[test]
fn test_next_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next"]);
    assert!(
        stdout.contains("Nothing to do") || stdout.contains("all caught up"),
        "Expected empty message: {}",
        stdout
    );
}

#[test]
fn test_next_claim() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Claimable Problem"]);

    let stdout = run_jjj_success(&dir, &["next", "--claim"]);
    assert!(
        stdout.contains("Claimed"),
        "Expected 'Claimed' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("Claimable Problem"),
        "Expected problem title in output: {}",
        stdout
    );

    // Verify assignment by checking the problem
    let show_stdout = run_jjj_success(&dir, &["problem", "show", "Claimable", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&show_stdout).expect("Failed to parse JSON");
    assert!(
        json["assignee"].is_string(),
        "Expected assignee to be set after claim: {:?}",
        json["assignee"]
    );
}

#[test]
fn test_next_claim_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON Claim"]);

    let stdout = run_jjj_success(&dir, &["next", "--claim", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(
        json["title"].as_str().unwrap_or("").contains("JSON Claim"),
        "Expected title in JSON output: {}",
        stdout
    );
}

#[test]
fn test_next_claim_empty() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next", "--claim"]);
    assert!(
        stdout.contains("Nothing to claim"),
        "Expected 'Nothing to claim' on empty repo: {}",
        stdout
    );
}

#[test]
fn test_next_claim_empty_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next", "--claim", "--json"]);
    assert!(
        stdout.trim() == "null",
        "Expected null JSON on empty repo: {}",
        stdout
    );
}
