//! Bodies must be writable without an editor.
//!
//! Every entity has one free-form body — a problem's description, a solution's
//! approach, a critique's argument — and for a tool whose model is conjecture
//! and refutation, that body *is* the payload. Until `--body` existed the only
//! way to write one was an interactive `$EDITOR`, which a headless agent does
//! not have. Agents in a swarm trial responded the only way they could: they
//! crammed multi-paragraph reasoning into titles, producing critique titles
//! hundreds of characters long, and the structured field stayed empty.
//!
//! Writing one at creation was only half of it. `--body` reached `new` for all
//! five kinds but `edit` for only `finding`, so the other four bodies were
//! **write-once from the CLI**: an agent that recorded a wrong measurement in a
//! problem's description had no way to correct it, and `jjj problem edit --body`
//! failed with "unexpected argument". The tests below cover correcting a body
//! after the fact, which is the case the model is actually built on — a claim
//! stands until something better replaces it.

mod test_helpers;

use test_helpers::{jj_available, run_jjj_stdin, run_jjj_success, setup_test_repo};

#[test]
fn a_problem_description_can_be_supplied_without_an_editor() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Has a body",
            "--body",
            "Why this matters.",
        ],
    );

    let shown = run_jjj_success(&dir, &["problem", "show", "Has a body"]);
    assert!(
        shown.contains("Why this matters."),
        "the description did not survive: {shown}"
    );
}

#[test]
fn a_solution_approach_can_be_supplied_without_an_editor() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    // `--approach` is the per-entity alias; the field name is what someone
    // reaches for before checking the help.
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Conjecture",
            "--problem",
            "Problem",
            "--approach",
            "How the conjecture works.",
        ],
    );

    let shown = run_jjj_success(&dir, &["solution", "show", "Conjecture"]);
    assert!(
        shown.contains("How the conjecture works."),
        "the approach did not survive: {shown}"
    );
}

#[test]
fn a_critique_argument_can_be_read_from_stdin() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );

    // A real refutation runs to paragraphs and contains quotes and newlines —
    // exactly what gets mangled when it has to travel as a shell argument.
    let argument = "The premise holds but the fix does not:\n\n\
                    it validates the cache rather than the content, so a \"clean\" \
                    cache that is stale passes.\n";
    let out = run_jjj_stdin(
        &dir,
        argument,
        &[
            "critique",
            "new",
            "Solution",
            "Validates the cache, not the content",
            "--body",
            "-",
            "--severity",
            "critical",
        ],
    );
    assert!(
        out.status.success(),
        "critique new --body - failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let shown = run_jjj_success(&dir, &["critique", "list"]);
    let id = shown
        .lines()
        .find(|l| l.contains("Validates the cache"))
        .and_then(|l| l.split_whitespace().next())
        .expect("the critique should be listed")
        .to_string();

    let detail = run_jjj_success(&dir, &["critique", "show", &id]);
    assert!(
        detail.contains("it validates the cache rather than the content"),
        "the argument did not survive stdin: {detail}"
    );
    assert!(
        detail.contains("passes."),
        "the argument was truncated at the first newline: {detail}"
    );
}

#[test]
fn omitting_the_body_leaves_it_empty_rather_than_failing() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    // The flag is additive: every existing invocation must keep working.
    run_jjj_success(&dir, &["problem", "new", "No body given"]);
    let shown = run_jjj_success(&dir, &["problem", "show", "No body given"]);
    assert!(shown.contains("No body given"), "{shown}");
}

// ---------------------------------------------------------------------------
// Correcting a body after creation
// ---------------------------------------------------------------------------

#[test]
fn a_problem_description_can_be_corrected_after_creation() {
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
            "The first measurement said 700,004 ops.",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "problem",
            "edit",
            "Decode is slow",
            "--body",
            "Corrected: the floor is 60,004 ops.",
        ],
    );

    let shown = run_jjj_success(&dir, &["problem", "show", "Decode is slow"]);
    assert!(
        shown.contains("Corrected: the floor is 60,004 ops."),
        "the new description is missing: {shown}"
    );
    assert!(
        !shown.contains("700,004"),
        "the superseded figure is still being shown: {shown}"
    );
}

#[test]
fn a_solution_approach_can_be_corrected_after_creation() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Conjecture",
            "--problem",
            "Problem",
            "--body",
            "Memoize the parse.",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "solution",
            "edit",
            "Conjecture",
            "--body",
            "Precompute the table instead; memoizing still pays the hash.",
        ],
    );

    let shown = run_jjj_success(&dir, &["solution", "show", "Conjecture"]);
    assert!(
        shown.contains("Precompute the table instead"),
        "the revised approach is missing: {shown}"
    );
    assert!(
        !shown.contains("Memoize the parse."),
        "the old approach is still being shown: {shown}"
    );
}

#[test]
fn a_critique_argument_can_be_corrected_from_stdin() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &["solution", "new", "Solution", "--problem", "Problem"],
    );
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Solution",
            "Validates the cache, not the content",
            "--body",
            "First pass at the objection.",
        ],
    );

    let listed = run_jjj_success(&dir, &["critique", "list"]);
    let id = listed
        .lines()
        .find(|l| l.contains("Validates the cache"))
        .and_then(|l| l.split_whitespace().next())
        .expect("the critique should be listed")
        .to_string();

    // Sharpening an objection is the common case, and a sharpened one runs to
    // paragraphs — so the edit path has to take stdin like the create path.
    let sharper = "The premise holds but the fix does not:\n\n\
                   it validates the cache rather than the content, so a \"clean\" \
                   cache that is stale passes.\n";
    let out = run_jjj_stdin(&dir, sharper, &["critique", "edit", &id, "--body", "-"]);
    assert!(
        out.status.success(),
        "critique edit --body - failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let detail = run_jjj_success(&dir, &["critique", "show", &id]);
    assert!(
        detail.contains("it validates the cache rather than the content"),
        "the revised argument did not survive stdin: {detail}"
    );
    assert!(
        detail.contains("passes."),
        "the argument was truncated at the first newline: {detail}"
    );
    assert!(
        !detail.contains("First pass at the objection."),
        "the superseded argument is still being shown: {detail}"
    );
}

#[test]
fn a_milestone_description_can_be_corrected_after_creation() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &["milestone", "new", "Make it fast", "--body", "Vague goal."],
    );
    run_jjj_success(
        &dir,
        &[
            "milestone",
            "edit",
            "Make it fast",
            "--body",
            "Decode under 100,004 ops on the synth2 fixture.",
        ],
    );

    let shown = run_jjj_success(&dir, &["milestone", "show", "Make it fast"]);
    assert!(
        shown.contains("Decode under 100,004 ops"),
        "the revised description is missing: {shown}"
    );
    assert!(
        !shown.contains("Vague goal."),
        "the old description is still being shown: {shown}"
    );
}

#[test]
fn editing_other_fields_leaves_the_body_alone() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Keeps its body",
            "--body",
            "Evidence worth keeping.",
        ],
    );
    // The flag is optional, so an edit that does not mention it must not blank
    // the body — the failure mode that would quietly destroy the payload.
    run_jjj_success(
        &dir,
        &["problem", "edit", "Keeps its body", "--priority", "high"],
    );

    let shown = run_jjj_success(&dir, &["problem", "show", "Keeps its body"]);
    assert!(
        shown.contains("Evidence worth keeping."),
        "editing the priority destroyed the description: {shown}"
    );
}
