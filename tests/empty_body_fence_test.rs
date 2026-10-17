//! `--body` with no content must not silently erase an entity's content.
//!
//! `edit --body` replaces a body wholesale, and an empty string is exactly what
//! a dead command substitution collapses to: `--body "$(cat notes.md)"` with the
//! file missing, or `--body -` reading a closed stdin. Before this fence that
//! erased the entity's entire content and reported success — measured, on a
//! problem whose description went from "Three weeks of measurements." to the
//! empty string, exit 0, `Updated problem`.
//!
//! At the flag layer the accident and a deliberate clear are the same bytes, so
//! the value is refused and clearing gets its own flag. This is beads' `--notes
//! ""` guard (`validateNotesUpdate`), which reasons identically: "at the flag
//! layer the accident and the deliberate clear are the same bytes, so the empty
//! value is refused outright and the deliberate clear gets its own verb".
//!
//! `new` is deliberately unguarded — creating an entity with no body is ordinary,
//! and there is nothing there to lose.

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

fn body_of(dir: &std::path::Path, kind: &str, id: &str, field: &str) -> String {
    let json = run_jjj_success(dir, &[kind, "show", id, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    v[field].as_str().unwrap_or_default().to_string()
}

/// Every kind, with the body field each one calls its own. A fence that covers
/// four of five is a fence nobody can rely on.
fn kinds(dir: &std::path::Path) -> Vec<(&'static str, String, &'static str)> {
    let pid = id_of(&run_jjj_success(
        dir,
        &[
            "problem",
            "new",
            "A problem",
            "--body",
            "Problem text.",
            "--json",
        ],
    ));
    let sid = id_of(&run_jjj_success(
        dir,
        &[
            "solution",
            "new",
            "A solution",
            "--problem",
            &pid,
            "--body",
            "Solution text.",
            "--json",
        ],
    ));
    let cid = id_of(&run_jjj_success(
        dir,
        &[
            "critique",
            "new",
            &sid,
            "A critique",
            "--body",
            "Critique text.",
            "--json",
        ],
    ));
    let fid = id_of(&run_jjj_success(
        dir,
        &[
            "finding",
            "new",
            &pid,
            "A finding",
            "--body",
            "Finding text.",
            "--json",
        ],
    ));
    run_jjj_success(
        dir,
        &[
            "milestone",
            "new",
            "A milestone",
            "--body",
            "Milestone text.",
        ],
    );
    let mid = first_id(&run_jjj_success(dir, &["milestone", "list", "--json"]));

    vec![
        ("problem", pid, "description"),
        ("solution", sid, "approach"),
        ("critique", cid, "argument"),
        ("finding", fid, "evidence"),
        ("milestone", mid, "description"),
    ]
}

#[test]
fn an_empty_body_is_refused_and_the_text_survives() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    for (kind, id, field) in kinds(p) {
        let before = body_of(p, kind, &id, field);
        assert!(!before.is_empty(), "{kind}: fixture has no body to lose");

        let out = run_jjj(p, &[kind, "edit", &id, "--body", ""]);
        assert!(
            !out.status.success(),
            "{kind}: an empty --body was accepted and erased the {field}"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--clear-body"),
            "{kind}: the refusal must name the deliberate way to do it: {stderr}"
        );
        assert!(
            stderr.contains(field),
            "{kind}: the refusal must say which field was at risk: {stderr}"
        );
        assert_eq!(
            body_of(p, kind, &id, field),
            before,
            "{kind}: the {field} changed despite a refused write"
        );
    }
}

/// Whitespace is the same accident — `$(echo)` yields a newline, not nothing.
#[test]
fn a_whitespace_only_body_is_refused_too() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    let pid = id_of(&run_jjj_success(
        p,
        &[
            "problem",
            "new",
            "Measured",
            "--body",
            "Real content.",
            "--json",
        ],
    ));

    let out = run_jjj(p, &["problem", "edit", &pid, "--body", "   \n  "]);
    assert!(!out.status.success(), "whitespace was accepted as a body");
    assert_eq!(body_of(p, "problem", &pid, "description"), "Real content.");
}

#[test]
fn clear_body_erases_deliberately_on_every_kind() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    for (kind, id, field) in kinds(p) {
        run_jjj_success(p, &[kind, "edit", &id, "--clear-body"]);
        assert_eq!(
            body_of(p, kind, &id, field),
            "",
            "{kind}: --clear-body did not clear the {field}"
        );
    }
}

/// The fence must not cost the ordinary case anything.
#[test]
fn a_real_body_still_replaces_the_old_one() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    for (kind, id, field) in kinds(p) {
        run_jjj_success(p, &[kind, "edit", &id, "--body", "Rewritten."]);
        assert_eq!(
            body_of(p, kind, &id, field),
            "Rewritten.",
            "{kind}: a legitimate --body did not land"
        );
    }
}

/// Creation is exempt: an entity with no body yet is normal, and nothing is
/// being destroyed.
#[test]
fn creating_with_an_empty_body_is_still_allowed() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    let out = run_jjj(p, &["problem", "new", "No body yet", "--body", ""]);
    assert!(
        out.status.success(),
        "creating with an empty body was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Asking for both is contradictory, and clap refuses it before any write.
#[test]
fn body_and_clear_body_together_are_refused() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    let pid = id_of(&run_jjj_success(
        p,
        &[
            "problem",
            "new",
            "Contradiction",
            "--body",
            "Keep me.",
            "--json",
        ],
    ));

    let out = run_jjj(p, &["problem", "edit", &pid, "--body", "x", "--clear-body"]);
    assert!(!out.status.success(), "a contradictory pair was accepted");
    assert_eq!(body_of(p, "problem", &pid, "description"), "Keep me.");
}
