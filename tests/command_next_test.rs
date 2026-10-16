mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn test_next_basic() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);

    let stdout = run_jjj_success(&dir, &["next"]);
    assert!(
        stdout.contains("TODO") || stdout.contains("Problem"),
        "Expected next action: {}",
        stdout
    );
}

#[test]
fn test_next_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next"]);
    assert!(
        stdout.contains("Nothing to do") || stdout.contains("all caught up"),
        "Expected empty message: {}",
        stdout
    );
}

#[test]
fn test_next_claim() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Claimable Problem"]);

    let stdout = run_jjj_success(&dir, &["next", "--claim"]);
    assert!(
        stdout.contains("Claimed"),
        "Expected 'Claimed' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("Claimable Problem"),
        "Expected problem title in output: {}",
        stdout
    );

    // Verify assignment by checking the problem
    let show_stdout = run_jjj_success(&dir, &["problem", "show", "Claimable", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&show_stdout).expect("Failed to parse JSON");
    assert!(
        json["assignee"].is_string(),
        "Expected assignee to be set after claim: {:?}",
        json["assignee"]
    );
}

#[test]
fn test_next_claim_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON Claim"]);

    let stdout = run_jjj_success(&dir, &["next", "--claim", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(
        json["title"].as_str().unwrap_or("").contains("JSON Claim"),
        "Expected title in JSON output: {}",
        stdout
    );
}

#[test]
fn test_next_claim_empty() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next", "--claim"]);
    assert!(
        stdout.contains("Nothing to claim"),
        "Expected 'Nothing to claim' on empty repo: {}",
        stdout
    );
}

#[test]
fn test_next_claim_empty_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next", "--claim", "--json"]);
    assert!(
        stdout.trim() == "null",
        "Expected null JSON on empty repo: {}",
        stdout
    );
}

/// A problem waiting on unfinished sub-problems must not be offered as work.
///
/// Before this rule, `jjj next` ranked such a parent by its own priority and
/// handed it over with every child still open — demonstrably the top of the
/// queue in a three-problem repository, with nothing anywhere saying it was not
/// ready. The agent then either flounders or silently does the children's work
/// without recording it against them.
#[test]
fn next_does_not_offer_a_parent_waiting_on_its_children() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let parent = {
        let out = run_jjj_success(
            &dir,
            &[
                "problem",
                "new",
                "Ship the release",
                "--priority",
                "critical",
                "--json",
            ],
        );
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        v["id"].as_str().expect("id").to_string()
    };
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Fix the failing test",
            "--parent",
            &parent,
            "--priority",
            "low",
        ],
    );

    let queue = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        queue.contains("Fix the failing test"),
        "the child is actionable and must be offered: {queue}"
    );
    assert!(
        !queue.contains("Ship the release"),
        "the waiting parent must not be offered, even at critical priority: {queue}"
    );

    // And it comes back once the child is finished, or the parent is lost.
    run_jjj_success(
        &dir,
        &[
            "problem",
            "dissolve",
            "Fix the failing test",
            "--rationale",
            "moot",
        ],
    );
    let after = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        after.contains("Ship the release"),
        "the parent must return to the queue once nothing blocks it: {after}"
    );
}

/// Dropping work from the queue silently is its own failure, so it has to be
/// visible somewhere.
#[test]
fn a_waiting_parent_is_discoverable() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let parent = {
        let out = run_jjj_success(&dir, &["problem", "new", "Epic", "--json"]);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        v["id"].as_str().expect("id").to_string()
    };
    run_jjj_success(&dir, &["problem", "new", "Step one", "--parent", &parent]);

    let shown = run_jjj_success(&dir, &["problem", "show", &parent]);
    assert!(
        shown.contains("Waiting on 1 unfinished sub-problem"),
        "`problem show` must say why it is not offered: {shown}"
    );

    let waiting = run_jjj_success(&dir, &["query", "waiting"]);
    assert!(
        waiting.contains("Epic") && !waiting.contains("Step one"),
        "`jjj query waiting` must list exactly the held-back parents: {waiting}"
    );
}
