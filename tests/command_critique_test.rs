mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_critique_new_creates_critique() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    let stdout = run_jjj_success(&dir, &["critique", "new", "s1", "Test Critique"]);

    assert!(stdout.contains("c1"), "Expected c1 in output: {}", stdout);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(
        &dir,
        &["critique", "new", "s1", "Critical Issue", "--severity", "critical"],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(
        &dir,
        &["critique", "new", "s1", "Review Request", "--reviewer", "bob"],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1", "--json"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "First Critique"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Second Critique"]);

    let stdout = run_jjj_success(&dir, &["critique", "list"]);
    assert!(stdout.contains("c1"), "Expected c1: {}", stdout);
    assert!(stdout.contains("c2"), "Expected c2: {}", stdout);
}

#[test]
fn test_critique_list_filter_by_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Solution 1", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "new", "Solution 2", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Critique on S1"]);
    run_jjj_success(&dir, &["critique", "new", "s2", "Critique on S2"]);

    let stdout = run_jjj_success(&dir, &["critique", "list", "--solution", "s1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Open Critique"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Addressed Critique"]);
    run_jjj_success(&dir, &["critique", "address", "c2"]);

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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(
        &dir,
        &["critique", "new", "s1", "For Alice", "--reviewer", "alice"],
    );
    run_jjj_success(
        &dir,
        &["critique", "new", "s1", "For Bob", "--reviewer", "bob"],
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "JSON Critique"]);

    let stdout = run_jjj_success(&dir, &["critique", "list", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "c1");
    assert_eq!(arr[0]["title"], "JSON Critique");
}

#[test]
fn test_critique_show_details() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(
        &dir,
        &["critique", "new", "s1", "Detailed Critique", "--severity", "high"],
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    assert!(stdout.contains("s1"), "Expected solution id: {}", stdout);
}

#[test]
fn test_critique_show_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "JSON Show"]);

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert_eq!(json["id"], "c1");
    assert_eq!(json["title"], "JSON Show");
    assert_eq!(json["solution_id"], "s1");
}

#[test]
fn test_critique_address() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "To Address"]);

    let stdout = run_jjj_success(&dir, &["critique", "address", "c1"]);
    assert!(
        stdout.contains("addressed"),
        "Expected addressed confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "To Dismiss"]);

    let stdout = run_jjj_success(&dir, &["critique", "dismiss", "c1"]);
    assert!(
        stdout.contains("dismissed"),
        "Expected dismissed confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Flawed Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Valid Flaw"]);

    let stdout = run_jjj_success(&dir, &["critique", "validate", "c1"]);
    assert!(
        stdout.contains("validated") || stdout.contains("correct"),
        "Expected validated confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Original"]);
    run_jjj_success(&dir, &["critique", "edit", "c1", "--title", "Updated Title"]);

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Low Issue"]);
    run_jjj_success(&dir, &["critique", "edit", "c1", "--severity", "critical"]);

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Discussion Starter"]);
    let reply_output = run_jjj_success(
        &dir,
        &["critique", "reply", "c1", "This is my response to the critique"],
    );
    // Reply command should confirm the reply was added
    assert!(
        reply_output.contains("reply") || reply_output.contains("Added"),
        "Expected reply confirmation: {}",
        reply_output
    );

    let stdout = run_jjj_success(&dir, &["critique", "show", "c1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Blocked", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Blocker"]);

    // Try to accept - should fail
    let output = run_jjj(&dir, &["solution", "accept", "s1"]);
    assert!(
        !output.status.success(),
        "Accept should fail with open critique"
    );

    // Address the critique
    run_jjj_success(&dir, &["critique", "address", "c1"]);

    // Now accept should work
    let stdout = run_jjj_success(&dir, &["solution", "accept", "s1"]);
    assert!(
        stdout.contains("accepted"),
        "Expected accepted: {}",
        stdout
    );
}

#[test]
fn test_critique_show_nonexistent_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["critique", "show", "c999"]);
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "accept", "s1"]);

    // Creating critique on accepted solution should warn but succeed
    let output = run_jjj(&dir, &["critique", "new", "s1", "Late Critique"]);
    assert!(output.status.success(), "Critique creation should succeed");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Should contain warning about finalized solution
    assert!(
        combined.contains("Warning") || combined.contains("Accepted") || combined.contains("already"),
        "Expected warning about finalized solution: {}",
        combined
    );
}
