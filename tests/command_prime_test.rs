//! `jjj prime` orients an agent in one call.
//!
//! An agent turn starts with no memory and re-derives the world. Measured in a
//! swarm run: 2,322 orientation calls against 138 that produced anything, a
//! ratio of 17 to 1. The harness had worked around it by hand-rolling a briefing
//! from four `jjj` invocations and sixty lines of embedded Python, which only
//! helped agents under that harness and could not be tested.

mod test_helpers;

use test_helpers::{jj_available, run_jjj_env, run_jjj_success, setup_test_repo};

fn id_of(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v["id"].as_str().expect("id").to_string()
}

fn populated() -> (tempfile::TempDir, String) {
    let dir = setup_test_repo();
    let p1 = id_of(&run_jjj_success(
        &dir,
        &["problem", "new", "Decode is slow", "--json"],
    ));
    run_jjj_success(&dir, &["problem", "new", "Docs are thin"]);
    let s1 = id_of(&String::from_utf8_lossy(
        &run_jjj_env(
            &dir,
            &[("JJJ_USER", "ana")],
            &[
                "solution",
                "new",
                "Memoize the table",
                "--problem",
                &p1,
                "--json",
            ],
        )
        .stdout,
    ));
    run_jjj_env(
        &dir,
        &[("JJJ_USER", "bo")],
        &[
            "solution",
            "new",
            "Precompute instead",
            "--problem",
            &p1,
            "--force",
        ],
    );
    run_jjj_env(
        &dir,
        &[("JJJ_USER", "cy")],
        &[
            "critique",
            "new",
            &s1,
            "Cache is unbounded",
            "--severity",
            "high",
        ],
    );
    run_jjj_env(
        &dir,
        &[("JJJ_USER", "ana")],
        &[
            "finding",
            "new",
            &p1,
            "decode.parse floors at 60k ops",
            "--method",
            "counted",
        ],
    );
    (dir, p1)
}

/// The briefing must carry the four things an agent would otherwise query for.
#[test]
fn prime_reports_work_proposals_objections_and_evidence() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out =
        String::from_utf8_lossy(&run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime"]).stdout)
            .to_string();

    assert!(out.contains("you are @dev"), "should name the actor: {out}");
    assert!(out.contains("Next for you"), "should say what to do: {out}");
    assert!(
        out.contains("Already proposed") && out.contains("Memoize the table"),
        "must show proposals so an agent does not duplicate one: {out}"
    );
    assert!(
        out.contains("Open critiques") && out.contains("Cache is unbounded"),
        "must show unanswered objections: {out}"
    );
    assert!(
        out.contains("Already measured") && out.contains("floors at 60k"),
        "must show evidence, the thing agents kept re-deriving: {out}"
    );
}

/// The author of an unassigned proposal has to be named — it is the field that
/// tells an agent whose idea it would be duplicating. Solutions store an
/// assignee but not an author, so this comes from the creation event.
#[test]
fn prime_names_who_proposed_each_approach() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out =
        String::from_utf8_lossy(&run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime"]).stdout)
            .to_string();
    assert!(out.contains("@ana"), "author of the first proposal: {out}");
    assert!(out.contains("@bo"), "author of the rival proposal: {out}");
    assert!(
        !out.contains("@?"),
        "an unresolved author makes the section useless: {out}"
    );
}

/// References must be pasteable. UUID7 ids are time-ordered, so entities created
/// in the same second share their leading characters — a fixed-width prefix
/// printed the same six characters for every section.
#[test]
fn prime_references_are_unambiguous() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out =
        String::from_utf8_lossy(&run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime"]).stdout)
            .to_string();

    // Collect the leading token of every bullet that starts with a hex id.
    let refs: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .map(|l| l.split_whitespace().next().unwrap_or(""))
        .filter(|t| t.len() >= 6 && t.chars().all(|c| c.is_ascii_hexdigit()))
        .collect();
    assert!(refs.len() >= 3, "expected several references: {out}");
    let unique: std::collections::HashSet<&&str> = refs.iter().collect();
    assert_eq!(
        unique.len(),
        refs.len(),
        "two references share a prefix, so at most one can be resolved: {refs:?}\n{out}"
    );
}

/// Escalations lead, for the same reason they lead `jjj status`: a fleet blocked
/// on a person must not have that ranked below its work queue.
#[test]
fn prime_leads_with_an_escalation() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    run_jjj_success(&dir, &["escalate", "Needs production credentials"]);
    let out =
        String::from_utf8_lossy(&run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime"]).stdout)
            .to_string();

    let esc = out
        .find("Needs production credentials")
        .expect("escalation shown");
    let next = out.find("Next for you").unwrap_or(usize::MAX);
    assert!(esc < next, "the escalation must come first:\n{out}");
}

/// Machine-readable, because the harness consumes this rather than a human.
#[test]
fn prime_json_carries_counts_and_items() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out = run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime", "--json"]);
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).expect("valid JSON");

    assert_eq!(v["actor"], "dev");
    assert_eq!(v["counts"]["proposed"], 2);
    assert_eq!(v["counts"]["open_critiques"], 1);
    assert_eq!(v["counts"]["findings_current"], 1);
    assert_eq!(
        v["proposed"].as_array().map(|a| a.len()),
        Some(2),
        "items as well as counts, so a caller need not re-query"
    );
}

/// An empty repository must still produce something an agent can act on,
/// rather than a wall of empty headings.
#[test]
fn prime_on_an_empty_repo_is_short_and_says_so() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out =
        String::from_utf8_lossy(&run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime"]).stdout)
            .to_string();
    assert!(out.contains("nothing queued"), "should say so: {out}");
    assert!(
        !out.contains("Already proposed"),
        "empty sections must be omitted: {out}"
    );
    assert!(
        out.lines().count() < 12,
        "a briefing must stay brief: {out}"
    );
}

/// The briefing's length must not track the corpus.
///
/// Every other section takes the per-section cap; the escalation section did
/// not, in either the text or the JSON path. A repository with 132 open
/// escalations printed all 132, which is the exact failure `prime` exists to
/// prevent — an agent re-reading an unbounded wall at the start of every turn.
/// The count still has to be exact, because the cap is only acceptable if the
/// heading admits what it is hiding.
#[test]
fn prime_caps_the_escalation_section_like_every_other_one() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    for i in 0..9 {
        run_jjj_success(&dir, &["escalate", &format!("Blocker number {i}")]);
    }

    let out =
        String::from_utf8_lossy(&run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime"]).stdout)
            .to_string();
    let listed = (0..9)
        .filter(|i| out.contains(&format!("Blocker number {i}")))
        .count();
    assert!(
        listed <= 5,
        "the text path listed {listed} of 9 escalations; the cap is 5:\n{out}"
    );
    assert!(
        out.contains("9 open escalations"),
        "the heading must still carry the true total:\n{out}"
    );
    assert!(
        out.contains("and 4 more"),
        "a capped section must admit what it is hiding:\n{out}"
    );

    let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime", "--json"]).stdout,
    ))
    .expect("valid JSON");
    assert_eq!(
        v["escalations"].as_array().map(|a| a.len()),
        Some(5),
        "the JSON path emitted the whole vector: {}",
        v["escalations"]
    );
    assert_eq!(
        v["counts"]["escalations"], 9,
        "the count must be exact even though the list is capped"
    );

    let full = String::from_utf8_lossy(
        &run_jjj_env(&dir, &[("JJJ_USER", "dev")], &["prime", "--full"]).stdout,
    )
    .to_string();
    let listed_full = (0..9)
        .filter(|i| full.contains(&format!("Blocker number {i}")))
        .count();
    assert_eq!(
        listed_full, 9,
        "`--full` must still show every escalation:\n{full}"
    );
}
