use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
fn run_jjj(dir: &Path, args: &[&str]) -> Output {
    Command::new(&jjj_binary())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute jjj")
}

/// Helper to setup a test repo with jj and jjj initialized
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Init jj repo
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(["git", "init", "--colocate"])
        .status()
        .expect("Failed to run jj init");
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .status()
        .expect("Failed to set user.name");
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set user.email");

    // Initialize jjj
    run_jjj(temp_dir.path(), &["init"]);

    temp_dir
}

#[test]
fn test_events_validate_clean() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a problem
    let output = run_jjj(dir, &["problem", "new", "Validate Problem"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Create a solution linked to the problem
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Validate Solution",
            "--problem",
            "Validate Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Run events validate -- should succeed with exit code 0
    let output = run_jjj(dir, &["events", "validate"]);
    assert!(
        output.status.success(),
        "events validate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("valid"),
        "Expected 'valid' in validate output, got: {}",
        stdout
    );
}

#[test]
fn test_events_rebuild() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a problem
    let output = run_jjj(dir, &["problem", "new", "Rebuild Problem"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Create a solution linked to the problem
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Rebuild Solution",
            "--problem",
            "Rebuild Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Run events rebuild
    let output = run_jjj(dir, &["events", "rebuild"]);
    assert!(
        output.status.success(),
        "events rebuild failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rebuilt") || stdout.contains("Event log rebuilt"),
        "Expected rebuild confirmation in output, got: {}",
        stdout
    );

    // Verify events list afterwards contains expected event types
    let output = run_jjj(dir, &["events", "--json"]);
    assert!(
        output.status.success(),
        "events list failed after rebuild: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("problem_created"),
        "Missing problem_created event after rebuild. Events: {}",
        stdout
    );
    assert!(
        stdout.contains("solution_created"),
        "Missing solution_created event after rebuild. Events: {}",
        stdout
    );
}

#[test]
fn test_events_list_json() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create entities to generate events
    let output = run_jjj(dir, &["problem", "new", "JSON Problem"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "JSON Solution",
            "--problem",
            "JSON Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // List events with --json
    let output = run_jjj(dir, &["events", "--json"]);
    assert!(
        output.status.success(),
        "events --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the JSON output
    let events: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse events JSON output");

    // Must be an array
    assert!(
        events.is_array(),
        "Expected JSON array, got: {}",
        events
    );

    let arr = events.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "Expected at least one event in JSON output"
    );

    // Verify each event has the expected fields
    for event in arr {
        assert!(
            event.get("when").is_some(),
            "Event missing 'when' field: {}",
            event
        );
        assert!(
            event.get("type").is_some(),
            "Event missing 'type' field: {}",
            event
        );
        assert!(
            event.get("entity").is_some(),
            "Event missing 'entity' field: {}",
            event
        );
        assert!(
            event.get("by").is_some(),
            "Event missing 'by' field: {}",
            event
        );
    }
}

#[test]
fn test_events_filter_by_type() {
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a problem and a solution to generate both event types
    let output = run_jjj(dir, &["problem", "new", "Filter Problem"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Filter Solution",
            "--problem",
            "Filter Problem",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Filter events to only problem_created
    let output = run_jjj(dir, &["events", "--event-type", "problem_created", "--json"]);
    assert!(
        output.status.success(),
        "events --type problem_created failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let events: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse filtered events JSON");

    let arr = events.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "Expected at least one problem_created event"
    );

    // All returned events must be problem_created
    for event in arr {
        let event_type = event
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        assert_eq!(
            event_type, "problem_created",
            "Expected only problem_created events, got: {}",
            event_type
        );
    }

    // Verify that solution_created is NOT in the filtered output
    assert!(
        !stdout.contains("solution_created"),
        "Filtered output should not contain solution_created events"
    );
}

#[test]
fn test_events_no_jsonl_file() {
    // Events are embedded in jjj commit descriptions, not written to a file.
    // events.jsonl must never be created — its absence is what makes merges
    // conflict-free.
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create entities to trigger event emission
    let output = run_jjj(dir, &["problem", "new", "No File Problem"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "No File Solution",
            "--problem",
            "No File Problem",
        ],
    );
    assert!(output.status.success());

    // events.jsonl must NOT exist anywhere under .jj/jjj-meta/
    let events_file = dir.join(".jj").join("jjj-meta").join("events.jsonl");
    assert!(
        !events_file.exists(),
        "events.jsonl must not exist — events live in commit descriptions, not a file"
    );

    // Events are still readable from commit history
    let output = run_jjj(dir, &["events", "--json"]);
    assert!(output.status.success(), "events --json failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("problem_created"),
        "problem_created event missing from commit history. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("solution_created"),
        "solution_created event missing from commit history. Got: {}",
        stdout
    );
}

#[test]
fn test_events_approve_emits_two_events_in_one_commit() {
    // Approving a solution that fully resolves its parent problem should emit
    // BOTH solution_approved AND problem_solved. Both are queued via
    // set_pending_event() inside the same with_metadata() closure, so they
    // land in a single commit as two `jjj: <json>` lines.
    if jjj::jj::find_executable("jj").is_none() {
        return;
    }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create an isolated problem+solution pair (setup_test_repo already made
    // "Workflow Problem"; use a fresh name to avoid interference)
    let output = run_jjj(dir, &["problem", "new", "Two-Event Problem"]);
    assert!(output.status.success());

    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Two-Event Solution",
            "--problem",
            "Two-Event Problem",
        ],
    );
    assert!(output.status.success());

    // Submit then approve (--force skips the review-required check)
    run_jjj(dir, &["solution", "submit", "Two-Event Solution"]);
    let output = run_jjj(
        dir,
        &["solution", "approve", "Two-Event Solution", "--force"],
    );
    assert!(
        output.status.success(),
        "approve failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both events must appear — they came from the same commit
    let output = run_jjj(dir, &["events", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("solution_approved"),
        "Missing solution_approved event. Events: {}",
        stdout
    );
    assert!(
        stdout.contains("problem_solved"),
        "Missing problem_solved (auto-solve should be emitted in same commit as solution_approved). Events: {}",
        stdout
    );
}
