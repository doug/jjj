//! Integration tests for push and fetch commands.
//!
//! These tests create a bare git repo as a "remote" to test actual push/fetch functionality.

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

/// Helper to run jj command
fn run_jj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new("jj")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute jj")
}

/// Create a bare git repo to use as a remote
fn create_bare_remote() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir for bare repo");

    let status = Command::new("git")
        .current_dir(temp_dir.path())
        .args(["init", "--bare"])
        .status()
        .expect("Failed to init bare repo");
    assert!(status.success(), "Failed to create bare git repo");

    temp_dir
}

/// Setup a jj repo with a remote configured
fn setup_repo_with_remote(remote_path: &std::path::Path) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Init jj repo with git colocate
    let status = Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["git", "init", "--colocate"])
        .status()
        .expect("Failed to run jj init");
    assert!(status.success(), "jj init failed");

    // Configure user
    Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .status()
        .expect("Failed to set user.name");
    Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set user.email");

    // Add remote
    let remote_url = format!("file://{}", remote_path.display());
    let status = Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["git", "remote", "add", "origin", &remote_url])
        .status()
        .expect("Failed to add remote");
    assert!(status.success(), "Failed to add git remote");

    temp_dir
}

#[test]
fn test_push_to_bare_remote() {
    if which::which("jj").is_err() {
        eprintln!("Skipping test: jj not found");
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup repo with remote
    let repo_dir = setup_repo_with_remote(remote_dir.path());
    let dir = repo_dir.path();

    // 3. Initialize jjj
    let output = run_jjj(dir, &["init"]);
    assert!(
        output.status.success(),
        "jjj init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 4. Create some entities
    let output = run_jjj(dir, &["problem", "new", "Test Problem for Push"]);
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
            "Test Solution",
            "--problem",
            "Test Problem for Push",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 5. Push to remote
    let output = run_jjj(dir, &["push", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Push stdout: {}", stdout);
    println!("Push stderr: {}", stderr);

    assert!(output.status.success(), "jjj push failed: {}", stderr);
    assert!(
        stdout.contains("Pushing jjj"),
        "Should mention pushing jjj bookmark"
    );

    // 6. Verify the jjj bookmark exists on the remote
    let output = Command::new("git")
        .current_dir(remote_dir.path())
        .args(["branch", "-a"])
        .output()
        .expect("Failed to list branches");
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        branches.contains("jjj"),
        "Remote should have jjj branch. Got: {}",
        branches
    );
}

#[test]
fn test_fetch_from_remote() {
    if which::which("jj").is_err() {
        eprintln!("Skipping test: jj not found");
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup first repo (Alice) and push data
    let alice_dir = setup_repo_with_remote(remote_dir.path());

    run_jjj(alice_dir.path(), &["init"]);
    run_jjj(alice_dir.path(), &["problem", "new", "Shared Problem"]);
    run_jjj(
        alice_dir.path(),
        &[
            "solution",
            "new",
            "Alice Solution",
            "--problem",
            "Shared Problem",
        ],
    );

    let output = run_jjj(alice_dir.path(), &["push", "--remote", "origin"]);
    assert!(
        output.status.success(),
        "Alice push failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. Setup second repo (Bob) pointing to same remote
    let bob_dir = setup_repo_with_remote(remote_dir.path());

    // 4. Initialize jjj for Bob (but don't create any entities yet)
    run_jjj(bob_dir.path(), &["init"]);

    // 5. Fetch from remote
    let output = run_jjj(bob_dir.path(), &["fetch", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Fetch stdout: {}", stdout);
    println!("Fetch stderr: {}", stderr);

    assert!(output.status.success(), "jjj fetch failed: {}", stderr);

    // 6. Verify Bob can see the problem and solution
    let output = run_jjj(bob_dir.path(), &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Shared Problem"),
        "Bob should see Shared Problem after fetch. Got: {}",
        stdout
    );

    let output = run_jjj(bob_dir.path(), &["solution", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Alice Solution"),
        "Bob should see Alice Solution after fetch. Got: {}",
        stdout
    );
}

#[test]
fn test_push_fetch_roundtrip() {
    if which::which("jj").is_err() {
        eprintln!("Skipping test: jj not found");
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup Alice's repo
    let alice_dir = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice_dir.path(), &["init"]);

    // 3. Alice creates a problem and pushes
    run_jjj(alice_dir.path(), &["problem", "new", "Auth timeout bug"]);
    let output = run_jjj(alice_dir.path(), &["push", "--remote", "origin"]);
    assert!(output.status.success(), "Alice initial push failed");

    // 4. Bob clones and fetches
    let bob_dir = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob_dir.path(), &["init"]);
    let output = run_jjj(bob_dir.path(), &["fetch", "--remote", "origin"]);
    assert!(output.status.success(), "Bob fetch failed");

    // 5. Bob adds a solution and pushes
    let output = run_jjj(
        bob_dir.path(),
        &[
            "solution",
            "new",
            "Token refresh fix",
            "--problem",
            "Auth timeout bug",
            "--force",
        ],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Bob solution new stdout: {}", stdout);
    println!("Bob solution new stderr: {}", stderr);
    assert!(
        output.status.success(),
        "Bob solution new failed: {}",
        stderr
    );

    let output = run_jjj(bob_dir.path(), &["push", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Bob push stdout: {}", stdout);
    println!("Bob push stderr: {}", stderr);
    assert!(output.status.success(), "Bob push failed: {}", stderr);

    // 6. Alice fetches Bob's changes
    let output = run_jjj(alice_dir.path(), &["fetch", "--remote", "origin"]);
    assert!(
        output.status.success(),
        "Alice fetch failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 7. Verify Alice sees Bob's solution
    let output = run_jjj(alice_dir.path(), &["solution", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Token refresh fix"),
        "Alice should see Bob's solution after fetch. Got: {}",
        stdout
    );
}

#[test]
fn test_push_dry_run() {
    if which::which("jj").is_err() {
        eprintln!("Skipping test: jj not found");
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup repo
    let repo_dir = setup_repo_with_remote(remote_dir.path());
    let dir = repo_dir.path();

    run_jjj(dir, &["init"]);
    run_jjj(dir, &["problem", "new", "Dry Run Test"]);

    // 3. Push with --dry-run
    let output = run_jjj(dir, &["push", "--remote", "origin", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Dry run should succeed");
    assert!(
        stdout.contains("Would push"),
        "Should indicate what would be pushed. Got: {}",
        stdout
    );

    // 4. Verify nothing was actually pushed
    let output = Command::new("git")
        .current_dir(remote_dir.path())
        .args(["branch", "-a"])
        .output()
        .expect("Failed to list branches");
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        !branches.contains("jjj"),
        "Dry run should not push. Remote branches: {}",
        branches
    );
}

#[test]
fn test_push_validates_before_pushing() {
    if which::which("jj").is_err() {
        eprintln!("Skipping test: jj not found");
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup repo
    let repo_dir = setup_repo_with_remote(remote_dir.path());
    let dir = repo_dir.path();

    let output = run_jjj(dir, &["init"]);
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_jjj(dir, &["problem", "new", "Validation Test"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. Push should validate and succeed
    let output = run_jjj(dir, &["push", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Push stdout: {}", stdout);
    println!("Push stderr: {}", stderr);

    assert!(output.status.success(), "Push failed: {}", stderr);
    assert!(
        stdout.contains("Validating") || stdout.contains("checks passed"),
        "Should show validation. Got: {}",
        stdout
    );
}
