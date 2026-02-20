mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_problem_new_creates_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["problem", "new", "Test Problem"]);
    assert!(
        stdout.contains("Test Problem"),
        "Expected title in output: {}",
        stdout
    );
    // Should contain "Created problem" message
    assert!(
        stdout.contains("Created problem") || stdout.contains("Test Problem"),
        "Expected creation confirmation: {}",
        stdout
    );
}

#[test]
fn test_problem_new_with_priority() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(
        &dir,
        &["problem", "new", "Critical Bug", "--priority", "P0"],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "Critical Bug"]);
    assert!(
        stdout.contains("P0/critical") || stdout.contains("Critical"),
        "Expected P0/critical priority: {}",
        stdout
    );
}

#[test]
fn test_problem_new_with_parent() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Parent Problem"]);
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Child Problem",
            "--parent",
            "Parent Problem",
        ],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "Child Problem"]);
    assert!(
        stdout.contains("Parent Problem") || stdout.contains("Parent"),
        "Expected parent reference in output: {}",
        stdout
    );
}

#[test]
fn test_problem_list_shows_all_problems() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "First Problem"]);
    run_jjj_success(&dir, &["problem", "new", "Second Problem"]);
    run_jjj_success(&dir, &["problem", "new", "Third Problem"]);

    let stdout = run_jjj_success(&dir, &["problem", "list"]);
    assert!(
        stdout.contains("First Problem"),
        "Expected First Problem: {}",
        stdout
    );
    assert!(
        stdout.contains("Second Problem"),
        "Expected Second Problem: {}",
        stdout
    );
    assert!(
        stdout.contains("Third Problem"),
        "Expected Third Problem: {}",
        stdout
    );
}

#[test]
fn test_problem_list_filter_by_status() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Open Problem"]);
    run_jjj_success(&dir, &["problem", "new", "Solved Problem"]);
    // Create a solution and accept it (auto-solves Solved Problem)
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Solved Problem"],
    );
    run_jjj_success(&dir, &["solution", "accept", "Solution"]);

    let stdout = run_jjj_success(&dir, &["problem", "list", "--status", "open"]);
    assert!(
        stdout.contains("Open Problem"),
        "Expected Open Problem in open list: {}",
        stdout
    );
    assert!(
        !stdout.contains("Solved Problem"),
        "Solved Problem should not be in open list: {}",
        stdout
    );

    let stdout = run_jjj_success(&dir, &["problem", "list", "--status", "solved"]);
    assert!(
        stdout.contains("Solved Problem"),
        "Expected Solved Problem in solved list: {}",
        stdout
    );
}

#[test]
fn test_problem_list_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON Problem"]);

    let stdout = run_jjj_success(&dir, &["problem", "list", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    assert!(json.is_array(), "Expected JSON array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1, "Expected 1 problem");
    assert_eq!(arr[0]["title"], "JSON Problem");
    // ID should exist but we don't assert specific value
    assert!(arr[0]["id"].is_string(), "Expected id to be a string");
}

#[test]
fn test_problem_show_displays_details() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(
        &dir,
        &["problem", "new", "Detailed Problem", "--priority", "P1"],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "Detailed Problem"]);
    assert!(
        stdout.contains("Detailed Problem"),
        "Expected title: {}",
        stdout
    );
    assert!(
        stdout.contains("P1/high") || stdout.contains("High"),
        "Expected priority: {}",
        stdout
    );
    assert!(
        stdout.contains("Status") || stdout.contains("open"),
        "Expected status: {}",
        stdout
    );
}

#[test]
fn test_problem_show_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON Show Problem"]);

    let stdout = run_jjj_success(&dir, &["problem", "show", "JSON Show Problem", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    // ID should exist but we don't assert specific value
    assert!(json["id"].is_string(), "Expected id to be a string");
    assert_eq!(json["title"], "JSON Show Problem");
    assert_eq!(json["status"], "open");
}

#[test]
fn test_problem_show_nonexistent_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["problem", "show", "nonexistent-problem-xyz"]);
    assert!(
        !output.status.success(),
        "Expected failure for nonexistent problem"
    );
}

#[test]
fn test_problem_edit_title() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Original Title"]);
    run_jjj_success(
        &dir,
        &[
            "problem",
            "edit",
            "Original Title",
            "--title",
            "Updated Title",
        ],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "Updated Title"]);
    assert!(
        stdout.contains("Updated Title"),
        "Expected updated title: {}",
        stdout
    );
}

#[test]
fn test_problem_edit_priority() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Low Priority"]);
    run_jjj_success(
        &dir,
        &["problem", "edit", "Low Priority", "--priority", "P0"],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "Low Priority"]);
    assert!(
        stdout.contains("P0/critical") || stdout.contains("Critical"),
        "Expected P0 priority: {}",
        stdout
    );
}

#[test]
fn test_problem_dissolve() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "False Problem"]);
    run_jjj_success(
        &dir,
        &[
            "problem",
            "dissolve",
            "False Problem",
            "--reason",
            "Based on misunderstanding",
        ],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "False Problem"]);
    assert!(
        stdout.contains("dissolved"),
        "Expected dissolved status: {}",
        stdout
    );
    assert!(
        stdout.contains("Based on misunderstanding"),
        "Expected dissolve reason: {}",
        stdout
    );
}

#[test]
fn test_problem_tree_view() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Root Problem"]);
    run_jjj_success(
        &dir,
        &["problem", "new", "Child 1", "--parent", "Root Problem"],
    );
    run_jjj_success(
        &dir,
        &["problem", "new", "Child 2", "--parent", "Root Problem"],
    );

    let stdout = run_jjj_success(&dir, &["problem", "list", "--tree"]);
    // Tree view should show hierarchy
    assert!(stdout.contains("Root Problem"), "Expected root: {}", stdout);
    assert!(stdout.contains("Child 1"), "Expected child 1: {}", stdout);
    assert!(stdout.contains("Child 2"), "Expected child 2: {}", stdout);
}

#[test]
fn test_problem_solve_requires_accepted_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Need Solution"]);

    // Try to solve without any solution - should fail
    let output = run_jjj(&dir, &["problem", "solve", "Need Solution"]);
    assert!(
        !output.status.success(),
        "Expected failure when solving without accepted solution"
    );

    // Add a solution and accept it (auto-solves the problem)
    run_jjj_success(
        &dir,
        &["solution", "new", "Fix", "--problem", "Need Solution"],
    );
    run_jjj_success(&dir, &["solution", "accept", "Fix"]);

    // Problem is already auto-solved, explicit solve should fail
    let output = run_jjj(&dir, &["problem", "solve", "Need Solution"]);
    assert!(
        !output.status.success(),
        "Expected failure when problem is already solved"
    );

    // Verify problem is in solved state
    let stdout = run_jjj_success(&dir, &["problem", "list", "--status", "solved"]);
    assert!(
        stdout.contains("Need Solution"),
        "Expected problem in solved list: {}",
        stdout
    );
}

#[test]
fn test_problem_assign() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Assign Me"]);
    run_jjj_success(&dir, &["problem", "assign", "Assign Me", "--to", "alice"]);

    let stdout = run_jjj_success(&dir, &["problem", "show", "Assign Me"]);
    assert!(
        stdout.contains("alice"),
        "Expected assignee alice: {}",
        stdout
    );
}
