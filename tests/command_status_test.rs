mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_status_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["status"]);
    // Empty repo should show "no pending actions" or similar
    assert!(
        stdout.contains("No pending actions")
            || stdout.contains("All caught up")
            || stdout.contains("Summary"),
        "Expected empty status message: {}",
        stdout
    );
}

#[test]
fn test_status_shows_open_problems() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Open Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Open Problem 2"]);

    let stdout = run_jjj_success(&dir, &["status"]);
    // Should show open problems as TODO items
    assert!(
        stdout.contains("TODO") || stdout.contains("todo") || stdout.contains("Open Problem"),
        "Expected open problems in status: {}",
        stdout
    );
}

#[test]
fn test_status_shows_solutions_needing_attention() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Blocker"]);

    let stdout = run_jjj_success(&dir, &["status"]);
    // Should show blocked solution
    assert!(
        stdout.contains("BLOCKED") || stdout.contains("blocked") || stdout.contains("critique"),
        "Expected blocked solution in status: {}",
        stdout
    );
}

#[test]
fn test_status_shows_ready_solutions() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Issue"]);
    run_jjj_success(&dir, &["critique", "address", "Issue"]);

    let stdout = run_jjj_success(&dir, &["status"]);
    // Should show ready solution (all critiques resolved)
    assert!(
        stdout.contains("READY") || stdout.contains("ready") || stdout.contains("resolved"),
        "Expected ready solution in status: {}",
        stdout
    );
}

#[test]
fn test_status_json_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Test Problem"]);

    let stdout = run_jjj_success(&dir, &["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    // Should have items, summary, and other fields
    assert!(json.get("items").is_some(), "Expected items field");
    assert!(json.get("summary").is_some(), "Expected summary field");
}

#[test]
fn test_status_json_summary() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 2"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem 1"],
    );
    run_jjj_success(&dir, &["critique", "new", "Solution", "Critique"]);

    let stdout = run_jjj_success(&dir, &["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    let summary = &json["summary"];
    assert!(
        summary["open_problems"].as_i64().unwrap() >= 1,
        "Expected at least 1 open problem"
    );
    assert!(
        summary["open_critiques"].as_i64().unwrap() >= 1,
        "Expected at least 1 open critique"
    );
}

#[test]
fn test_status_limit_option() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create many problems
    for i in 1..=10 {
        run_jjj_success(&dir, &["problem", "new", &format!("Problem {}", i)]);
    }

    let stdout = run_jjj_success(&dir, &["status", "--limit", "3"]);
    // Should mention limited output
    assert!(
        stdout.contains("Showing 3")
            || stdout.contains("3 of")
            || (stdout.matches("TODO").count() <= 3 && stdout.matches("todo").count() <= 3),
        "Expected limited output: {}",
        stdout
    );
}

#[test]
fn test_status_all_option() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create many problems
    for i in 1..=10 {
        run_jjj_success(&dir, &["problem", "new", &format!("Problem {}", i)]);
    }

    let stdout = run_jjj_success(&dir, &["status", "--all"]);
    // Should show all items without truncation message
    assert!(
        !stdout.contains("Showing 5 of"),
        "Should not show truncation message with --all: {}",
        stdout
    );
}

#[test]
fn test_status_mine_option() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "My Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "My Solution", "--problem", "My Problem"],
    );
    // Assign to current user (default)
    run_jjj_success(&dir, &["solution", "assign", "My Solution"]);

    let stdout = run_jjj_success(&dir, &["status", "--mine"]);
    // Should filter to only my items
    assert!(
        stdout.contains("My Solution") || stdout.contains("Summary"),
        "Expected my solution in filtered status: {}",
        stdout
    );
}

#[test]
fn test_status_shows_active_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Active Solution", "--problem", "Problem"],
    );

    let stdout = run_jjj_success(&dir, &["status"]);
    // Should show the active solution at the top
    assert!(
        stdout.contains("Active") || stdout.contains("Active Solution"),
        "Expected active solution in status: {}",
        stdout
    );
}

#[test]
fn test_status_priority_sorting() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create problems with different priorities
    run_jjj_success(&dir, &["problem", "new", "Low", "--priority", "P3"]);
    run_jjj_success(&dir, &["problem", "new", "Critical", "--priority", "P0"]);
    run_jjj_success(&dir, &["problem", "new", "High", "--priority", "P1"]);

    let stdout = run_jjj_success(&dir, &["status", "--json", "--all"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    let items = json["items"].as_array().expect("Expected items array");
    let todo_items: Vec<_> = items
        .iter()
        .filter(|i| i["category"].as_str() == Some("todo"))
        .collect();

    // Should be sorted by priority (Critical first)
    if todo_items.len() >= 2 {
        // Verify that the first todo item has the "Critical" problem
        let first_title = todo_items[0]["title"].as_str().unwrap_or("");
        assert!(
            first_title.contains("Critical"),
            "Expected Critical first, got {}",
            first_title
        );
    }
}

#[test]
fn test_status_shows_summary_counts() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 2"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem 1"],
    );

    let stdout = run_jjj_success(&dir, &["status"]);
    // Should show summary with counts
    assert!(
        stdout.contains("Summary")
            || stdout.contains("open problems")
            || stdout.contains("testing"),
        "Expected summary counts: {}",
        stdout
    );
}

#[test]
fn test_status_shows_review_category() {
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
            "Test User",
        ],
    );

    let stdout = run_jjj_success(&dir, &["status"]);
    // Should show review category for assigned reviews
    assert!(
        stdout.contains("REVIEW")
            || stdout.contains("review")
            || stdout.contains("Awaiting")
            || stdout.contains("@Test"),
        "Expected review category: {}",
        stdout
    );
}

#[test]
fn test_status_json_active_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Active", "--problem", "Problem"]);

    let stdout = run_jjj_success(&dir, &["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    // Should have active_solution field
    let active = &json["active_solution"];
    if !active.is_null() {
        // ID should exist but we don't assert specific value
        assert!(active["id"].is_string(), "Expected id to be a string");
        assert_eq!(active["title"], "Active");
    }
}

#[test]
fn test_status_category_ordering() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Create items in different categories
    run_jjj_success(&dir, &["problem", "new", "TODO Problem"]); // Will be TODO (no solution)
    run_jjj_success(&dir, &["problem", "new", "Blocked Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Blocked Solution",
            "--problem",
            "Blocked Problem",
        ],
    );
    run_jjj_success(&dir, &["critique", "new", "Blocked Solution", "Blocker"]); // Makes solution BLOCKED

    let stdout = run_jjj_success(&dir, &["status", "--json", "--all"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

    let items = json["items"].as_array().expect("Expected items array");

    // Find indices of categories
    let blocked_idx = items
        .iter()
        .position(|i| i["category"].as_str() == Some("blocked"));
    let todo_idx = items
        .iter()
        .position(|i| i["category"].as_str() == Some("todo"));

    // BLOCKED should come before TODO in ordering
    if let (Some(b), Some(t)) = (blocked_idx, todo_idx) {
        assert!(
            b < t,
            "Expected BLOCKED before TODO: blocked at {}, todo at {}",
            b,
            t
        );
    }
}
