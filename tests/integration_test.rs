use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run the jjj binary
fn run_jjj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let debug_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/jjj");
    // Ensure binary exists
    if !debug_dir.exists() {
        // Fallback or panic? For tests, we assume cargo test builds binaries or we can use cargo run
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
fn test_init_and_create_task() {
    // Skip if jj is not installed
    if which::which("jj").is_err() {
        return;
    }

    let temp_dir = setup_test_repo();
    let dir_path = temp_dir.path();

    // 1. Run jjj init
    let output = run_jjj(dir_path, &["init"]);
    assert!(output.status.success(), "jjj init failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify jjj/meta bookmark exists (this might fail if we query improperly, but let's check file structure if possible)
    // In a real integration test we'd query jj, but let's trust jjj internal checks for now or check if .jj directory is modified?
    // Proper way: check if jjj commands work now.

    // 2. Create a feature
    let output = run_jjj(dir_path, &["feature", "new", "Integration Feature"]);
    assert!(output.status.success(), "feature new failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Assuming feature creation output contains the ID, e.g. "Created feature F-1"
    // We can grep it or just blindly use F-1 if it's the first one.
    // Let's assume F-1 for now or list features to get it.
    
    // 3. Create a task associated with the feature
    let output = run_jjj(dir_path, &["task", "new", "Test Integration Task", "--feature", "F-1", "--tag", "integration"]);
    assert!(output.status.success(), "task new failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // 4. List tasks and verify
    let output = run_jjj(dir_path, &["task", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Integration Task"));
    assert!(stdout.contains("integration"));
    // F-1 is not shown in list view
    
    // 5. Show task details to verify feature
    let output = run_jjj(dir_path, &["task", "show", "T-1"]);
    assert!(output.status.success(), "task show failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Feature ID should be present (assuming F-1 is the first feature)
    assert!(stdout.contains("F-1"));
}
