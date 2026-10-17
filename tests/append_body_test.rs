//! Adding to a body without rewriting it.
//!
//! Every body edit used to be a replacement, so an agent adding a line had to
//! read the body, build the new text, and write the whole thing back. Anything
//! written between its read and its write was discarded without a word, and
//! `--if-lamport` could only turn that into a refusal after the fact — it tells
//! you the write failed, it does not keep the text.
//!
//! `--append-body` moves the read inside the write transaction, so there is no
//! window to lose an edit in. It does not make concurrent appends from two
//! clones safe: the body is one key in the per-field merge, so one side's
//! addition still wins. That is a smaller problem than the one it removes, and
//! these tests pin the part that is actually fixed rather than implying more.

mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

fn body_of(json: &str, field: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v[field].as_str().unwrap_or_default().to_string()
}

#[test]
fn an_append_keeps_what_was_there() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Decode is slow",
            "--body",
            "First measurement: 40ms.",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "problem",
            "edit",
            "Decode is slow",
            "--append-body",
            "Second measurement: 38ms.",
        ],
    );

    let shown = run_jjj_success(&dir, &["problem", "show", "Decode is slow", "--json"]);
    let body = body_of(&shown, "description");
    assert_eq!(
        body, "First measurement: 40ms.\n\nSecond measurement: 38ms.",
        "both measurements must survive, separated by a blank line"
    );
}

#[test]
fn appending_to_an_empty_body_does_not_leave_leading_blank_lines() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "No body yet"]);
    run_jjj_success(
        &dir,
        &[
            "problem",
            "edit",
            "No body yet",
            "--append-body",
            "First note.",
        ],
    );

    let shown = run_jjj_success(&dir, &["problem", "show", "No body yet", "--json"]);
    assert_eq!(
        body_of(&shown, "description"),
        "First note.",
        "an append onto nothing is just the text, with no separator to lead with"
    );
}

/// Every kind that has a body can be appended to.
///
/// The per-command filter flags drifted exactly this way before `jjj query`
/// replaced them — present on two kinds, missing on the others, with no reason
/// behind the split. A writing verb available on three of five kinds would be
/// the same mistake, so the coverage is asserted rather than assumed.
#[test]
fn every_kind_with_a_body_accepts_an_append() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let problem = run_jjj_success(&dir, &["problem", "new", "Parent problem", "--json"]);
    let pid = {
        let v: serde_json::Value = serde_json::from_str(&problem).expect("valid JSON");
        v["id"].as_str().expect("id").to_string()
    };
    run_jjj_success(&dir, &["solution", "new", "A fix", "--problem", &pid]);
    run_jjj_success(&dir, &["critique", "new", "A fix", "A doubt"]);
    run_jjj_success(&dir, &["finding", "new", &pid, "A measurement"]);
    run_jjj_success(&dir, &["milestone", "new", "v1", "--date", "2026-12-01"]);

    for (kind, reference, field) in [
        ("problem", "Parent problem", "description"),
        ("solution", "A fix", "approach"),
        ("critique", "A doubt", "argument"),
        ("finding", "A measurement", "evidence"),
        ("milestone", "v1", "description"),
    ] {
        let out = run_jjj(
            &dir,
            &[kind, "edit", reference, "--append-body", "Added later."],
        );
        assert!(
            out.status.success(),
            "`jjj {kind} edit --append-body` must work: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let shown = run_jjj_success(&dir, &[kind, "show", reference, "--json"]);
        assert!(
            body_of(&shown, field).contains("Added later."),
            "the append must land in `{field}` on a {kind}: {shown}"
        );
    }
}

/// An append cannot double as a clear, and cannot be combined with a replace.
#[test]
fn an_empty_append_is_refused_and_so_is_mixing_it_with_body() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &["problem", "new", "Keeps its body", "--body", "Original."],
    );

    // Whitespace only: the usual cause is an empty command substitution, and
    // appending nothing is never the intent. Exit 1 — re-reading cannot help,
    // the caller has to change the command.
    let empty = run_jjj(
        &dir,
        &["problem", "edit", "Keeps its body", "--append-body", "   "],
    );
    assert_eq!(
        empty.status.code(),
        Some(1),
        "an empty append is a malformed command, not a precondition"
    );

    // --body and --append-body together is ambiguous, so clap refuses it.
    let both = run_jjj(
        &dir,
        &[
            "problem",
            "edit",
            "Keeps its body",
            "--body",
            "Replacement.",
            "--append-body",
            "Addition.",
        ],
    );
    assert!(
        !both.status.success(),
        "replace and append together is ambiguous"
    );

    // Neither refusal may have touched the body.
    let shown = run_jjj_success(&dir, &["problem", "show", "Keeps its body", "--json"]);
    assert_eq!(
        body_of(&shown, "description"),
        "Original.",
        "a refused edit must leave the body exactly as it was: {shown}"
    );
}

/// The append is a substantive write, so the clock fence applies to it.
///
/// This is the property that makes the verb safe to use in a loop: exit 13 says
/// the world moved and the same command may succeed after a re-read, where exit
/// 1 says the command itself was wrong.
#[test]
fn an_append_is_fenced_by_the_lamport_clock_like_any_other_write() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Fenced", "--body", "Start."]);

    // Bump the clock before fencing against it. A freshly created entity sits
    // at zero, and zero is `skip_serializing_if`-elided, so reading it back
    // gives no `lamport` key at all — `unwrap_or(0)` then makes the "stale"
    // value identical to the real one, the guard matches, and the write
    // correctly succeeds. The first draft of this test asserted 13 against
    // exactly that and was wrong about jjj rather than finding a bug in it.
    run_jjj_success(
        &dir,
        &["problem", "edit", "Fenced", "--append-body", "Bump."],
    );

    let shown = run_jjj_success(&dir, &["problem", "show", "Fenced", "--json"]);
    let lamport: u64 = {
        let v: serde_json::Value = serde_json::from_str(&shown).expect("valid JSON");
        v["lamport"].as_u64().unwrap_or(0)
    };
    assert!(
        lamport > 0,
        "a written entity must carry a clock, or there is nothing to fence: {shown}"
    );

    let stale = run_jjj(
        &dir,
        &[
            "problem",
            "edit",
            "Fenced",
            "--append-body",
            "Should not land.",
            "--if-lamport",
            &(lamport.saturating_sub(1)).to_string(),
        ],
    );
    assert_eq!(
        stale.status.code(),
        Some(13),
        "a stale clock is a precondition failure, retryable after a re-read"
    );

    let current = run_jjj(
        &dir,
        &[
            "problem",
            "edit",
            "Fenced",
            "--append-body",
            "Lands.",
            "--if-lamport",
            &lamport.to_string(),
        ],
    );
    assert!(current.status.success(), "a current clock must still write");

    let after = run_jjj_success(&dir, &["problem", "show", "Fenced", "--json"]);
    let body = body_of(&after, "description");
    assert!(body.contains("Lands."), "{after}");
    assert!(
        !body.contains("Should not land."),
        "the refused append must have written nothing: {after}"
    );
}
