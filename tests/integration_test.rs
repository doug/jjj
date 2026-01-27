use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run the jjj binary
fn run_jjj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let debug_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/jjj");
    // Ensure binary exists
    if !debug_dir.exists() {
        panic!("jjj binary not found at {:?}. Make sure to build first.", debug_dir);
    }

    Command::new(&debug_dir)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute jjj")
}

/// Helper to setup a test repo with jj
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Check if jj is installed
    if which::which("jj").is_err() {
        eprintln!("Skipping test: jj not found");
        return temp_dir;
    }

    // Init jj repo
    let status = Command::new("jj")
        .current_dir(&temp_dir)
        .arg("git")
        .arg("init")
        .status()
        .expect("Failed to run jj init");

    assert!(status.success(), "jj git init failed");

    // Configure user for jj
    Command::new("jj")
        .current_dir(&temp_dir)
        .args(&["config", "set", "--user", "user.name", "Test User"])
        .status()
        .expect("Failed to set user name");

    Command::new("jj")
        .current_dir(&temp_dir)
        .args(&["config", "set", "--user", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set user email");

    temp_dir
}

#[test]
fn test_init_and_create_problem_solution() {
    // Skip if jj is not installed
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Run jjj init
    let output = run_jjj(dir_path, &["init"]);
    assert!(output.status.success(), "jjj init failed: {}", String::from_utf8_lossy(&output.stderr));

    // 2. Create a problem
    let output = run_jjj(dir_path, &["problem", "new", "Integration Problem"]);
    assert!(output.status.success(), "problem new failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("P-1") || stdout.contains("Integration Problem"));

    // 3. Create a solution associated with the problem
    let output = run_jjj(dir_path, &["solution", "new", "Test Solution", "--problem", "P-1", "--tag", "integration"]);
    assert!(output.status.success(), "solution new failed: {}", String::from_utf8_lossy(&output.stderr));

    // 4. List solutions and verify
    let output = run_jjj(dir_path, &["solution", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Solution") || stdout.contains("S-1"));

    // 5. Show solution details to verify problem link
    let output = run_jjj(dir_path, &["solution", "show", "S-1"]);
    assert!(output.status.success(), "solution show failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Problem ID should be present
    assert!(stdout.contains("P-1"));
}

#[test]
fn test_critique_workflow() {
    // Skip if jj is not installed
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Initialize
    let output = run_jjj(dir_path, &["init"]);
    assert!(output.status.success());

    // 2. Create problem and solution
    run_jjj(dir_path, &["problem", "new", "Test Problem"]);
    run_jjj(dir_path, &["solution", "new", "Test Solution", "--problem", "P-1"]);

    // 3. Add a critique
    let output = run_jjj(dir_path, &["critique", "new", "S-1", "This has a flaw", "--severity", "high"]);
    assert!(output.status.success(), "critique new failed: {}", String::from_utf8_lossy(&output.stderr));

    // 4. List critiques
    let output = run_jjj(dir_path, &["critique", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flaw") || stdout.contains("CQ-1"));
}

#[test]
fn test_problem_hierarchy() {
    // Skip if jj is not installed
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Initialize
    run_jjj(dir_path, &["init"]);

    // 2. Create parent problem
    let output = run_jjj(dir_path, &["problem", "new", "Parent Problem"]);
    assert!(output.status.success());

    // 3. Create child problem
    let output = run_jjj(dir_path, &["problem", "new", "Child Problem", "--parent", "P-1"]);
    assert!(output.status.success(), "child problem failed: {}", String::from_utf8_lossy(&output.stderr));

    // 4. Show parent should reference child
    let output = run_jjj(dir_path, &["problem", "show", "P-1"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should mention it has sub-problems
    assert!(stdout.contains("P-2") || stdout.contains("Sub-problems") || stdout.contains("Child"));
}

#[test]
fn test_problem_priority() {
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();
    run_jjj(dir_path, &["init"]);

    // Create with P0 priority
    let output = run_jjj(dir_path, &["problem", "new", "Critical bug", "--priority", "P0"]);
    assert!(output.status.success(), "problem new with priority failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify in show output (text mode should show priority)
    let output = run_jjj(dir_path, &["problem", "show", "P-1"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("P0/critical") || stdout.contains("Critical"), "Priority not shown in output: {}", stdout);

    // Create with default priority
    let output = run_jjj(dir_path, &["problem", "new", "Normal bug"]);
    assert!(output.status.success());
    let output = run_jjj(dir_path, &["problem", "show", "P-2"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("P2/medium") || stdout.contains("Medium"), "Default priority not shown: {}", stdout);
}

#[test]
fn test_problem_dissolve_reason() {
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();
    run_jjj(dir_path, &["init"]);

    let output = run_jjj(dir_path, &["problem", "new", "Ghost bug"]);
    assert!(output.status.success());

    let output = run_jjj(dir_path, &["problem", "dissolve", "P-1", "--reason", "Test data was stale"]);
    assert!(output.status.success(), "dissolve with reason failed: {}", String::from_utf8_lossy(&output.stderr));

    let output = run_jjj(dir_path, &["problem", "show", "P-1"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dissolved"), "Status not dissolved: {}", stdout);
    assert!(stdout.contains("Test data was stale"), "Dissolved reason not shown: {}", stdout);
}

#[test]
fn test_solution_supersedes() {
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();
    run_jjj(dir_path, &["init"]);

    run_jjj(dir_path, &["problem", "new", "Slow queries"]);
    run_jjj(dir_path, &["solution", "new", "Add index", "--problem", "P-1"]);
    run_jjj(dir_path, &["solution", "refute", "S-1"]);

    let output = run_jjj(dir_path, &["solution", "new", "Use connection pool", "--problem", "P-1", "--supersedes", "S-1"]);
    assert!(output.status.success(), "solution new with supersedes failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Supersedes") || stdout.contains("S-1"), "Supersedes not shown in creation output: {}", stdout);

    let output = run_jjj(dir_path, &["solution", "show", "S-2"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Supersedes") || stdout.contains("S-1"), "Supersedes not shown in show: {}", stdout);
}
