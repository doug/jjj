mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_tags_no_tags() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["tags"]);
    assert!(
        stdout.contains("No tags in use"),
        "Expected 'No tags in use' message: {}",
        stdout
    );
}

#[test]
fn test_tags_with_tagged_entities() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "P1", "--tags", "backend,auth"]);
    run_jjj_success(&dir, &["problem", "new", "P2", "--tags", "backend"]);

    let stdout = run_jjj_success(&dir, &["tags"]);
    assert!(
        stdout.contains("backend") && stdout.contains("2"),
        "Expected backend with count 2: {}",
        stdout
    );
    assert!(
        stdout.contains("auth") && stdout.contains("1"),
        "Expected auth with count 1: {}",
        stdout
    );
}

#[test]
fn test_tags_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "P1", "--tags", "backend,auth"]);

    let stdout = run_jjj_success(&dir, &["tags", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    assert!(json.is_array(), "Expected JSON array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2, "Expected 2 tags");
    // Tags sorted by count desc, then alphabetically
    assert!(
        arr.iter().any(|t| t["tag"] == "auth"),
        "Expected auth tag in JSON"
    );
    assert!(
        arr.iter().any(|t| t["tag"] == "backend"),
        "Expected backend tag in JSON"
    );
}
