use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run the jjj binary
fn run_jjj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let debug_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/jjj");
    if !debug_dir.exists() {
        panic!("jjj binary not found at {:?}. Make sure to build first.", debug_dir);
    }
    Command::new(&debug_dir).current_dir(dir).args(args).output().expect("Failed to execute jjj")
}

/// Helper to setup a test repo with jj
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // Init jj repo
    Command::new("jj").current_dir(&temp_dir).args(&["git", "init", "--colocate"]).status().expect("Failed to run jj init");
    Command::new("jj").current_dir(&temp_dir).args(&["config", "set", "--user", "user.name", "Test User"]).status().expect("Failed user");
    Command::new("jj").current_dir(&temp_dir).args(&["config", "set", "--user", "user.email", "test@example.com"]).status().expect("Failed email");

    // Initialize jjj
    run_jjj(temp_dir.path(), &["init"]);

    // Create a problem to work with
    run_jjj(temp_dir.path(), &["problem", "new", "Workflow Problem"]);

    temp_dir
}

#[test]
fn test_workflow_start_new_solution() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // 1. Start a new solution by title
    let output = run_jjj(dir, &["start", "New Solution", "--problem", "P-1"]);
    assert!(output.status.success(), "start failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify solution created
    let list = run_jjj(dir, &["solution", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("New Solution") || stdout.contains("S-1"));

    // Verify current change has description
    let jj_log = Command::new("jj").current_dir(dir).args(&["log", "--no-graph", "-r", "@", "-T", "description"]).output().unwrap();
    if !jj_log.status.success() {
        println!("DEBUG: jj log failed: {}", String::from_utf8_lossy(&jj_log.stderr));
    }
    let desc = String::from_utf8_lossy(&jj_log.stdout);
    println!("DEBUG: Current Change Description: '{}'", desc);
    assert!(desc.contains("New Solution"), "Description mismatch. Got: '{}'", desc);
}

#[test]
fn test_workflow_start_resume_solution() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // 1. Create solution manually first
    run_jjj(dir, &["solution", "new", "Resume Me", "--problem", "P-1"]);

    // 2. Start (resume) by ID (ID should be S-1)
    let output = run_jjj(dir, &["start", "S-1"]);
    assert!(output.status.success(), "Start failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify change created for S-1
    let jj_log = Command::new("jj").current_dir(dir).args(&["log", "--no-graph", "-r", "@", "-T", "description"]).output().unwrap();
    let desc = String::from_utf8_lossy(&jj_log.stdout);
    println!("DEBUG: Resume Description: '{}'", desc);
    assert!(desc.contains("Resume Me"), "Resume desc mismatch. Got: '{}'", desc);
}

#[test]
fn test_workflow_submit_force() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // 1. Start solution
    let output = run_jjj(dir, &["start", "Solution to Submit", "--problem", "P-1"]);
    assert!(output.status.success(), "Start 1 failed: {}", String::from_utf8_lossy(&output.stderr));

    std::fs::write(dir.join("file.txt"), "content").unwrap();

    // Create 'main' manually
    let status = Command::new("jj").current_dir(dir).args(&["new", "root()", "-m", "initial"]).status().unwrap();
    assert!(status.success(), "Failed to create initial commit");
    let status = Command::new("jj").current_dir(dir).args(&["bookmark", "create", "main"]).status().unwrap();
    assert!(status.success());

    let log = Command::new("jj").current_dir(dir).args(&["log", "--no-graph", "-r", "all"]).output().unwrap();
    println!("DEBUG: Repo Graph before start 2:\nSTDOUT:{}\nSTDERR:{}",
        String::from_utf8_lossy(&log.stdout), String::from_utf8_lossy(&log.stderr));

    // Start solution 2
    let output = run_jjj(dir, &["start", "Solution to Submit 2", "--problem", "P-1"]);
    assert!(output.status.success(), "Start 2 failed: {}", String::from_utf8_lossy(&output.stderr));

    std::fs::write(dir.join("file2.txt"), "content").unwrap();

    let log = Command::new("jj").current_dir(dir).args(&["log", "--no-graph", "-r", "all"]).output().unwrap();
    println!("DEBUG: Repo Graph before submit:\nSTDOUT:{}\nSTDERR:{}",
        String::from_utf8_lossy(&log.stdout), String::from_utf8_lossy(&log.stderr));

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
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a solution
    run_jjj(dir, &["solution", "new", "Test Solution", "--problem", "P-1"]);

    // Check initial status is proposed
    let show = run_jjj(dir, &["solution", "show", "S-1"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("Proposed") || stdout.contains("proposed"));

    // Move to testing
    let output = run_jjj(dir, &["solution", "test", "S-1"]);
    assert!(output.status.success());

    let show = run_jjj(dir, &["solution", "show", "S-1"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("Testing") || stdout.contains("testing"));

    // Accept the solution
    let output = run_jjj(dir, &["solution", "accept", "S-1"]);
    assert!(output.status.success());

    let show = run_jjj(dir, &["solution", "show", "S-1"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("Accepted") || stdout.contains("accepted"));
}

#[test]
fn test_critique_blocks_acceptance() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create a solution and move to testing
    run_jjj(dir, &["solution", "new", "Test Solution", "--problem", "P-1"]);
    run_jjj(dir, &["solution", "test", "S-1"]);

    // Add a critique
    run_jjj(dir, &["critique", "new", "S-1", "Major flaw in approach", "--severity", "high"]);

    // Try to accept - should fail or warn due to open critique
    let output = run_jjj(dir, &["solution", "accept", "S-1"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention critique or warning
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("critique") || combined.contains("warning") || combined.contains("open"),
        "Expected warning about open critique. Got: {}", combined
    );
}
