//! `jjj plan apply` — one call, one graph.
//!
//! An agent that has thought a problem through has several entities in mind at
//! once: a problem, its sub-problems, the evidence already measured, a
//! conjecture for each. Creating those one command at a time costs a process
//! per entity and a metadata commit per entity, and gives up halfway if the
//! fourth call fails — leaving a half-built graph and no record of the intent.
//!
//! The borrowed idea is symbolic keys: a node names its parent by a key local to
//! the plan, because the id does not exist yet. The result maps each key to the
//! id it became, which is what the caller needs to say anything else about the
//! work it just created.
//!
//! What these tests care about most is that **a bad plan writes nothing**. A
//! partial graph is worse than a rejected one, because nothing records which
//! half is missing.

mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

fn write_plan(dir: &std::path::Path, json: &str) -> std::path::PathBuf {
    let path = dir.join("plan.json");
    std::fs::write(&path, json).expect("write plan");
    path
}

fn problem_count(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir.join(".jj/jjj-meta/problems"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0)
}

const FULL_PLAN: &str = r#"{
  "milestones": [{"key": "m1", "title": "Make it fast", "date": "2026-12-01"}],
  "problems": [
    {"key": "root", "title": "Decode is slow", "body": "700,004 ops.",
     "priority": "critical", "tags": ["perf"], "milestone": "m1"},
    {"key": "hdr", "title": "Header parse dominates", "parent": "root"},
    {"key": "tbl", "title": "Table lookup is linear", "parent": "root"}
  ],
  "findings": [
    {"key": "f1", "problem": "hdr", "title": "The parse floors at a fixed cost",
     "method": "counted, three runs", "body": "60,004 ops minimum.",
     "measured_at": "2026-10-16T14:00:00Z"}
  ],
  "solutions": [
    {"key": "s1", "title": "Memoize the parse", "problem": "hdr",
     "body": "Cache by (path, mtime).", "cites": ["f1"], "tags": ["perf"]}
  ],
  "critiques": [
    {"key": "c1", "solution": "s1", "title": "Validates the cache, not the content",
     "severity": "critical", "body": "A stale clean cache passes."}
  ]
}"#;

#[test]
fn one_call_creates_a_wired_graph() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    let plan = write_plan(p, FULL_PLAN);

    let out = run_jjj_success(p, &["plan", "apply", plan.to_str().unwrap(), "--json"]);
    let ids: serde_json::Value = serde_json::from_str(&out).expect("key→id map");
    for key in ["m1", "root", "hdr", "tbl", "f1", "s1", "c1"] {
        assert!(
            ids[key].as_str().is_some(),
            "no id returned for key '{key}': {out}"
        );
    }

    // The hierarchy came from symbolic keys, not from ids that did not exist.
    let tree = run_jjj_success(p, &["problem", "tree"]);
    let root_line = tree
        .lines()
        .position(|l| l.contains("Decode is slow"))
        .expect("root in tree");
    let child_line = tree
        .lines()
        .position(|l| l.contains("Header parse dominates"))
        .expect("child in tree");
    assert!(
        child_line > root_line,
        "the sub-problem is not under its parent:\n{tree}"
    );

    // Scalars survived the round trip.
    let root = run_jjj_success(p, &["problem", "show", "Decode is slow", "--json"]);
    let root: serde_json::Value = serde_json::from_str(&root).expect("valid JSON");
    assert_eq!(root["priority"].as_str(), Some("critical"));
    assert_eq!(root["description"].as_str(), Some("700,004 ops."));
    assert!(
        root["milestone_id"].as_str().is_some(),
        "the milestone created in the same plan was not attached: {root}"
    );

    // And a reference between two nodes of the same plan resolved to the real
    // id — the thing symbolic keys exist to do.
    let sol = run_jjj_success(p, &["solution", "show", "Memoize the parse", "--json"]);
    let sol: serde_json::Value = serde_json::from_str(&sol).expect("valid JSON");
    let cited = sol["cites"][0].as_str().expect("a citation");
    assert_eq!(
        cited,
        ids["f1"].as_str().unwrap(),
        "the solution cites something other than the finding from this plan"
    );
}

#[test]
fn dry_run_validates_and_writes_nothing() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    let plan = write_plan(p, FULL_PLAN);
    let before = problem_count(p);

    let out = run_jjj_success(p, &["plan", "apply", plan.to_str().unwrap(), "--dry-run"]);
    assert!(out.contains("valid"), "{out}");
    assert!(out.contains("7"), "it should say how many: {out}");
    assert_eq!(
        problem_count(p),
        before,
        "--dry-run created entities: {out}"
    );
}

/// Every one of these is a mistake an agent actually makes. Each must be caught
/// before anything is written, and must say which node is at fault.
#[test]
fn a_bad_plan_writes_nothing() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    let cases: [(&str, &str, &str); 8] = [
        (
            "a misspelled field",
            r#"{"problems":[{"key":"a","title":"T","prioirty":"high"}]}"#,
            "prioirty",
        ),
        (
            "an unknown priority",
            r#"{"problems":[{"key":"a","title":"T","priority":"urgent"}]}"#,
            "priority",
        ),
        (
            "a duplicate key",
            r#"{"problems":[{"key":"a","title":"One"},{"key":"a","title":"Two"}]}"#,
            "duplicate key",
        ),
        (
            "a parent cycle",
            r#"{"problems":[{"key":"a","title":"A","parent":"b"},{"key":"b","title":"B","parent":"a"}]}"#,
            "cycle",
        ),
        (
            "a reference to nothing",
            r#"{"solutions":[{"key":"s","title":"S","problem":"no-such-problem"}]}"#,
            "neither a key in this plan",
        ),
        (
            "a bad date",
            r#"{"milestones":[{"key":"m","title":"M","date":"Dec 2026"}]}"#,
            "YYYY-MM-DD",
        ),
        (
            "a measured_at that is not a timestamp",
            r#"{"problems":[{"key":"a","title":"T"}],"findings":[{"key":"f","problem":"a","title":"F","measured_at":"last Tuesday"}]}"#,
            "RFC3339",
        ),
        ("an empty plan", r#"{}"#, "does nothing"),
    ];

    for (what, body, expected) in cases {
        let plan = write_plan(p, body);
        let out = run_jjj(p, &["plan", "apply", plan.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{what}: the plan was accepted\n{stderr}"
        );
        assert!(
            stderr.contains(expected),
            "{what}: the error did not mention '{expected}': {stderr}"
        );
        assert_eq!(
            problem_count(p),
            0,
            "{what}: a rejected plan still wrote entities, leaving a half-built graph"
        );
    }
}

#[test]
fn a_plan_can_hang_new_work_off_an_existing_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    run_jjj_success(p, &["problem", "new", "Existing work"]);

    // References that are not plan keys are resolved against the repository the
    // same way every other command resolves them — including by fuzzy title.
    let plan = write_plan(
        p,
        r#"{"problems":[{"key":"child","title":"New sub-problem","parent":"Existing work"}]}"#,
    );
    run_jjj_success(p, &["plan", "apply", plan.to_str().unwrap()]);

    let tree = run_jjj_success(p, &["problem", "tree"]);
    let parent = tree
        .lines()
        .position(|l| l.contains("Existing work"))
        .expect("parent");
    let child = tree
        .lines()
        .position(|l| l.contains("New sub-problem"))
        .expect("child");
    assert!(child > parent, "the child did not attach:\n{tree}");
}

#[test]
fn a_plan_can_be_read_from_stdin() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    // An agent composing a plan has it in memory, not on disk.
    let out = test_helpers::run_jjj_stdin(
        p,
        r#"{"problems":[{"key":"only","title":"From stdin"}]}"#,
        &["plan", "apply", "-", "--json"],
    );
    assert!(
        out.status.success(),
        "stdin plan failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ids: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).expect("key→id map");
    assert!(ids["only"].as_str().is_some());

    let listed = run_jjj_success(p, &["problem", "list"]);
    assert!(listed.contains("From stdin"), "{listed}");
}

// ---------------------------------------------------------------------------
// Bulk field updates and lifecycle actions
// ---------------------------------------------------------------------------

#[test]
fn a_plan_can_update_fields_in_bulk() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    for n in ["One", "Two"] {
        run_jjj_success(
            p,
            &["problem", "new", &format!("Stale {n}"), "--priority", "low"],
        );
    }

    let plan = write_plan(
        p,
        r#"{"updates":[
            {"kind":"problem","on":"Stale One","priority":"critical","tags":["triaged"]},
            {"kind":"problem","on":"Stale Two","body":"Re-measured: still relevant."}
        ]}"#,
    );
    run_jjj_success(p, &["plan", "apply", plan.to_str().unwrap()]);

    let one = run_jjj_success(p, &["problem", "show", "Stale One", "--json"]);
    let one: serde_json::Value = serde_json::from_str(&one).expect("valid JSON");
    assert_eq!(one["priority"].as_str(), Some("critical"));
    assert_eq!(one["tags"][0].as_str(), Some("triaged"));

    let two = run_jjj_success(p, &["problem", "show", "Stale Two", "--json"]);
    let two: serde_json::Value = serde_json::from_str(&two).expect("valid JSON");
    assert_eq!(
        two["description"].as_str(),
        Some("Re-measured: still relevant.")
    );
}

/// The point of routing lifecycle through `actions` rather than letting
/// `updates` set a status: the guards must behave exactly as they do for the
/// single-entity command. beads has precisely this asymmetry and documents it —
/// its batch `close` "does not apply that policy at all", unlike its own
/// `bd close`.
#[test]
fn a_bulk_lifecycle_action_obeys_the_same_guard_as_the_command() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    run_jjj_success(p, &["problem", "new", "Unsolved"]);

    // The single command refuses: solving needs an approved solution.
    let single = run_jjj(p, &["problem", "solve", "Unsolved"]);
    assert!(!single.status.success());
    let single_err = String::from_utf8_lossy(&single.stderr).to_string();
    assert!(single_err.contains("No approved solution"), "{single_err}");

    // The bulk path must refuse with the same reason, and change nothing.
    let plan = write_plan(p, r#"{"actions":[{"action":"solve","on":"Unsolved"}]}"#);
    let bulk = run_jjj(p, &["plan", "apply", plan.to_str().unwrap()]);
    assert!(
        !bulk.status.success(),
        "the bulk path bypassed the guard the command enforces"
    );
    assert!(
        String::from_utf8_lossy(&bulk.stderr).contains("No approved solution"),
        "the bulk refusal gave a different reason than the command: {}",
        String::from_utf8_lossy(&bulk.stderr)
    );

    let shown = run_jjj_success(p, &["problem", "show", "Unsolved", "--json"]);
    let shown: serde_json::Value = serde_json::from_str(&shown).expect("valid JSON");
    assert_eq!(
        shown["status"].as_str(),
        Some("open"),
        "a refused bulk action still changed the status"
    );
}

#[test]
fn a_bulk_action_that_is_allowed_runs_with_its_side_effects() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    run_jjj_success(p, &["problem", "new", "Wrong premise"]);

    let plan = write_plan(
        p,
        r#"{"actions":[{"action":"dissolve","on":"Wrong premise","reason":"premise was wrong"}]}"#,
    );
    run_jjj_success(p, &["plan", "apply", plan.to_str().unwrap()]);

    let shown = run_jjj_success(p, &["problem", "show", "Wrong premise", "--json"]);
    let shown: serde_json::Value = serde_json::from_str(&shown).expect("valid JSON");
    assert_eq!(shown["status"].as_str(), Some("dissolved"));
    assert_eq!(
        shown["dissolved_reason"].as_str(),
        Some("premise was wrong")
    );

    // Dissolving emits an event; the bulk path must not lose it, or the
    // timeline would show a status that changed for no recorded reason.
    let events = run_jjj_success(p, &["events"]);
    assert!(
        events.contains("dissolved") || events.contains("problem_dissolved"),
        "the lifecycle event was not recorded: {events}"
    );
}

/// A field that does not apply to a kind is a mistake, and ignoring it would
/// leave the author believing it was applied.
#[test]
fn an_inapplicable_field_is_refused_not_ignored() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    run_jjj_success(p, &["problem", "new", "Anything"]);

    for (body, expected) in [
        (
            r#"{"updates":[{"kind":"milestone","on":"x","priority":"high"}]}"#,
            "no priority",
        ),
        (
            r#"{"updates":[{"kind":"critique","on":"x","tags":["a"]}]}"#,
            "no tags",
        ),
        (
            r#"{"updates":[{"kind":"problem","on":"Anything"}]}"#,
            "changes nothing",
        ),
        (
            r#"{"updates":[{"kind":"gremlin","on":"x","title":"t"}]}"#,
            "unknown kind",
        ),
    ] {
        let plan = write_plan(p, body);
        let out = run_jjj(p, &["plan", "apply", plan.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "accepted: {body}");
        assert!(
            stderr.contains(expected),
            "expected '{expected}' for {body}, got: {stderr}"
        );
    }
}

/// Each update carries its own guard, because a plan touching ten entities has
/// ten independent "has this moved" answers.
#[test]
fn an_update_can_guard_itself_with_a_clock() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    run_jjj_success(p, &["problem", "new", "Guarded"]);

    let plan = write_plan(
        p,
        r#"{"updates":[{"kind":"problem","on":"Guarded","title":"Rewritten","if_lamport":99}]}"#,
    );
    let out = run_jjj(p, &["plan", "apply", plan.to_str().unwrap()]);
    assert!(!out.status.success(), "a stale guarded update was applied");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("has changed since you read it"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let shown = run_jjj_success(p, &["problem", "show", "Guarded", "--json"]);
    let shown: serde_json::Value = serde_json::from_str(&shown).expect("valid JSON");
    assert_eq!(shown["title"].as_str(), Some("Guarded"));
}

/// A finding from a plan is the case where measurement time and recording time
/// differ most: the plan is written after the numbers are taken and applied
/// later still. Without `measured_at` the only time the entity carries is when
/// it happened to be created, which is the wrong answer by however long the
/// plan sat unapplied.
///
/// The author matters for the same kind of reason. Every entity a plan creates
/// used to be anonymous, so a fifty-problem plan produced fifty findings and
/// critiques that no author filter and no `--mine` could see.
#[test]
fn a_plan_records_when_evidence_was_measured_and_who_filed_it() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    let plan = write_plan(p, FULL_PLAN);
    run_jjj_success(p, &["plan", "apply", plan.to_str().unwrap()]);

    let findings = run_jjj_success(p, &["finding", "list", "--json"]);
    let findings: serde_json::Value = serde_json::from_str(&findings).expect("valid JSON");
    let f = &findings.as_array().expect("a list")[0];
    assert_eq!(
        f["measured_at"].as_str(),
        Some("2026-10-16T14:00:00Z"),
        "the plan's measured_at did not survive: {f}"
    );
    assert!(
        f["author"].as_str().is_some_and(|a| !a.is_empty()),
        "a finding from a plan has no author, so no author filter can find it: {f}"
    );

    let critiques = run_jjj_success(p, &["critique", "list", "--json"]);
    let critiques: serde_json::Value = serde_json::from_str(&critiques).expect("valid JSON");
    let c = &critiques.as_array().expect("a list")[0];
    assert!(
        c["author"].as_str().is_some_and(|a| !a.is_empty()),
        "a critique from a plan has no author: {c}"
    );
}
