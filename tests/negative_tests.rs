//! Negative tests — verify that invalid operations produce appropriate errors.

mod test_helpers;
use test_helpers::{jj_available, run_jjj, setup_test_repo};

// ── Invalid state transitions ──

#[test]
fn test_problem_solved_to_in_progress_rejected() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);
    run_jjj(
        &dir,
        &["solution", "new", "Fix it", "--problem", "Neg Problem"],
    );
    run_jjj(&dir, &["solution", "submit", "Fix it"]);
    run_jjj(&dir, &["solution", "approve", "Fix it", "--force"]);

    // Solve the problem (now has an approved solution)
    run_jjj(&dir, &["problem", "solve", "Neg Problem"]);

    // Attempt Solved → InProgress via edit
    let output = run_jjj(
        &dir,
        &["problem", "edit", "Neg Problem", "--status", "in_progress"],
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success() || combined.contains("Invalid"),
        "Expected rejection of Solved→InProgress. Got: {}",
        combined
    );
}

#[test]
fn test_solution_approved_to_submitted_rejected() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);
    run_jjj(
        &dir,
        &[
            "solution",
            "new",
            "Neg Solution",
            "--problem",
            "Neg Problem",
        ],
    );
    run_jjj(&dir, &["solution", "submit", "Neg Solution"]);
    run_jjj(&dir, &["solution", "approve", "Neg Solution", "--force"]);

    // Attempt Approved → Submitted via edit
    let output = run_jjj(
        &dir,
        &["solution", "edit", "Neg Solution", "--status", "submitted"],
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success() || combined.contains("Invalid"),
        "Expected rejection of Approved→Submitted. Got: {}",
        combined
    );
}

// ── Nonexistent entity references ──

#[test]
fn test_critique_on_nonexistent_solution_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["critique", "new", "no-such-solution", "A critique"]);
    assert!(
        !output.status.success(),
        "Expected failure when solution doesn't exist"
    );
}

// ── Approve already-approved solution ──

#[test]
fn test_approve_already_approved_solution_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);
    run_jjj(
        &dir,
        &[
            "solution",
            "new",
            "Neg Solution",
            "--problem",
            "Neg Problem",
        ],
    );
    run_jjj(&dir, &["solution", "submit", "Neg Solution"]);
    let output = run_jjj(&dir, &["solution", "approve", "Neg Solution", "--force"]);
    assert!(output.status.success(), "First approve should succeed");

    // Second approve should fail
    let output = run_jjj(&dir, &["solution", "approve", "Neg Solution", "--force"]);
    assert!(
        !output.status.success(),
        "Approving an already-approved solution should fail"
    );
}

// ── Approve with open critiques (no --force) ──

#[test]
fn test_approve_with_open_critiques_without_force_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);
    run_jjj(
        &dir,
        &[
            "solution",
            "new",
            "Neg Solution",
            "--problem",
            "Neg Problem",
        ],
    );
    run_jjj(&dir, &["solution", "submit", "Neg Solution"]);
    run_jjj(
        &dir,
        &[
            "critique",
            "new",
            "Neg Solution",
            "Blocking flaw",
            "--severity",
            "high",
        ],
    );

    // Approve without --force should fail
    let output = run_jjj(&dir, &["solution", "approve", "Neg Solution"]);
    assert!(
        !output.status.success(),
        "Approve without --force should fail when open critiques exist"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("critique") || stderr.contains("open"),
        "Error should mention critiques: {}",
        stderr
    );
}

// ── Invalid enum strings in edit commands ──

#[test]
fn test_invalid_priority_string_rejected() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);

    let output = run_jjj(
        &dir,
        &["problem", "edit", "Neg Problem", "--priority", "ultra"],
    );
    assert!(
        !output.status.success(),
        "Invalid priority 'ultra' should be rejected"
    );
}

#[test]
fn test_invalid_severity_string_rejected() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);
    run_jjj(
        &dir,
        &[
            "solution",
            "new",
            "Neg Solution",
            "--problem",
            "Neg Problem",
        ],
    );

    let output = run_jjj(
        &dir,
        &[
            "critique",
            "new",
            "Neg Solution",
            "Bad severity",
            "--severity",
            "extreme",
        ],
    );
    assert!(
        !output.status.success(),
        "Invalid severity 'extreme' should be rejected"
    );
}

#[test]
fn test_invalid_problem_status_rejected() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj(&dir, &["problem", "new", "Neg Problem"]);

    let output = run_jjj(
        &dir,
        &["problem", "edit", "Neg Problem", "--status", "wontfix"],
    );
    assert!(
        !output.status.success(),
        "Invalid status 'wontfix' should be rejected"
    );
}
