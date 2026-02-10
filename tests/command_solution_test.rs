mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_solution_new_creates_solution() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Test Problem"]);
    let stdout = run_jjj_success(&dir, &["solution", "new", "Test Solution", "--problem", "p1"]);

    assert!(stdout.contains("s1"), "Expected s1 in output: {}", stdout);
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
    run_jjj_success(&dir, &["solution", "new", "Linked Solution", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "s1"]);
    assert!(
        stdout.contains("p1"),
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
    run_jjj_success(&dir, &["solution", "new", "Solution", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "s1"]);
    // Solution new auto-attaches and moves to testing
    assert!(
        stdout.contains("Changes") || stdout.contains("Testing") || stdout.contains("testing"),
        "Expected change attached or testing status: {}",
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
    run_jjj_success(&dir, &["solution", "new", "Solution 1", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "new", "Solution 2", "--problem", "p2"]);

    let stdout = run_jjj_success(&dir, &["solution", "list"]);
    assert!(stdout.contains("s1"), "Expected s1: {}", stdout);
    assert!(stdout.contains("s2"), "Expected s2: {}", stdout);
}

#[test]
fn test_solution_list_filter_by_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem 1"]);
    run_jjj_success(&dir, &["problem", "new", "Problem 2"]);
    run_jjj_success(&dir, &["solution", "new", "Solution for P1", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "new", "Solution for P2", "--problem", "p2"]);

    let stdout = run_jjj_success(&dir, &["solution", "list", "--problem", "p1"]);
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
    run_jjj_success(&dir, &["solution", "new", "Testing Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "new", "Accepted Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "accept", "s2"]);

    let stdout = run_jjj_success(&dir, &["solution", "list", "--status", "accepted"]);
    assert!(
        stdout.contains("Accepted Solution"),
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
    run_jjj_success(&dir, &["solution", "new", "JSON Solution", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "list", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "s1");
    assert_eq!(arr[0]["title"], "JSON Solution");
}

#[test]
fn test_solution_show_details() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Detailed Solution", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "s1"]);
    assert!(
        stdout.contains("Detailed Solution"),
        "Expected title: {}",
        stdout
    );
    assert!(stdout.contains("p1"), "Expected problem id: {}", stdout);
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
    run_jjj_success(&dir, &["solution", "new", "JSON Show", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "s1", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert_eq!(json["id"], "s1");
    assert_eq!(json["title"], "JSON Show");
    assert_eq!(json["problem_id"], "p1");
}

#[test]
fn test_solution_accept() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Acceptable", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "accept", "s1"]);
    assert!(
        stdout.contains("accepted"),
        "Expected accepted confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["solution", "show", "s1"]);
    assert!(
        show.contains("Accepted") || show.contains("accepted"),
        "Expected accepted status: {}",
        show
    );
}

#[test]
fn test_solution_accept_blocked_by_critiques() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Blocked Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Major flaw"]);

    let output = run_jjj(&dir, &["solution", "accept", "s1"]);
    assert!(
        !output.status.success(),
        "Accept should fail with open critiques"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("critique") || stderr.contains("c1"),
        "Expected critique blocking message: {}",
        stderr
    );
}

#[test]
fn test_solution_accept_force() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Force Accept", "--problem", "p1"]);
    run_jjj_success(&dir, &["critique", "new", "s1", "Minor issue"]);

    let stdout = run_jjj_success(&dir, &["solution", "accept", "s1", "--force"]);
    assert!(
        stdout.contains("accepted"),
        "Expected force accepted: {}",
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
    run_jjj_success(&dir, &["solution", "new", "Bad Solution", "--problem", "p1"]);

    let stdout = run_jjj_success(&dir, &["solution", "refute", "s1"]);
    assert!(
        stdout.contains("refuted"),
        "Expected refuted confirmation: {}",
        stdout
    );

    let show = run_jjj_success(&dir, &["solution", "show", "s1"]);
    assert!(
        show.contains("Refuted") || show.contains("refuted"),
        "Expected refuted status: {}",
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
    run_jjj_success(&dir, &["solution", "new", "Original Solution", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "refute", "s1"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Better Solution",
            "--problem",
            "p1",
            "--supersedes",
            "s1",
        ],
    );

    let stdout = run_jjj_success(&dir, &["solution", "show", "s2"]);
    assert!(
        stdout.contains("Supersedes") || stdout.contains("s1"),
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
    run_jjj_success(&dir, &["solution", "new", "Test Status", "--problem", "p1"]);

    // solution new auto-attaches and moves to testing
    let show = run_jjj_success(&dir, &["solution", "show", "s1"]);
    assert!(
        show.contains("Testing") || show.contains("testing"),
        "Expected testing status: {}",
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
    run_jjj_success(&dir, &["solution", "new", "Assigned", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "assign", "s1", "--to", "bob"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "s1"]);
    assert!(stdout.contains("bob"), "Expected assignee bob: {}", stdout);
}

#[test]
fn test_solution_edit() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["solution", "new", "Original", "--problem", "p1"]);
    run_jjj_success(&dir, &["solution", "edit", "s1", "--title", "Updated Title"]);

    let stdout = run_jjj_success(&dir, &["solution", "show", "s1"]);
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
            "p1",
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

    let output = run_jjj(&dir, &["solution", "show", "s999"]);
    assert!(
        !output.status.success(),
        "Expected failure for nonexistent solution"
    );
}
