//! End-to-end integration tests for GitHub sync against a real private repo (doug/jjjtest).
//!
//! These tests are gated behind `gh auth status` and `which jj` checks.
//! They use unique prefixes per run to avoid conflicts and clean up after themselves.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

const TEST_REPO: &str = "doug/jjjtest";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the jjj binary path.
fn jjj_binary() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/jjj");
    if !path.exists() {
        panic!(
            "jjj binary not found at {:?}. Make sure to build first.",
            path
        );
    }
    path
}

/// Run jjj in the given directory with the given args.
fn run_jjj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(&jjj_binary())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute jjj")
}

/// Run jjj and assert success, returning stdout.
fn run_jjj_success(dir: &std::path::Path, args: &[&str]) -> String {
    let output = run_jjj(dir, args);
    assert!(
        output.status.success(),
        "jjj {} failed.\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run an arbitrary command and return its Output.
fn run_cmd(dir: &std::path::Path, program: &str, args: &[&str]) -> std::process::Output {
    Command::new(program)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute {} {}: {}", program, args.join(" "), e))
}

/// Run gh CLI targeting the test repo. Returns stdout on success.
fn run_gh(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("gh")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute gh");
    assert!(
        output.status.success(),
        "gh {} failed.\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Generate a unique suffix for this test run (epoch millis).
fn unique_suffix() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}", ts)
}

/// Create a GitHub issue on the test repo, return the issue number.
/// Parses the issue number from the URL that `gh issue create` prints.
fn create_gh_issue(dir: &std::path::Path, title: &str, body: &str) -> u64 {
    let url = run_gh(
        dir,
        &[
            "issue",
            "create",
            "--repo",
            TEST_REPO,
            "--title",
            title,
            "--body",
            body,
        ],
    );
    // Output is a URL like https://github.com/doug/jjjtest/issues/42
    url.trim()
        .rsplit('/')
        .next()
        .expect("Failed to parse issue number from URL")
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("Failed to parse issue number from URL: {}", url))
}

/// Check prerequisites: jj available, gh available and authenticated.
/// Returns false if any check fails (test should skip).
fn prerequisites_met() -> bool {
    // jj installed?
    if jjj::jj::find_executable("jj").is_none() {
        eprintln!("Skipping github_sync_e2e: jj not found");
        return false;
    }

    // gh installed?
    if jjj::jj::find_executable("gh").is_none() {
        eprintln!("Skipping github_sync_e2e: gh not found");
        return false;
    }

    // gh authenticated?
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .expect("Failed to run gh auth status");
    if !output.status.success() {
        eprintln!("Skipping github_sync_e2e: gh not authenticated");
        return false;
    }

    true
}

/// Set up a temp jj repo (colocated with git), init jjj, add remote for doug/jjjtest.
fn setup_github_test_repo() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let path = dir.path();

    // jj git init --colocate
    let output = run_cmd(path, "jj", &["git", "init", "--colocate"]);
    assert!(
        output.status.success(),
        "jj git init --colocate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Configure user
    run_cmd(
        path,
        "jj",
        &["config", "set", "--repo", "user.name", "E2E Test"],
    );
    run_cmd(
        path,
        "jj",
        &["config", "set", "--repo", "user.email", "e2e@test.local"],
    );

    // Add git remote pointing to the test repo
    let remote_url = format!("https://github.com/{}.git", TEST_REPO);
    let output = run_cmd(
        path,
        "jj",
        &["git", "remote", "add", "origin", &remote_url],
    );
    assert!(
        output.status.success(),
        "Failed to add remote: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Initialize jjj
    let stdout = run_jjj_success(path, &["init"]);
    assert!(
        stdout.contains("Initialized") || stdout.contains("jjj"),
        "jjj init output unexpected: {}",
        stdout
    );

    dir
}

/// Close a GitHub issue by number on the test repo (cleanup helper).
fn close_issue_on_repo(dir: &std::path::Path, number: u64) {
    let num_str = number.to_string();
    let _ = Command::new("gh")
        .current_dir(dir)
        .args([
            "issue",
            "close",
            &num_str,
            "--repo",
            TEST_REPO,
        ])
        .output();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full end-to-end: create local problem, create GH issue, import, close, reopen, status, cleanup.
#[test]
fn test_github_sync_e2e_full_flow() {
    if !prerequisites_met() {
        return;
    }

    let suffix = unique_suffix();
    let dir = setup_github_test_repo();
    let path = dir.path();

    // Track issue numbers we create so we can clean up on any failure path.
    let mut created_issue_numbers: Vec<u64> = Vec::new();

    // Helper closure-style cleanup (we'll call at the end and on panics via Drop).
    // We do explicit cleanup at the end; the Drop guard is just in case.
    struct CleanupGuard {
        dir: PathBuf,
        issues: Vec<u64>,
    }
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            for &num in &self.issues {
                close_issue_on_repo(&self.dir, num);
            }
        }
    }

    let mut guard = CleanupGuard {
        dir: path.to_path_buf(),
        issues: Vec::new(),
    };

    // -----------------------------------------------------------------------
    // Step 1: Create a local problem
    // -----------------------------------------------------------------------
    let problem_title = format!("E2E Test Problem {}", suffix);
    let stdout = run_jjj_success(path, &["problem", "new", &problem_title]);
    assert!(
        stdout.contains(&problem_title),
        "Expected title in output: {}",
        stdout
    );

    // -----------------------------------------------------------------------
    // Step 2: Create a GitHub issue directly via gh, then import it
    // -----------------------------------------------------------------------
    let gh_issue_title = format!("GH E2E Import Test {}", suffix);
    let gh_issue_body = "Automated test issue created by github_sync_e2e_test. Safe to delete.";

    // Create issue on the real repo
    let issue_number = create_gh_issue(path, &gh_issue_title, gh_issue_body);
    created_issue_numbers.push(issue_number);
    guard.issues.push(issue_number);

    eprintln!(
        "Created GitHub issue #{} on {}",
        issue_number, TEST_REPO
    );

    // Import the issue via jjj sync github import
    let issue_ref = format!("#{}", issue_number);
    let stdout = run_jjj_success(path, &["sync", "github", "import", &issue_ref]);
    assert!(
        stdout.contains("Imported") || stdout.contains(&gh_issue_title),
        "Expected import confirmation: {}",
        stdout
    );

    // -----------------------------------------------------------------------
    // Step 3: Verify the imported problem has github_issue set
    // -----------------------------------------------------------------------
    // List problems in JSON and find the imported one
    let stdout = run_jjj_success(path, &["problem", "list", "--json"]);
    let problems: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse problem list JSON");
    let problems_arr = problems.as_array().expect("Expected JSON array");

    let imported = problems_arr
        .iter()
        .find(|p| {
            p["title"]
                .as_str()
                .map_or(false, |t| t.contains(&gh_issue_title) || t == gh_issue_title)
        })
        .expect("Imported problem not found in problem list");

    let imported_id = imported["id"].as_str().expect("Expected id string");
    let github_issue_field = imported.get("github_issue");
    assert!(
        github_issue_field.is_some()
            && github_issue_field.unwrap().as_u64() == Some(issue_number),
        "Expected github_issue={} on imported problem, got {:?}",
        issue_number,
        github_issue_field
    );

    // Also verify via `problem show --json`
    let stdout = run_jjj_success(path, &["problem", "show", imported_id, "--json"]);
    let show_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse problem show JSON");
    assert_eq!(
        show_json["github_issue"].as_u64(),
        Some(issue_number),
        "problem show --json should have github_issue={}",
        issue_number
    );

    // -----------------------------------------------------------------------
    // Step 4: Close the issue via jjj sync github close
    // -----------------------------------------------------------------------
    let stdout = run_jjj_success(path, &["sync", "github", "close", imported_id]);
    assert!(
        stdout.contains("Closed") || stdout.contains("close"),
        "Expected close confirmation: {}",
        stdout
    );

    // Verify the issue is closed on GitHub
    let gh_state = run_gh(
        path,
        &[
            "issue",
            "view",
            &issue_number.to_string(),
            "--repo",
            TEST_REPO,
            "--json",
            "state",
            "--jq",
            ".state",
        ],
    );
    assert_eq!(
        gh_state.trim(),
        "CLOSED",
        "Issue #{} should be CLOSED on GitHub, got: {}",
        issue_number,
        gh_state
    );

    // -----------------------------------------------------------------------
    // Step 5: Reopen the issue via jjj sync github reopen
    // -----------------------------------------------------------------------
    let stdout = run_jjj_success(path, &["sync", "github", "reopen", imported_id]);
    assert!(
        stdout.contains("Reopened") || stdout.contains("reopen"),
        "Expected reopen confirmation: {}",
        stdout
    );

    // Verify the issue is open on GitHub
    let gh_state = run_gh(
        path,
        &[
            "issue",
            "view",
            &issue_number.to_string(),
            "--repo",
            TEST_REPO,
            "--json",
            "state",
            "--jq",
            ".state",
        ],
    );
    assert_eq!(
        gh_state.trim(),
        "OPEN",
        "Issue #{} should be OPEN after reopen, got: {}",
        issue_number,
        gh_state
    );

    // -----------------------------------------------------------------------
    // Step 6: Check sync status
    // -----------------------------------------------------------------------
    let stdout = run_jjj_success(path, &["sync", "github", "status"]);
    // Status should mention the repo and the linked issue
    assert!(
        stdout.contains("jjjtest") || stdout.contains(TEST_REPO),
        "Status should mention the repo: {}",
        stdout
    );
    assert!(
        stdout.contains(&format!("#{}", issue_number))
            || stdout.contains(&gh_issue_title),
        "Status should mention the linked issue: {}",
        stdout
    );

    // -----------------------------------------------------------------------
    // Step 7: Cleanup - close all test issues
    // -----------------------------------------------------------------------
    for &num in &created_issue_numbers {
        close_issue_on_repo(path, num);
    }

    // Verify cleanup: issue should be closed
    let gh_state = run_gh(
        path,
        &[
            "issue",
            "view",
            &issue_number.to_string(),
            "--repo",
            TEST_REPO,
            "--json",
            "state",
            "--jq",
            ".state",
        ],
    );
    assert_eq!(
        gh_state.trim(),
        "CLOSED",
        "Cleanup: issue #{} should be CLOSED, got: {}",
        issue_number,
        gh_state
    );

    // Clear guard so it doesn't double-close
    guard.issues.clear();
}

/// Test: creating a local problem does NOT auto-create a GH issue (auto_push is off by default).
#[test]
fn test_github_sync_local_problem_no_auto_push() {
    if !prerequisites_met() {
        return;
    }

    let suffix = unique_suffix();
    let dir = setup_github_test_repo();
    let path = dir.path();

    let title = format!("Local Only E2E {}", suffix);
    run_jjj_success(path, &["problem", "new", &title]);

    // The problem should NOT have a github_issue (auto_push is off)
    let stdout = run_jjj_success(path, &["problem", "list", "--json"]);
    let problems: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON");
    let problems_arr = problems.as_array().unwrap();

    let local = problems_arr
        .iter()
        .find(|p| p["title"].as_str().map_or(false, |t| t == title))
        .expect("Local problem not found");

    let gh_issue = local.get("github_issue");
    assert!(
        gh_issue.is_none() || gh_issue.unwrap().is_null(),
        "Local-only problem should not have github_issue, got: {:?}",
        gh_issue
    );
}

/// Test: importing an already-imported issue is idempotent.
#[test]
fn test_github_sync_import_idempotent() {
    if !prerequisites_met() {
        return;
    }

    let suffix = unique_suffix();
    let dir = setup_github_test_repo();
    let path = dir.path();

    // Create a GH issue
    let title = format!("Idempotent Import E2E {}", suffix);
    let body = "Automated test issue for idempotency check. Safe to delete.";

    let issue_number = create_gh_issue(path, &title, body);

    struct CleanupGuard {
        dir: PathBuf,
        issue: u64,
    }
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            close_issue_on_repo(&self.dir, self.issue);
        }
    }
    let _guard = CleanupGuard {
        dir: path.to_path_buf(),
        issue: issue_number,
    };

    let issue_ref = format!("#{}", issue_number);

    // First import
    let stdout = run_jjj_success(path, &["sync", "github", "import", &issue_ref]);
    assert!(
        stdout.contains("Imported"),
        "First import should say Imported: {}",
        stdout
    );

    // Second import should be a no-op (already linked)
    let stdout = run_jjj_success(path, &["sync", "github", "import", &issue_ref]);
    assert!(
        stdout.contains("already linked") || stdout.contains("already"),
        "Second import should report already linked: {}",
        stdout
    );

    // Should still have exactly one problem with that github_issue
    let stdout = run_jjj_success(path, &["problem", "list", "--json"]);
    let problems: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let matching: Vec<_> = problems
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["github_issue"].as_u64() == Some(issue_number))
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "Expected exactly 1 problem linked to issue #{}, got {}",
        issue_number,
        matching.len()
    );
}
