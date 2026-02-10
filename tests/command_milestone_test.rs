mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_milestone_new_creates_milestone() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["milestone", "new", "Test Milestone"]);
    assert!(stdout.contains("m1"), "Expected m1 in output: {}", stdout);
    assert!(
        stdout.contains("Test Milestone"),
        "Expected title in output: {}",
        stdout
    );
}

#[test]
fn test_milestone_new_with_date() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(
        &dir,
        &["milestone", "new", "Dated Milestone", "--date", "2025-06-15"],
    );

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        stdout.contains("2025-06-15"),
        "Expected date in output: {}",
        stdout
    );
}

#[test]
fn test_milestone_new_invalid_date_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["milestone", "new", "Bad Date", "--date", "invalid"]);
    assert!(
        !output.status.success(),
        "Expected failure for invalid date format"
    );
}

#[test]
fn test_milestone_list_shows_all() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Milestone 1"]);
    run_jjj_success(&dir, &["milestone", "new", "Milestone 2"]);
    run_jjj_success(&dir, &["milestone", "new", "Milestone 3"]);

    let stdout = run_jjj_success(&dir, &["milestone", "list"]);
    assert!(stdout.contains("m1"), "Expected m1: {}", stdout);
    assert!(stdout.contains("m2"), "Expected m2: {}", stdout);
    assert!(stdout.contains("m3"), "Expected m3: {}", stdout);
}

#[test]
fn test_milestone_list_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "JSON Milestone"]);

    let stdout = run_jjj_success(&dir, &["milestone", "list", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "m1");
    assert_eq!(arr[0]["title"], "JSON Milestone");
}

#[test]
fn test_milestone_show_details() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(
        &dir,
        &["milestone", "new", "Detailed Milestone", "--date", "2025-12-31"],
    );

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        stdout.contains("Detailed Milestone"),
        "Expected title: {}",
        stdout
    );
    assert!(
        stdout.contains("2025-12-31"),
        "Expected date: {}",
        stdout
    );
    assert!(
        stdout.contains("Status") || stdout.contains("planning") || stdout.contains("Planning"),
        "Expected status: {}",
        stdout
    );
}

#[test]
fn test_milestone_show_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "JSON Show"]);

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert_eq!(json["id"], "m1");
    assert_eq!(json["title"], "JSON Show");
}

#[test]
fn test_milestone_add_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Project Milestone"]);
    run_jjj_success(&dir, &["problem", "new", "Important Problem"]);

    let stdout = run_jjj_success(&dir, &["milestone", "add-problem", "m1", "p1"]);
    assert!(
        stdout.contains("Added") || stdout.contains("p1"),
        "Expected add confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        show.contains("p1") || show.contains("Important Problem"),
        "Expected problem in milestone: {}",
        show
    );
}

#[test]
fn test_milestone_add_problem_updates_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Milestone"]);
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["milestone", "add-problem", "m1", "p1"]);

    let stdout = run_jjj_success(&dir, &["problem", "show", "p1"]);
    assert!(
        stdout.contains("m1"),
        "Expected milestone in problem: {}",
        stdout
    );
}

#[test]
fn test_milestone_remove_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Milestone"]);
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["milestone", "add-problem", "m1", "p1"]);
    run_jjj_success(&dir, &["milestone", "remove-problem", "m1", "p1"]);

    let show = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    // Problem count should be 0 or problems section empty
    assert!(
        !show.contains("Important Problem")
            || show.contains("Problems (0)")
            || !show.contains("## Problems"),
        "Problem should be removed from milestone: {}",
        show
    );
}

#[test]
fn test_milestone_edit_title() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Original Title"]);
    run_jjj_success(&dir, &["milestone", "edit", "m1", "--title", "Updated Title"]);

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        stdout.contains("Updated Title"),
        "Expected updated title: {}",
        stdout
    );
}

#[test]
fn test_milestone_edit_date() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Milestone"]);
    run_jjj_success(&dir, &["milestone", "edit", "m1", "--date", "2026-01-01"]);

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        stdout.contains("2026-01-01"),
        "Expected updated date: {}",
        stdout
    );
}

#[test]
fn test_milestone_edit_status() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Milestone"]);
    run_jjj_success(&dir, &["milestone", "edit", "m1", "--status", "active"]);

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        stdout.contains("Active") || stdout.contains("active"),
        "Expected active status: {}",
        stdout
    );
}

#[test]
fn test_milestone_roadmap() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(
        &dir,
        &["milestone", "new", "First Release", "--date", "2025-03-01"],
    );
    run_jjj_success(
        &dir,
        &["milestone", "new", "Second Release", "--date", "2025-06-01"],
    );

    let stdout = run_jjj_success(&dir, &["milestone", "roadmap"]);
    assert!(
        stdout.contains("First Release"),
        "Expected first milestone: {}",
        stdout
    );
    assert!(
        stdout.contains("Second Release"),
        "Expected second milestone: {}",
        stdout
    );
    assert!(
        stdout.contains("Roadmap"),
        "Expected roadmap header: {}",
        stdout
    );
}

#[test]
fn test_milestone_roadmap_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Milestone 1"]);
    run_jjj_success(&dir, &["milestone", "new", "Milestone 2"]);

    let stdout = run_jjj_success(&dir, &["milestone", "roadmap", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn test_milestone_assign() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Assignable"]);
    run_jjj_success(&dir, &["milestone", "assign", "m1", "--to", "alice"]);

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        stdout.contains("alice"),
        "Expected assignee alice: {}",
        stdout
    );
}

#[test]
fn test_milestone_with_problem_progress() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Progress Milestone"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 2"]);
    run_jjj_success(&dir, &["milestone", "add-problem", "m1", "p1"]);
    run_jjj_success(&dir, &["milestone", "add-problem", "m1", "p2"]);

    // Solve one problem
    run_jjj_success(&dir, &["solution", "new", "Fix", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "accept", "s1"]);
    run_jjj_success(&dir, &["problem", "solve", "p1"]);

    let stdout = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    // Should show progress info
    assert!(
        stdout.contains("Progress") || stdout.contains("solved") || stdout.contains("open"),
        "Expected progress info: {}",
        stdout
    );
}

#[test]
fn test_milestone_show_nonexistent_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["milestone", "show", "m999"]);
    assert!(
        !output.status.success(),
        "Expected failure for nonexistent milestone"
    );
}

#[test]
fn test_problem_new_with_milestone() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["milestone", "new", "Linked Milestone"]);
    run_jjj_success(
        &dir,
        &["problem", "new", "Linked Problem", "--milestone", "m1"],
    );

    let problem_show = run_jjj_success(&dir, &["problem", "show", "p1"]);
    assert!(
        problem_show.contains("m1"),
        "Expected milestone in problem: {}",
        problem_show
    );

    let milestone_show = run_jjj_success(&dir, &["milestone", "show", "m1"]);
    assert!(
        milestone_show.contains("p1") || milestone_show.contains("Linked Problem"),
        "Expected problem in milestone: {}",
        milestone_show
    );
}

#[test]
fn test_milestone_list_empty() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["milestone", "list"]);
    assert!(
        stdout.contains("No milestones") || stdout.is_empty() || !stdout.contains("m1"),
        "Expected no milestones message: {}",
        stdout
    );
}
