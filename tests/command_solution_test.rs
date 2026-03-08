mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_solution_new_creates_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Test Problem"]);
    let stdout = run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Test Problem",
        ],
    );

    assert!(
        stdout.contains("Test Solution"),
        "Expected title in output: {}",
        stdout
    );
}

#[test]
fn test_solution_new_links_to_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Linked Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Linked Solution",
            "--problem",
            "Linked Problem",
        ],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Linked Solution"]);
    // Output shows "Addresses:" field with the problem UUID
    assert!(
        stdout.contains("Linked Problem") || stdout.contains("Addresses:"),
        "Expected problem link in solution: {}",
        stdout
    );
}

#[test]
fn test_solution_new_auto_attaches_change() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Solution"]);
    // Solution new auto-attaches but stays Proposed; solution review advances to Review
    assert!(
        stdout.contains("Changes") || stdout.contains("Review") || stdout.contains("review"),
        "Expected change attached: {}",
        stdout
    );
}

#[test]
fn test_solution_list_shows_all() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 2"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution 1", "--problem", "Problem 1"],
    );
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution 2", "--problem", "Problem 2"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "list"]);
    assert!(
        stdout.contains("Solution 1"),
        "Expected Solution 1: {}",
        stdout
    );
    assert!(
        stdout.contains("Solution 2"),
        "Expected Solution 2: {}",
        stdout
    );
}

#[test]
fn test_solution_list_filter_by_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 2"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Solution for P1",
            "--problem",
            "Problem 1",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Solution for P2",
            "--problem",
            "Problem 2",
        ],
    );

    let stdout = run_jjj_success(&dir, &["solution", "list", "--problem", "Problem 1"]);
    assert!(
        stdout.contains("Solution for P1"),
        "Expected P1 solution: {}",
        stdout
    );
    assert!(
        !stdout.contains("Solution for P2"),
        "Should not contain P2 solution: {}",
        stdout
    );
}

#[test]
fn test_solution_list_filter_by_status() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Testing Solution",
            "--problem",
            "Problem",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Accepted Solution",
            "--problem",
            "Problem",
        ],
    );
    run_jjj_success(&dir, &["solution", "submit", "Accepted Solution"]);
    run_jjj_success(&dir, &["solution", "approve", "Accepted Solution"]);

    let stdout = run_jjj_success(&dir, &["solution", "list", "--status", "approved"]);
    assert!(
        stdout.contains("Accepted Solution")
            || stdout.contains("Approved Solution")
            || stdout.contains("Accepted Solution"),
        "Expected accepted solution: {}",
        stdout
    );
    assert!(
        !stdout.contains("Testing Solution"),
        "Should not contain testing solution: {}",
        stdout
    );
}

#[test]
fn test_solution_list_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "JSON Solution", "--problem", "Problem"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "list", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    // ID should exist but we don't assert specific value
    assert!(arr[0]["id"].is_string(), "Expected id to be a string");
    assert_eq!(arr[0]["title"], "JSON Solution");
}

#[test]
fn test_solution_show_details() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Detailed Solution",
            "--problem",
            "Problem",
        ],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Detailed Solution"]);
    assert!(
        stdout.contains("Detailed Solution"),
        "Expected title: {}",
        stdout
    );
    // Output shows "Addresses:" field with the problem UUID
    assert!(
        stdout.contains("Addresses:") || stdout.contains("Problem"),
        "Expected problem reference: {}",
        stdout
    );
    assert!(
        stdout.contains("Status") || stdout.contains("testing") || stdout.contains("Testing"),
        "Expected status: {}",
        stdout
    );
}

#[test]
fn test_solution_show_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "JSON Show", "--problem", "Problem"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "JSON Show", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    // ID should exist but we don't assert specific value
    assert!(json["id"].is_string(), "Expected id to be a string");
    assert_eq!(json["title"], "JSON Show");
    assert!(
        json["problem_id"].is_string(),
        "Expected problem_id to be a string"
    );
}

#[test]
fn test_solution_submit() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Submittable", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["solution", "submit", "Submittable"]);

    let stdout = run_jjj_success(&dir, &["solution", "approve", "Submittable"]);
    assert!(
        stdout.contains("approved"),
        "Expected accepted confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["solution", "show", "Submittable"]);
    assert!(
        show.contains("Approved") || show.contains("approved"),
        "Expected accepted status: {}",
        show
    );
}

#[test]
fn test_solution_submit_blocked_by_critiques() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Blocked Solution",
            "--problem",
            "Problem",
        ],
    );
    run_jjj_success(&dir, &["solution", "submit", "Blocked Solution"]);
    run_jjj_success(&dir, &["critique", "new", "Blocked Solution", "Major flaw"]);

    let output = run_jjj(&dir, &["solution", "approve", "Blocked Solution"]);
    assert!(
        !output.status.success(),
        "Submit should fail with open critiques"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("critique") || stderr.contains("open"),
        "Expected critique blocking message: {}",
        stderr
    );
}

#[test]
fn test_solution_submit_force() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Force Submit", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["solution", "submit", "Force Submit"]);
    run_jjj_success(&dir, &["critique", "new", "Force Submit", "Minor issue"]);

    let stdout = run_jjj_success(&dir, &["solution", "approve", "Force Submit", "--force"]);
    assert!(
        stdout.contains("approved"),
        "Expected force-accepted confirmation: {}",
        stdout
    );
}

#[test]
fn test_solution_refute() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Bad Solution", "--problem", "Problem"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "withdraw", "Bad Solution"]);
    assert!(
        stdout.contains("withdrawn"),
        "Expected withdrawn confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["solution", "show", "Bad Solution"]);
    assert!(
        show.contains("Withdrawn") || show.contains("withdrawn"),
        "Expected withdrawn status: {}",
        show
    );
}

#[test]
fn test_solution_supersedes() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Original Solution",
            "--problem",
            "Problem",
        ],
    );
    run_jjj_success(&dir, &["solution", "withdraw", "Original Solution"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Better Solution",
            "--problem",
            "Problem",
            "--supersedes",
            "Original Solution",
        ],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Better Solution"]);
    assert!(
        stdout.contains("Supersedes") || stdout.contains("Original Solution"),
        "Expected supersedes reference: {}",
        stdout
    );
}

#[test]
fn test_solution_test_status() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Test Status", "--problem", "Problem"],
    );

    // solution new auto-attaches but stays in Proposed; explicit review advances to Review
    let show = run_jjj_success(&dir, &["solution", "show", "Test Status"]);
    assert!(
        show.contains("Proposed") || show.contains("proposed"),
        "Expected proposed status after creation: {}",
        show
    );

    run_jjj_success(&dir, &["solution", "submit", "Test Status"]);
    let show = run_jjj_success(&dir, &["solution", "show", "Test Status"]);
    assert!(
        show.contains("Submitted") || show.contains("submitted"),
        "Expected submitted status after solution submit: {}",
        show
    );
}

#[test]
fn test_solution_assign() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Assigned", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["solution", "assign", "Assigned", "--to", "bob"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "Assigned"]);
    assert!(stdout.contains("bob"), "Expected assignee bob: {}", stdout);
}

#[test]
fn test_solution_edit() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Original", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &["solution", "edit", "Original", "--title", "Updated Title"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Updated Title"]);
    assert!(
        stdout.contains("Updated Title"),
        "Expected updated title: {}",
        stdout
    );
}

#[test]
fn test_solution_with_reviewer() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Needs Review",
            "--problem",
            "Problem",
            "--reviewer",
            "alice",
        ],
    );

    // Should create an awaiting review critique
    let stdout = run_jjj_success(&dir, &["critique", "list", "--json"]);
    assert!(
        stdout.contains("Awaiting review from @alice"),
        "Expected awaiting review critique: {}",
        stdout
    );
}

#[test]
fn test_solution_show_nonexistent_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["solution", "show", "nonexistent-solution-xyz"]);
    assert!(
        !output.status.success(),
        "Expected failure for nonexistent solution"
    );
}

#[test]
fn test_solution_new_with_tags() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Tagged Solution",
            "--problem",
            "Problem",
            "--tags",
            "refactor,backend",
        ],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Tagged Solution"]);
    assert!(
        stdout.contains("Tags:") && stdout.contains("backend") && stdout.contains("refactor"),
        "Expected tags in show output: {}",
        stdout
    );
}

#[test]
fn test_solution_edit_add_remove_tag() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Tag Edit Sol", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &["solution", "edit", "Tag Edit Sol", "--add-tag", "refactor"],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "Tag Edit Sol"]);
    assert!(
        stdout.contains("refactor"),
        "Expected tag refactor in output: {}",
        stdout
    );

    // Remove the tag
    run_jjj_success(
        &dir,
        &[
            "solution",
            "edit",
            "Tag Edit Sol",
            "--remove-tag",
            "refactor",
        ],
    );
    let stdout = run_jjj_success(&dir, &["solution", "show", "Tag Edit Sol"]);
    assert!(
        !stdout.contains("Tags:"),
        "Expected no tags after removal: {}",
        stdout
    );
}
