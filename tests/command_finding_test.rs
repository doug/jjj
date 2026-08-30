mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

/// A finding is evidence about a problem: it records, it surfaces on the
/// problem, and it never acquires an approval state.
#[test]
fn test_finding_new_records_evidence_on_a_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Wasm bundle is too large"]);
    let stdout = run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            "Wasm bundle",
            "harfbuzz is 20% of the binary",
            "--body",
            "twiggy top -n 20 gallery.wasm",
            "--method",
            "twiggy 0.7 against a release build",
        ],
    );
    assert!(
        stdout.contains("Recorded finding"),
        "Expected creation confirmation: {stdout}"
    );

    let shown = run_jjj_success(&dir, &["finding", "show", "harfbuzz"]);
    assert!(
        shown.contains("current"),
        "Expected current status: {shown}"
    );
    assert!(shown.contains("twiggy 0.7"), "Expected method: {shown}");
    assert!(shown.contains("twiggy top"), "Expected evidence: {shown}");
}

/// The problem is where the reader looks first, so findings have to show up
/// there — evidence nobody encounters is evidence nobody uses.
#[test]
fn test_findings_surface_on_the_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Slow startup"]);
    run_jjj_success(
        &dir,
        &["finding", "new", "Slow startup", "First paint is 168ms"],
    );

    let stdout = run_jjj_success(&dir, &["problem", "show", "Slow startup"]);
    assert!(
        stdout.contains("Findings (1)"),
        "Expected the findings section on the problem: {stdout}"
    );
    assert!(
        stdout.contains("First paint is 168ms"),
        "Expected the finding title: {stdout}"
    );
}

/// A measurement is corrected, not rejected. Superseding keeps the old record
/// and points at what replaced it, which is what stops the same investigation
/// being run a third time.
#[test]
fn test_supersede_keeps_the_old_measurement_and_names_its_replacement() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Binary size"]);
    run_jjj_success(&dir, &["finding", "new", "Binary size", "Blob is 11.27MB"]);
    run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            "Binary size",
            "Blob is 3.78MB with -no-debug",
        ],
    );

    run_jjj_success(&dir, &["finding", "supersede", "11.27MB", "--by", "3.78MB"]);

    let shown = run_jjj_success(&dir, &["finding", "show", "11.27MB"]);
    assert!(
        shown.contains("superseded"),
        "Expected superseded status: {shown}"
    );
    assert!(
        shown.contains("Replaced by") && shown.contains("3.78MB"),
        "A superseded finding must name what corrected it: {shown}"
    );

    // The old one is still listed — deleting it would lose the fact that the
    // wrong number was once believed.
    let listed = run_jjj_success(&dir, &["finding", "list"]);
    assert!(
        listed.contains("11.27MB"),
        "Expected the old finding: {listed}"
    );

    let current = run_jjj_success(&dir, &["finding", "list", "--status", "current"]);
    assert!(
        !current.contains("11.27MB"),
        "A superseded finding must not appear as current: {current}"
    );
}

/// Superseding by itself would make `finding show` follow a cycle and assert
/// something that cannot be true.
#[test]
fn test_a_finding_cannot_supersede_itself() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["finding", "new", "Problem", "The only measurement"]);

    let out = run_jjj(&dir, &["finding", "supersede", "only", "--by", "only"]);
    assert!(
        !out.status.success(),
        "Expected self-supersession to be refused"
    );
}

/// The whole point of the type is that citation is machine-readable: without it
/// nothing can answer whether an investigation was ever actually used.
#[test]
fn test_solutions_cite_findings() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Decode is slow"]);
    run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            "Decode is slow",
            "decode.parse floors at 120004 ops",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Memoize the parse table",
            "--problem",
            "Decode is slow",
            // Referenced by title text, not by "120004" — an all-hex string of
            // six or more characters is read as a UUID prefix, not a title.
            "--cites",
            "floors at",
        ],
    );

    let shown = run_jjj_success(&dir, &["solution", "show", "Memoize"]);
    assert!(
        shown.contains("Evidence cited (1)"),
        "Expected the citation to surface on the solution: {shown}"
    );
    assert!(
        shown.contains("floors at 120004"),
        "Expected the cited finding's title: {shown}"
    );
}

/// A citation that resolves to nothing is a typo. Failing at creation beats
/// storing a dangling id nothing will ever notice.
#[test]
fn test_an_unresolvable_citation_is_refused() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    let out = run_jjj(
        &dir,
        &[
            "solution",
            "new",
            "Approach",
            "--problem",
            "Problem",
            "--cites",
            "no-such-finding",
        ],
    );
    assert!(
        !out.status.success(),
        "Expected an unresolvable --cites to fail rather than store a dangling id"
    );
}

/// `--ref` is untyped because a finding routinely bears on several kinds of
/// thing at once, and `--about` has to resolve the same way to find them again.
#[test]
fn test_findings_can_reference_a_solution_and_be_found_by_it() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Tick deletion"]);
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Restore the lookup",
            "--problem",
            "Tick deletion",
        ],
    );
    run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            "Tick deletion",
            "The restored lookup resolves the tick-deletion critique",
            "--ref",
            "Restore the lookup",
        ],
    );

    let listed = run_jjj_success(&dir, &["finding", "list", "--about", "Restore the lookup"]);
    assert!(
        listed.contains("tick-deletion"),
        "Expected the finding to be reachable from what it bears on: {listed}"
    );
}

/// Deleting loses the record of what was once believed, so it asks first.
#[test]
fn test_delete_requires_force_and_points_at_supersede() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(&dir, &["finding", "new", "Problem", "A measurement"]);

    let out = run_jjj(&dir, &["finding", "delete", "A measurement"]);
    assert!(!out.status.success(), "Expected delete to require --force");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("supersede"),
        "The refusal should point at the usually-correct action: {stderr}"
    );

    run_jjj_success(&dir, &["finding", "delete", "A measurement", "--force"]);
    let listed = run_jjj_success(&dir, &["finding", "list"]);
    assert!(
        listed.contains("No findings"),
        "Expected deletion: {listed}"
    );
}

/// Findings are evidence about the problem, so their events belong on its
/// timeline — otherwise conjectures appear from nowhere.
#[test]
fn test_findings_appear_on_the_problem_timeline() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Memory growth"]);
    run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            "Memory growth",
            "Heap grows 4MB per frame",
        ],
    );

    let stdout = run_jjj_success(&dir, &["timeline", "Memory growth"]);
    assert!(
        stdout.contains("recorded"),
        "Expected the finding on the timeline: {stdout}"
    );
}

/// JSON output is how agents read this; the fields they need must survive the
/// round trip through the SQLite cache.
#[test]
fn test_finding_json_round_trips_through_the_cache() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);
    run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            "Problem",
            "A claim",
            "--body",
            "the evidence",
            "--method",
            "how it was measured",
            "--tag",
            "perf",
        ],
    );

    // Force the cache to be built and read back through it.
    run_jjj_success(&dir, &["db", "rebuild"]);
    let json = run_jjj_success(&dir, &["finding", "list", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let first = &parsed[0];

    assert_eq!(first["title"], "A claim");
    assert_eq!(first["status"], "current");
    assert_eq!(
        first["method"], "how it was measured",
        "method must survive the cache round trip: {json}"
    );
    assert_eq!(first["evidence"], "the evidence", "body lost: {json}");
    assert_eq!(first["tags"][0], "perf");
}
