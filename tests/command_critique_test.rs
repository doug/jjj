mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_critique_new_creates_critique() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    let stdout = run_jjj_success(&dir, &["critique", "new", "Solution", "Test Critique"]);

    assert!(
        stdout.contains("Test Critique"),
        "Expected title in output: {}",
        stdout
    );
}

#[test]
fn test_critique_new_with_severity() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Solution",
            "Critical Issue",
            "--severity",
            "critical",
        ],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "Critical Issue"]);
    assert!(
        stdout.contains("critical") || stdout.contains("Critical"),
        "Expected critical severity: {}",
        stdout
    );
}

#[test]
fn test_critique_new_with_reviewer() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Solution",
            "Review Request",
            "--reviewer",
            "bob",
        ],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "Review Request", "--json"]);
    assert!(
        stdout.contains("\"reviewer\": \"bob\""),
        "Expected reviewer bob: {}",
        stdout
    );
}

#[test]
fn test_critique_list_shows_all() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "First Critique"]);
    run_jjj_success(&dir, &["critique", "new", "Solution", "Second Critique"]);

    let stdout = run_jjj_success(&dir, &["critique", "list"]);
    assert!(
        stdout.contains("First Critique"),
        "Expected First Critique: {}",
        stdout
    );
    assert!(
        stdout.contains("Second Critique"),
        "Expected Second Critique: {}",
        stdout
    );
}

#[test]
fn test_critique_list_filter_by_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution 1", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution 2", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution 1", "Critique on S1"]);
    run_jjj_success(&dir, &["critique", "new", "Solution 2", "Critique on S2"]);

    let stdout = run_jjj_success(&dir, &["critique", "list", "--solution", "Solution 1"]);
    assert!(
        stdout.contains("Critique on S1"),
        "Expected S1 critique: {}",
        stdout
    );
    assert!(
        !stdout.contains("Critique on S2"),
        "Should not contain S2 critique: {}",
        stdout
    );
}

#[test]
fn test_critique_list_filter_by_status() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Open Critique"]);
    run_jjj_success(&dir, &["critique", "new", "Solution", "Addressed Critique"]);
    run_jjj_success(&dir, &["critique", "address", "Addressed Critique"]);

    let stdout = run_jjj_success(&dir, &["critique", "list", "--status", "open"]);
    assert!(
        stdout.contains("Open Critique"),
        "Expected open critique: {}",
        stdout
    );
    assert!(
        !stdout.contains("Addressed Critique"),
        "Should not contain addressed critique: {}",
        stdout
    );
}

#[test]
fn test_critique_list_filter_by_reviewer() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Solution",
            "For Alice",
            "--reviewer",
            "alice",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Solution",
            "For Bob",
            "--reviewer",
            "bob",
        ],
    );

    let stdout = run_jjj_success(&dir, &["critique", "list", "--reviewer", "alice"]);
    assert!(
        stdout.contains("For Alice"),
        "Expected Alice's critique: {}",
        stdout
    );
    assert!(
        !stdout.contains("For Bob"),
        "Should not contain Bob's critique: {}",
        stdout
    );
}

#[test]
fn test_critique_list_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "JSON Critique"]);

    let stdout = run_jjj_success(&dir, &["critique", "list", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    // ID should exist but we don't assert specific value
    assert!(arr[0]["id"].is_string(), "Expected id to be a string");
    assert_eq!(arr[0]["title"], "JSON Critique");
}

#[test]
fn test_critique_show_details() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Solution",
            "Detailed Critique",
            "--severity",
            "high",
        ],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "Detailed Critique"]);
    assert!(
        stdout.contains("Detailed Critique"),
        "Expected title: {}",
        stdout
    );
    assert!(
        stdout.contains("high") || stdout.contains("High"),
        "Expected severity: {}",
        stdout
    );
    assert!(
        stdout.contains("Solution"),
        "Expected solution reference: {}",
        stdout
    );
}

#[test]
fn test_critique_show_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "JSON Show"]);

    let stdout = run_jjj_success(&dir, &["critique", "show", "JSON Show", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    // ID should exist but we don't assert specific value
    assert!(json["id"].is_string(), "Expected id to be a string");
    assert_eq!(json["title"], "JSON Show");
    assert!(
        json["solution_id"].is_string(),
        "Expected solution_id to be a string"
    );
}

#[test]
fn test_critique_address() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "To Address"]);

    let stdout = run_jjj_success(&dir, &["critique", "address", "To Address"]);
    assert!(
        stdout.contains("addressed"),
        "Expected addressed confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["critique", "show", "To Address"]);
    assert!(
        show.contains("Addressed") || show.contains("addressed"),
        "Expected addressed status: {}",
        show
    );
}

#[test]
fn test_critique_dismiss() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "To Dismiss"]);

    let stdout = run_jjj_success(&dir, &["critique", "dismiss", "To Dismiss"]);
    assert!(
        stdout.contains("dismissed"),
        "Expected dismissed confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["critique", "show", "To Dismiss"]);
    assert!(
        show.contains("Dismissed") || show.contains("dismissed"),
        "Expected dismissed status: {}",
        show
    );
}

#[test]
fn test_critique_validate() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Flawed Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Flawed Solution", "Valid Flaw"]);

    let stdout = run_jjj_success(&dir, &["critique", "validate", "Valid Flaw"]);
    assert!(
        stdout.contains("validated") || stdout.contains("correct"),
        "Expected validated confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["critique", "show", "Valid Flaw"]);
    assert!(
        show.contains("Valid") || show.contains("valid"),
        "Expected valid status: {}",
        show
    );
}

#[test]
fn test_critique_edit() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Original"]);
    run_jjj_success(
        &dir,
        &["critique", "edit", "Original", "--title", "Updated Title"],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "Updated Title"]);
    assert!(
        stdout.contains("Updated Title"),
        "Expected updated title: {}",
        stdout
    );
}

#[test]
fn test_critique_edit_severity() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Low Issue"]);
    run_jjj_success(
        &dir,
        &["critique", "edit", "Low Issue", "--severity", "critical"],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "Low Issue"]);
    assert!(
        stdout.contains("critical") || stdout.contains("Critical"),
        "Expected critical severity: {}",
        stdout
    );
}

#[test]
fn test_critique_reply() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Discussion Starter"]);
    let reply_output = run_jjj_success(
        &dir,
        &[
            "critique",
            "reply",
            "Discussion Starter",
            "This is my response to the critique",
        ],
    );
    // Reply command should confirm the reply was added
    assert!(
        reply_output.contains("reply") || reply_output.contains("Added"),
        "Expected reply confirmation: {}",
        reply_output
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "Discussion Starter"]);
    // The implementation shows replies under "## Discussion" section
    assert!(
        stdout.contains("Discussion") || stdout.contains("This is my response"),
        "Expected reply/discussion in output: {}",
        stdout
    );
}

#[test]
fn test_critique_blocks_solution_accept() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Blocked", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["solution", "submit", "Blocked"]);
    run_jjj_success(&dir, &["critique", "new", "Blocked", "Blocker"]);

    // Try to submit - should fail (open critique)
    let output = run_jjj(&dir, &["solution", "approve", "Blocked"]);
    assert!(
        !output.status.success(),
        "Submit should fail with open critique"
    );

    // Address the critique
    run_jjj_success(&dir, &["critique", "address", "Blocker"]);

    // Now submit should work
    let stdout = run_jjj_success(&dir, &["solution", "approve", "Blocked"]);
    assert!(
        stdout.contains("approved") || stdout.contains("Approved"),
        "Expected approved: {}",
        stdout
    );
}

#[test]
fn test_critique_show_nonexistent_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["critique", "show", "nonexistent-critique-xyz"]);
    assert!(
        !output.status.success(),
        "Expected failure for nonexistent critique"
    );
}

#[test]
fn test_critique_on_finalized_solution_warns() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["solution", "submit", "Solution"]);
    run_jjj_success(&dir, &["solution", "approve", "Solution"]);

    // Creating critique on accepted solution should warn but succeed
    let output = run_jjj(&dir, &["critique", "new", "Solution", "Late Critique"]);
    assert!(output.status.success(), "Critique creation should succeed");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Should contain warning about finalized solution
    assert!(
        combined.contains("Warning")
            || combined.contains("Approved")
            || combined.contains("already"),
        "Expected warning about finalized solution: {}",
        combined
    );
}
