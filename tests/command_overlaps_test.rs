mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_overlaps_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["overlaps"]);
    assert!(
        stdout.contains("No file overlaps"),
        "Expected 'No file overlaps' message: {}",
        stdout
    );
}

#[test]
fn test_overlaps_json_empty() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["overlaps", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array(), "Expected JSON array");
    assert!(
        json.as_array().unwrap().is_empty(),
        "Expected empty array: {}",
        stdout
    );
}
