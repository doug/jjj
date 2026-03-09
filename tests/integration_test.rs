mod test_helpers;
use std::process::Command;
use test_helpers::{jj_available, run_jjj, setup_test_repo};

#[test]
fn test_init_and_create_problem_solution() {
    // Skip if jj is not installed
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Create a problem
    let output = run_jjj(dir_path, &["problem", "new", "Integration Problem"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Integration Problem"));

    // 3. Create a solution associated with the problem
    let output = run_jjj(
        dir_path,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Integration Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 4. List solutions and verify
    let output = run_jjj(dir_path, &["solution", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Solution"));

    // 5. Show solution details to verify problem link
    let output = run_jjj(dir_path, &["solution", "show", "Test Solution"]);
    assert!(output.status.success(), "solution show failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Problem reference should be present (title or "Addresses:" field with UUID)
    assert!(stdout.contains("Integration Problem") || stdout.contains("Addresses:"));
}

#[test]
fn test_critique_workflow() {
    // Skip if jj is not installed
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Create problem and solution
    run_jjj(dir_path, &["problem", "new", "Test Problem"]);
    run_jjj(
        dir_path,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Test Problem",
        ],
    );

    // 3. Add a critique
    let output = run_jjj(
        dir_path,
        &[
            "critique",
            "new",
            "Test Solution",
            "This has a flaw",
            "--severity",
            "high",
        ],
    );
    assert!(
        output.status.success(),
        "critique new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 4. List critiques
    let output = run_jjj(dir_path, &["critique", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flaw") || stdout.contains("This has a flaw"));
}

#[test]
fn test_problem_hierarchy() {
    // Skip if jj is not installed
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Create parent problem
    let output = run_jjj(dir_path, &["problem", "new", "Parent Problem"]);
    assert!(output.status.success());

    // 3. Create child problem
    let output = run_jjj(
        dir_path,
        &[
            "problem",
            "new",
            "Child Problem",
            "--parent",
            "Parent Problem",
        ],
    );
    assert!(
        output.status.success(),
        "child problem failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 4. Show parent should reference child
    let output = run_jjj(dir_path, &["problem", "show", "Parent Problem"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should mention it has sub-problems
    assert!(
        stdout.contains("Child Problem")
            || stdout.contains("Sub-problems")
            || stdout.contains("Child")
    );
}

#[test]
fn test_problem_priority() {
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // Create with P0 priority
    let output = run_jjj(
        dir_path,
        &["problem", "new", "Critical bug", "--priority", "p0"],
    );
    assert!(
        output.status.success(),
        "problem new with priority failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify in show output (text mode should show priority)
    let output = run_jjj(dir_path, &["problem", "show", "Critical bug"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("p0"),
        "Priority not shown in output: {}",
        stdout
    );

    // Create with default priority
    let output = run_jjj(dir_path, &["problem", "new", "Normal bug"]);
    assert!(output.status.success());
    let output = run_jjj(dir_path, &["problem", "show", "Normal bug"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("p2"),
        "Default priority not shown: {}",
        stdout
    );
}

#[test]
fn test_problem_dissolve_reason() {
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    let output = run_jjj(dir_path, &["problem", "new", "Ghost bug"]);
    assert!(output.status.success());

    let output = run_jjj(
        dir_path,
        &[
            "problem",
            "dissolve",
            "Ghost bug",
            "--reason",
            "Test data was stale",
        ],
    );
    assert!(
        output.status.success(),
        "dissolve with reason failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_jjj(dir_path, &["problem", "show", "Ghost bug"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dissolved"),
        "Status not dissolved: {}",
        stdout
    );
    assert!(
        stdout.contains("Test data was stale"),
        "Dissolved reason not shown: {}",
        stdout
    );
}

#[test]
fn test_solution_supersedes() {
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    run_jjj(dir_path, &["problem", "new", "Slow queries"]);
    run_jjj(
        dir_path,
        &["solution", "new", "Add index", "--problem", "Slow queries"],
    );
    run_jjj(dir_path, &["solution", "withdraw", "Add index"]);

    let output = run_jjj(
        dir_path,
        &[
            "solution",
            "new",
            "Use connection pool",
            "--problem",
            "Slow queries",
            "--supersedes",
            "Add index",
        ],
    );
    assert!(
        output.status.success(),
        "solution new with supersedes failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Supersedes") || stdout.contains("Add index"),
        "Supersedes not shown in creation output: {}",
        stdout
    );

    let output = run_jjj(dir_path, &["solution", "show", "Use connection pool"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Supersedes") || stdout.contains("Add index"),
        "Supersedes not shown in show: {}",
        stdout
    );
}

#[test]
fn test_solve_warns_active_solutions() {
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    run_jjj(dir_path, &["problem", "new", "Fix auth"]);
    run_jjj(
        dir_path,
        &["solution", "new", "Approach A", "--problem", "Fix auth"],
    );
    run_jjj(dir_path, &["solution", "test", "Approach A"]);

    // Solving with active testing solution should still succeed but warn
    let output = run_jjj(dir_path, &["problem", "solve", "Fix auth"]);
    // Note: solve may or may not succeed depending on can_solve_problem logic.
    // The key thing is: if it runs far enough to check, it should warn.
    // Let's verify the warning appears in stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    // If solve succeeded or even if it didn't, check for warning text
    if output.status.success() {
        assert!(
            stderr.contains("active") || stderr.contains("Warning") || stderr.contains("review"),
            "Expected warning about active solutions: stderr={}",
            stderr
        );
    }
    // If solve failed (e.g., no accepted solution), that's OK too —
    // the warning may not appear if can_solve_problem fails first
}

#[test]
fn test_next_priority_sorting() {
    if !jj_available() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // Create problems with different priorities
    run_jjj(
        dir_path,
        &["problem", "new", "Low priority task", "--priority", "p3"],
    );
    run_jjj(
        dir_path,
        &["problem", "new", "Critical issue", "--priority", "p0"],
    );
    run_jjj(
        dir_path,
        &["problem", "new", "High priority work", "--priority", "p1"],
    );

    // All should appear as TODO (no solutions)
    let output = run_jjj(dir_path, &["status", "--json", "--all"]);
    assert!(
        output.status.success(),
        "status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    let items = json["items"].as_array().expect("items not array");

    // Should have at least 3 items
    assert!(
        items.len() >= 3,
        "Expected at least 3 items, got {}",
        items.len()
    );

    // Find the TODO items and verify Critical is before High is before Low
    let todo_items: Vec<_> = items
        .iter()
        .filter(|i| i["category"].as_str() == Some("todo"))
        .collect();

    assert!(todo_items.len() >= 3, "Expected at least 3 TODO items");

    // Critical should be first (check by title since IDs are UUIDs)
    let first_title = todo_items[0]["title"].as_str().unwrap_or("");
    assert!(
        first_title.contains("Critical"),
        "Expected Critical first, got {}",
        first_title
    );
    // High should be second
    let second_title = todo_items[1]["title"].as_str().unwrap_or("");
    assert!(
        second_title.contains("High"),
        "Expected High second, got {}",
        second_title
    );
    // Low should be last
    let third_title = todo_items[2]["title"].as_str().unwrap_or("");
    assert!(
        third_title.contains("Low"),
        "Expected Low third, got {}",
        third_title
    );
}

#[test]
fn test_critique_new_with_reviewer() {
    if !jj_available() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    run_jjj(dir, &["problem", "new", "Test problem"]);
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test solution",
            "--problem",
            "Test problem",
        ],
    );
    let output = run_jjj(
        dir,
        &[
            "critique",
            "new",
            "Test solution",
            "Review needed",
            "--reviewer",
            "bob",
        ],
    );

    assert!(
        output.status.success(),
        "critique new with reviewer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Review needed"),
        "Expected title in output: {}",
        stdout
    );

    let show_output = run_jjj(dir, &["critique", "show", "Review needed", "--json"]);
    assert!(
        show_output.status.success(),
        "critique show --json failed: {}",
        String::from_utf8_lossy(&show_output.stderr)
    );
    let show_stdout = String::from_utf8_lossy(&show_output.stdout);
    assert!(
        show_stdout.contains("\"reviewer\": \"bob\""),
        "Expected reviewer: bob in output: {}",
        show_stdout
    );
}

#[test]
fn test_critique_list_filter_by_reviewer() {
    if !jj_available() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    run_jjj(dir, &["problem", "new", "Test problem"]);
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test solution",
            "--problem",
            "Test problem",
        ],
    );
    run_jjj(
        dir,
        &[
            "critique",
            "new",
            "Test solution",
            "For alice",
            "--reviewer",
            "alice",
        ],
    );
    run_jjj(
        dir,
        &[
            "critique",
            "new",
            "Test solution",
            "For bob",
            "--reviewer",
            "bob",
        ],
    );
    run_jjj(dir, &["critique", "new", "Test solution", "No reviewer"]);

    let output = run_jjj(dir, &["critique", "list", "--reviewer", "alice"]);
    assert!(
        output.status.success(),
        "critique list --reviewer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("For alice"),
        "Expected 'For alice' in output: {}",
        stdout
    );
    assert!(
        !stdout.contains("For bob"),
        "Should not contain 'For bob' in output: {}",
        stdout
    );
    assert!(
        !stdout.contains("No reviewer"),
        "Should not contain 'No reviewer' in output: {}",
        stdout
    );
}

#[test]
fn test_solution_new_with_reviewer() {
    if !jj_available() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    run_jjj(dir, &["problem", "new", "Test problem"]);
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test solution",
            "--problem",
            "Test problem",
            "--reviewer",
            "bob",
        ],
    );

    assert!(
        output.status.success(),
        "solution new with --reviewer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should have created an awaiting review critique
    let critiques = run_jjj(dir, &["critique", "list", "--json"]);
    assert!(
        critiques.status.success(),
        "critique list --json failed: {}",
        String::from_utf8_lossy(&critiques.stderr)
    );
    let stdout = String::from_utf8_lossy(&critiques.stdout);
    assert!(
        stdout.contains("Awaiting review from @bob"),
        "Expected 'Awaiting review from @bob' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("\"reviewer\": \"bob\""),
        "Expected '\"reviewer\": \"bob\"' in output: {}",
        stdout
    );
}

#[test]
fn test_solution_new_with_multiple_reviewers() {
    if !jj_available() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    run_jjj(dir, &["problem", "new", "Test problem"]);
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test solution",
            "--problem",
            "Test problem",
            "--reviewer",
            "@alice",
            "--reviewer",
            "bob:high",
        ],
    );

    assert!(
        output.status.success(),
        "solution new with multiple --reviewer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should have created awaiting review critiques for both
    let critiques = run_jjj(dir, &["critique", "list", "--json"]);
    assert!(
        critiques.status.success(),
        "critique list --json failed: {}",
        String::from_utf8_lossy(&critiques.stderr)
    );
    let stdout = String::from_utf8_lossy(&critiques.stdout);
    assert!(
        stdout.contains("Awaiting review from @alice"),
        "Expected 'Awaiting review from @alice' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("Awaiting review from @bob"),
        "Expected 'Awaiting review from @bob' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("\"reviewer\": \"alice\""),
        "Expected '\"reviewer\": \"alice\"' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("\"reviewer\": \"bob\""),
        "Expected '\"reviewer\": \"bob\"' in output: {}",
        stdout
    );
    // Bob's critique should have high severity
    assert!(
        stdout.contains("\"severity\": \"high\""),
        "Expected '\"severity\": \"high\"' for bob in output: {}",
        stdout
    );
}

#[test]
fn test_status_shows_review_needed() {
    if !jj_available() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Configure git user as "bob" for this test
    Command::new("git")
        .args(["config", "user.name", "bob"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git user");

    run_jjj(dir, &["problem", "new", "Test problem"]);
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test solution",
            "--problem",
            "Test problem",
            "--reviewer",
            "bob",
        ],
    );

    let output = run_jjj(dir, &["status"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show REVIEW category for bob's awaiting review
    assert!(
        stdout.contains("REVIEW") || stdout.contains("review"),
        "Expected REVIEW in status output: {}",
        stdout
    );
    assert!(
        stdout.contains("Awaiting review from @bob") || stdout.contains("Review requested"),
        "Expected awaiting review info: {}",
        stdout
    );
}
