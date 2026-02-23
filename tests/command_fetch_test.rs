mod test_helpers;

use test_helpers::{jj_available, run_jjj, setup_test_repo};

#[test]
fn test_fetch_no_remote_configured_fails() {
    if !jj_available() {
        return;
    }

    // setup_test_repo creates a jj repo with NO remote configured
    let dir = setup_test_repo();

    // Fetch with default remote (origin) should fail since no remote exists
    let output = run_jjj(&dir, &["fetch"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "fetch without a configured remote should fail. stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_fetch_nonexistent_remote_fails() {
    if !jj_available() {
        return;
    }

    let dir = setup_test_repo();

    // Fetch with a named remote that doesn't exist should fail
    let output = run_jjj(&dir, &["fetch", "--remote", "nonexistent"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "fetch from nonexistent remote should fail. stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_fetch_uses_origin_by_default() {
    if !jj_available() {
        return;
    }

    let dir = setup_test_repo();

    // Fetch without --remote flag uses "origin" by default
    // Since there's no remote configured, it should fail with an error
    // referencing "origin"
    let output = run_jjj(&dir, &["fetch"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The command should fail (no origin remote)
    assert!(
        !output.status.success(),
        "fetch with default remote should fail when no origin exists. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Stdout or stderr should mention "origin" since that's the default remote
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("origin"),
        "Error output should reference 'origin' as the default remote. Got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}
