//! `edit --if-lamport` — refuse a write when the entity moved underneath you.
//!
//! `edit --body` replaces a body wholesale, so an agent that writes one without
//! having seen the current text destroys whatever another agent put there. In a
//! fleet, "I read this three minutes ago" is not the same as "this is what it
//! says now", and nothing in the CLI used to be able to tell the difference.
//!
//! The Lamport clock each entity already carries is exactly the token needed:
//! read it from `show --json`, pass it back with the write, and the write is
//! refused if anything intervened. beads reaches for the same primitive
//! (`--if-revision`, backed by a `row_lock` compare-and-set) and is careful about
//! what it buys — even on a real database its docs call the compare-and-set "a
//! correctness fence against concurrent claims, not an authorization boundary".
//! Here it is a fence within one clone; two clones can still both pass it and
//! merge afterwards, which is what the merge is for.
//!
//! The flag is optional and omitting it changes nothing, so every existing
//! invocation keeps working — the guard is available to callers that want it.

mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

fn id_of(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v["id"].as_str().expect("id field").to_string()
}

fn first_id(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v[0]["id"].as_str().expect("id field").to_string()
}

/// The clock as a caller would read it. Absent means zero, which is what a
/// freshly created entity looks like.
fn clock(dir: &std::path::Path, kind: &str, id: &str) -> u64 {
    let json = run_jjj_success(dir, &[kind, "show", id, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    v["lamport"].as_u64().unwrap_or(0)
}

#[test]
fn a_matching_clock_lets_the_write_through() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let pid = id_of(&run_jjj_success(
        dir.path(),
        &[
            "problem",
            "new",
            "Decode is slow",
            "--body",
            "First.",
            "--json",
        ],
    ));
    let seen = clock(dir.path(), "problem", &pid);

    run_jjj_success(
        dir.path(),
        &[
            "problem",
            "edit",
            &pid,
            "--body",
            "Second.",
            "--if-lamport",
            &seen.to_string(),
        ],
    );

    let json = run_jjj_success(dir.path(), &["problem", "show", &pid, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(v["description"].as_str(), Some("Second."));
}

#[test]
fn a_stale_clock_is_refused_and_the_body_survives() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let pid = id_of(&run_jjj_success(
        dir.path(),
        &[
            "problem",
            "new",
            "Decode is slow",
            "--body",
            "First.",
            "--json",
        ],
    ));
    let seen = clock(dir.path(), "problem", &pid);

    // Another agent gets there first.
    run_jjj_success(
        dir.path(),
        &[
            "problem",
            "edit",
            &pid,
            "--body",
            "Theirs, carefully measured.",
        ],
    );

    let out = run_jjj(
        dir.path(),
        &[
            "problem",
            "edit",
            &pid,
            "--body",
            "Mine, written blind.",
            "--if-lamport",
            &seen.to_string(),
        ],
    );
    assert!(
        !out.status.success(),
        "a write against a stale clock was allowed through"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("has changed since you read it"),
        "the refusal must say what happened: {stderr}"
    );
    // Both clock values, so the caller can tell how far behind it was.
    assert!(
        stderr.contains("you expected"),
        "the refusal must name the clock the caller passed: {stderr}"
    );

    let json = run_jjj_success(dir.path(), &["problem", "show", &pid, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(
        v["description"].as_str(),
        Some("Theirs, carefully measured."),
        "the other agent's body was clobbered by a refused write"
    );
}

#[test]
fn omitting_the_guard_changes_nothing() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let pid = id_of(&run_jjj_success(
        dir.path(),
        &["problem", "new", "Unguarded", "--body", "First.", "--json"],
    ));
    // The flag is additive: every existing invocation must keep working, however
    // far the clock has moved.
    run_jjj_success(dir.path(), &["problem", "edit", &pid, "--body", "Second."]);
    run_jjj_success(dir.path(), &["problem", "edit", &pid, "--body", "Third."]);

    let json = run_jjj_success(dir.path(), &["problem", "show", &pid, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(v["description"].as_str(), Some("Third."));
}

/// All five kinds, because a guard that covers four of them is a guard an agent
/// cannot rely on.
#[test]
fn every_entity_kind_honours_the_guard() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    let pid = id_of(&run_jjj_success(
        p,
        &["problem", "new", "A problem", "--json"],
    ));
    let sid = id_of(&run_jjj_success(
        p,
        &["solution", "new", "A solution", "--problem", &pid, "--json"],
    ));
    let cid = id_of(&run_jjj_success(
        p,
        &["critique", "new", &sid, "A critique", "--json"],
    ));
    let fid = id_of(&run_jjj_success(
        p,
        &["finding", "new", &pid, "A finding", "--json"],
    ));
    run_jjj_success(p, &["milestone", "new", "A milestone"]);
    let mid = first_id(&run_jjj_success(p, &["milestone", "list", "--json"]));

    for (kind, id) in [
        ("problem", &pid),
        ("solution", &sid),
        ("critique", &cid),
        ("finding", &fid),
        ("milestone", &mid),
    ] {
        // Move the entity, then write against the clock from before the move.
        let before = clock(p, kind, id);
        run_jjj_success(p, &[kind, "edit", id, "--title", "Moved on"]);

        let out = run_jjj(
            p,
            &[
                kind,
                "edit",
                id,
                "--title",
                "Written blind",
                "--if-lamport",
                &before.to_string(),
            ],
        );
        assert!(
            !out.status.success(),
            "{kind}: a stale guarded write was allowed through"
        );

        // And the current clock still lets a write through, so the guard is a
        // fence rather than a lock that never opens.
        let now = clock(p, kind, id);
        run_jjj_success(
            p,
            &[
                kind,
                "edit",
                id,
                "--title",
                "Written with a fresh read",
                "--if-lamport",
                &now.to_string(),
            ],
        );
    }
}

/// The refusal must not teach the agent how to defeat the guard.
///
/// A fence that fires and then says "or drop the flag to overwrite" is a fence
/// an agent will step over, because the agent is trying to finish its task and
/// has just been handed the way to. beads wrote this lesson down about its own
/// migration refusal: migrating is one-way, so the override "stays out of this
/// payload entirely" and the error carries only the non-runnable decision.
///
/// jjj's overwrite is not irreversible — the merge and the event log see to that
/// — but it does discard another agent's text, and the refusal is the one moment
/// where that can be said. So the message names the remedy that is always right
/// (re-read, reapply) and describes the overwrite as a decision rather than a
/// retry.
#[test]
fn the_refusal_does_not_hand_over_a_way_to_defeat_it() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let pid = id_of(&run_jjj_success(
        dir.path(),
        &["problem", "new", "Contested", "--body", "First.", "--json"],
    ));
    let seen = clock(dir.path(), "problem", &pid);
    run_jjj_success(dir.path(), &["problem", "edit", &pid, "--body", "Theirs."]);

    let out = run_jjj(
        dir.path(),
        &[
            "problem",
            "edit",
            &pid,
            "--body",
            "Mine.",
            "--if-lamport",
            &seen.to_string(),
        ],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !stderr.contains("drop --if-lamport"),
        "the refusal offers the override as the fix: {stderr}"
    );
    // The remedy it does give must be the correct one, and runnable.
    assert!(
        stderr.contains("jjj problem show"),
        "the refusal must name how to re-read the entity: {stderr}"
    );
    assert!(
        stderr.contains("decision about someone else's work"),
        "the refusal must frame an unguarded write as a decision: {stderr}"
    );
}

/// A guard is only usable in a loop if the caller can tell "re-read and retry"
/// from "your command was wrong" without matching on prose.
///
/// jjj exited 1 for every failure, so `--if-lamport` — shipped in this same
/// effort — could not be acted on programmatically: an agent had to string-match
/// stderr, and the wording of that very refusal was rewritten hours later, which
/// would have broken anyone doing it. beads reserves 13 for the same meaning
/// (`if_revision.go:19`); there is no reason two agent-facing tools should
/// disagree on the number.
#[test]
fn a_failed_precondition_has_its_own_exit_code() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj_success(&dir, &["problem", "new", "measure the hot path", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let id = v["id"].as_str().expect("id").to_string();

    // A stale read: nothing is written, and a re-read would let it through.
    let stale = run_jjj(
        &dir,
        &[
            "problem",
            "edit",
            &id,
            "--body",
            "rewritten",
            "--if-lamport",
            "9999",
        ],
    );
    assert_eq!(
        stale.status.code(),
        Some(13),
        "a stale-read refusal must be distinguishable: {}",
        String::from_utf8_lossy(&stale.stderr)
    );

    // A malformed call is *not* retryable, and must not share the code — this
    // is the distinction the whole thing rests on.
    let bad = run_jjj(&dir, &["problem", "edit", &id, "--body", ""]);
    assert_eq!(
        bad.status.code(),
        Some(1),
        "an empty body is the caller's bug, not a race: {}",
        String::from_utf8_lossy(&bad.stderr)
    );

    let missing = run_jjj(&dir, &["problem", "show", "nosuchentity"]);
    assert_eq!(
        missing.status.code(),
        Some(1),
        "a bad reference is not a race"
    );

    // And the guard still passes when the clock matches, or 13 would just mean
    // "guarded" rather than "the world moved".
    let v2: serde_json::Value =
        serde_json::from_str(&run_jjj_success(&dir, &["problem", "show", &id, "--json"]))
            .expect("valid JSON");
    let clock = v2["lamport"].as_u64().unwrap_or(0);
    let ok = run_jjj(
        &dir,
        &[
            "problem",
            "edit",
            &id,
            "--body",
            "rewritten",
            "--if-lamport",
            &clock.to_string(),
        ],
    );
    assert!(
        ok.status.success(),
        "a current clock must still write: {}",
        String::from_utf8_lossy(&ok.stderr)
    );
}

/// A live claim is the other member of the retryable class: well-formed command,
/// nothing written, and it succeeds once the lease lapses.
#[test]
fn a_live_claim_refusal_is_also_a_precondition() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj_success(&dir, &["problem", "new", "contended work", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let id = v["id"].as_str().expect("id").to_string();

    run_jjj_success(&dir, &["problem", "assign", &id, "--to", "agent-a"]);
    run_jjj_success(&dir, &["next", "--claim"]);

    let contested = run_jjj(&dir, &["problem", "assign", &id, "--to", "agent-b"]);
    if !contested.status.success() {
        assert_eq!(
            contested.status.code(),
            Some(13),
            "a live claim is retryable after the TTL, so it is a precondition: {}",
            String::from_utf8_lossy(&contested.stderr)
        );
    }
}
