use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the jjj binary path
fn jjj_binary() -> PathBuf {
    let debug_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/jjj");
    if !debug_dir.exists() {
        panic!(
            "jjj binary not found at {:?}. Make sure to build first.",
            debug_dir
        );
    }
    debug_dir
}

/// Helper to run the jjj binary
fn run_jjj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(&jjj_binary())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute jjj")
}

/// Helper to setup a test repo with jj
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // Init jj repo
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(&["git", "init", "--colocate"])
        .status()
        .expect("Failed to run jj init");
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(&["config", "set", "--repo", "user.name", "Test User"])
        .status()
        .expect("Failed user");
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(&["config", "set", "--repo", "user.email", "test@example.com"])
        .status()
        .expect("Failed email");

    // Initialize jjj
    run_jjj(temp_dir.path(), &["init"]);

    // Create a problem to work with
    run_jjj(temp_dir.path(), &["problem", "new", "Workflow Problem"]);

    temp_dir
}

#[test]
fn test_workflow_start_new_solution() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // 1. Create a new solution via solution new (replaces start)
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "New Solution",
            "--problem",
            "Workflow Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify solution created
    let list = run_jjj(dir, &["solution", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("New Solution"));

    // Verify current change has description
    let jj_log = Command::new("jj")
        .current_dir(dir)
        .args(&["log", "--no-graph", "-r", "@", "-T", "description"])
        .output()
        .unwrap();
    if !jj_log.status.success() {
        println!(
            "DEBUG: jj log failed: {}",
            String::from_utf8_lossy(&jj_log.stderr)
        );
    }
    let desc = String::from_utf8_lossy(&jj_log.stdout);
    println!("DEBUG: Current Change Description: '{}'", desc);
    assert!(
        desc.contains("New Solution"),
        "Description mismatch. Got: '{}'",
        desc
    );
}

#[test]
fn test_workflow_start_resume_solution() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // 1. Create solution via solution new (auto-attaches change)
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Resume Me",
            "--problem",
            "Workflow Problem",
        ],
    );

    // 2. Resume by title using solution resume
    let output = run_jjj(dir, &["solution", "resume", "Resume Me"]);
    assert!(
        output.status.success(),
        "Resume failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify change is for the solution
    let jj_log = Command::new("jj")
        .current_dir(dir)
        .args(&["log", "--no-graph", "-r", "@", "-T", "description"])
        .output()
        .unwrap();
    let desc = String::from_utf8_lossy(&jj_log.stdout);
    println!("DEBUG: Resume Description: '{}'", desc);
    assert!(
        desc.contains("Resume Me"),
        "Resume desc mismatch. Got: '{}'",
        desc
    );
}

#[test]
fn test_workflow_submit_force() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // 1. Create solution via solution new (replaces start)
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Solution to Submit",
            "--problem",
            "Workflow Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new 1 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::write(dir.join("file.txt"), "content").unwrap();

    // Create 'main' manually
    let status = Command::new("jj")
        .current_dir(dir)
        .args(&["new", "root()", "-m", "initial"])
        .status()
        .unwrap();
    assert!(status.success(), "Failed to create initial commit");
    let status = Command::new("jj")
        .current_dir(dir)
        .args(&["bookmark", "create", "main"])
        .status()
        .unwrap();
    assert!(status.success());

    let log = Command::new("jj")
        .current_dir(dir)
        .args(&["log", "--no-graph", "-r", "all"])
        .output()
        .unwrap();
    println!(
        "DEBUG: Repo Graph before start 2:\nSTDOUT:{}\nSTDERR:{}",
        String::from_utf8_lossy(&log.stdout),
        String::from_utf8_lossy(&log.stderr)
    );

    // Create solution 2
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Solution to Submit 2",
            "--problem",
            "Workflow Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new 2 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::write(dir.join("file2.txt"), "content").unwrap();

    let log = Command::new("jj")
        .current_dir(dir)
        .args(&["log", "--no-graph", "-r", "all"])
        .output()
        .unwrap();
    println!(
        "DEBUG: Repo Graph before submit:\nSTDOUT:{}\nSTDERR:{}",
        String::from_utf8_lossy(&log.stdout),
        String::from_utf8_lossy(&log.stderr)
    );

    // Move to review before submitting (--force bypasses open critiques, not review requirement)
    run_jjj(dir, &["solution", "review", "Solution to Submit 2"]);

    let output = run_jjj(dir, &["submit", "--force"]);

    if !output.status.success() {
        println!("Submit failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    // Verify solution is Accepted
    let list = run_jjj(dir, &["solution", "list", "--status", "accepted"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("Solution to Submit 2") || stdout.contains("accepted"));
}

#[test]
fn test_solution_status_workflow() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a solution (auto-attaches change but stays Proposed)
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Workflow Problem",
        ],
    );

    // Check initial status is proposed (solution new stays Proposed now)
    let show = run_jjj(dir, &["solution", "show", "Test Solution"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(
        stdout.contains("Proposed") || stdout.contains("proposed"),
        "Expected proposed status after solution new. Got: {}",
        stdout
    );

    // Advance to review, then submit
    run_jjj(dir, &["solution", "review", "Test Solution"]);
    let output = run_jjj(dir, &["submit", "Test Solution"]);
    assert!(output.status.success());

    let show = run_jjj(dir, &["solution", "show", "Test Solution"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("Accepted") || stdout.contains("accepted"));
}

#[test]
fn test_critique_blocks_acceptance() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a solution and move to review
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Workflow Problem",
        ],
    );
    run_jjj(dir, &["solution", "review", "Test Solution"]);

    // Add a critique
    run_jjj(
        dir,
        &[
            "critique",
            "new",
            "Test Solution",
            "Major flaw in approach",
            "--severity",
            "high",
        ],
    );

    // Try to submit - should fail due to open critique
    let output = run_jjj(dir, &["submit", "Test Solution"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !output.status.success() && (combined.contains("critique") || combined.contains("open")),
        "Expected failure due to open critique. Got: {}",
        combined
    );
}

#[test]
fn test_submit_blocked_by_critiques() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a main bookmark so submit has something to rebase onto
    Command::new("jj")
        .current_dir(dir)
        .args(&["new", "root()", "-m", "initial"])
        .status()
        .unwrap();
    Command::new("jj")
        .current_dir(dir)
        .args(&["bookmark", "create", "main"])
        .status()
        .unwrap();

    // Create solution (creates change, auto-attaches; stays Proposed)
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Token refresh",
            "--problem",
            "Workflow Problem",
        ],
    );

    // Move to review so submit can reach the critique check
    run_jjj(dir, &["solution", "review", "Token refresh"]);

    // Add a critique
    run_jjj(
        dir,
        &[
            "critique",
            "new",
            "Token refresh",
            "Not thread safe",
            "--severity",
            "high",
        ],
    );

    // Submit without --force should fail
    let output = run_jjj(dir, &["submit"]);
    assert!(
        !output.status.success(),
        "Expected submit to fail with open critiques"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("critique") || stderr.contains("Cannot"),
        "Expected critique blocking message in stderr: {}",
        stderr
    );
}

// Note: test_submit_blocked_by_review was removed as the reviewer/sign-off system
// has been replaced with critique-based reviews. The test_submit_blocked_by_critiques
// test covers the blocking behavior via the unified critique system.

/// Regression test for the "working copy is stale" bug.
///
/// `commit_changes()` in storage/mod.rs coordinates two jj workspaces:
///  1. meta workspace: `jj workspace update-stale` then `jj new` (snapshots
///     metadata files into a commit, advances shared op log to op N)
///  2. main workspace: `jj bookmark set --ignore-working-copy` (points the
///     jjj bookmark at the new meta commit; --ignore-working-copy bypasses the
///     stale check that would otherwise fail because op N > main's last sync)
///
/// Without `workspace update-stale` before `jj new` in the meta workspace, a
/// prior `bookmark set` in the main workspace (op N-1) would leave the meta
/// workspace stale, causing `jj new` to fail on the next call.
///
/// This test exercises the full call path through multiple metadata writes and
/// verifies that `jj status` in the main workspace succeeds after each one.
#[test]
fn test_no_stale_working_copy_after_metadata_writes() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    let assert_not_stale = |label: &str| {
        let out = Command::new("jj")
            .current_dir(dir)
            .args(&["status"])
            .output()
            .expect("Failed to run jj status");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("stale"),
            "{}: jj status reported stale working copy:\n{}",
            label,
            stderr
        );
        assert!(
            out.status.success(),
            "{}: jj status exited non-zero:\n{}",
            label,
            stderr
        );
    };

    // setup already ran `jjj init` + `problem new` (two commit_changes calls)
    assert_not_stale("after setup");

    // Each of these triggers another commit_changes()
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Stale Regression Solution",
            "--problem",
            "Workflow Problem",
        ],
    );
    assert_not_stale("after solution new");

    run_jjj(
        dir,
        &[
            "critique",
            "new",
            "Stale Regression Solution",
            "Needs more tests",
            "--severity",
            "low",
        ],
    );
    assert_not_stale("after critique new");

    run_jjj(dir, &["solution", "review", "Stale Regression Solution"]);
    assert_not_stale("after solution review");

    run_jjj(
        dir,
        &["submit", "Stale Regression Solution", "--force"],
    );
    assert_not_stale("after submit");
}

#[test]
fn test_submit_blocked_by_awaiting_review() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    run_jjj(dir, &["init"]);
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

    // Move to review — submit now requires Review state; the awaiting-review critique blocks it
    run_jjj(dir, &["solution", "review", "Test solution"]);

    // Submit should fail because awaiting review critique is open
    let output = Command::new(jjj_binary())
        .args(["submit"])
        .current_dir(dir)
        .output()
        .expect("Failed to execute jjj submit");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "Submit should have failed");
    assert!(
        stderr.contains("open critique") || stderr.contains("Awaiting review"),
        "Expected open critique error, got: {}",
        stderr
    );
}

#[test]
fn test_events_logged_on_status_changes() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create and submit a solution (force to bypass review requirement)
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Workflow Problem",
        ],
    );
    run_jjj(dir, &["solution", "review", "Test Solution"]);
    run_jjj(dir, &["submit", "Test Solution", "--force"]);

    // Check events
    let output = run_jjj(dir, &["events", "--json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("solution_created"),
        "Missing solution_created event"
    );
    assert!(
        stdout.contains("solution_accepted"),
        "Missing solution_accepted event"
    );
}
