//! One query grammar across every entity kind.
//!
//! jjj grew filter flags a command at a time and they drifted: `--tag` and
//! `--sort` on problems and solutions but not critiques or findings, `--author`
//! on critiques and findings where the others say `--assignee`, and
//! `milestone list` with no filters at all. One query string
//! (`status:open sort:edit`) covers every kind instead.

mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_env, run_jjj_success, setup_test_repo};

fn id_of(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v["id"].as_str().expect("id").to_string()
}

/// A repo with one of every kind, so cross-kind behaviour is exercised.
fn populated() -> (tempfile::TempDir, String) {
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["milestone", "new", "Cycle"]);
    let p1 = id_of(&run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Decode is slow",
            "--tags",
            "perf,core",
            "--json",
        ],
    ));
    run_jjj_success(&dir, &["problem", "new", "Docs are thin", "--tags", "docs"]);
    let s1 = id_of(
        &run_jjj_env(
            &dir,
            &[("JJJ_USER", "ana")],
            &[
                "solution",
                "new",
                "Memoize",
                "--problem",
                &p1,
                "--tags",
                "perf",
                "--json",
            ],
        )
        .stdout
        .iter()
        .map(|b| *b as char)
        .collect::<String>(),
    );
    run_jjj_env(
        &dir,
        &[("JJJ_USER", "bo")],
        &["critique", "new", &s1, "Cache is unbounded"],
    );
    run_jjj_env(
        &dir,
        &[("JJJ_USER", "ana")],
        &[
            "finding",
            "new",
            &p1,
            "Parse floors at 60k",
            "--method",
            "counted",
        ],
    );
    (dir, p1)
}

#[test]
fn an_empty_query_spans_every_kind() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let json = run_jjj_success(&dir, &["query", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let kinds: std::collections::HashSet<&str> = v
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|r| r["kind"].as_str())
        .collect();
    for want in ["problem", "solution", "critique", "finding", "milestone"] {
        assert!(
            kinds.contains(want),
            "{want} missing from an unfiltered query: {kinds:?}"
        );
    }
}

#[test]
fn type_and_tag_filters_apply_to_kinds_that_had_no_flags() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();

    // `milestone list` accepts no filters at all; the grammar covers it.
    let ms = run_jjj_success(&dir, &["query", "type:milestone", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&ms).expect("valid JSON");
    assert_eq!(
        v.as_array().map(|a| a.len()),
        Some(1),
        "expected one milestone: {ms}"
    );
    assert_eq!(v[0]["kind"], "milestone");

    // Tags, which critiques and findings have no flag for.
    let tagged = run_jjj_success(&dir, &["query", "tag:perf", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&tagged).expect("valid JSON");
    let titles: Vec<&str> = v
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|r| r["title"].as_str())
        .collect();
    assert!(
        titles.contains(&"Decode is slow"),
        "tag:perf missed the problem: {titles:?}"
    );
    assert!(
        titles.contains(&"Memoize"),
        "tag:perf missed the solution: {titles:?}"
    );
    assert!(
        !titles.contains(&"Docs are thin"),
        "tag:perf matched too much: {titles:?}"
    );
}

#[test]
fn repeating_a_key_ors_and_different_keys_and() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out = run_jjj_success(&dir, &["query", "type:problem,solution tag:perf", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let rows = v.as_array().expect("array");
    assert_eq!(
        rows.len(),
        2,
        "expected the perf problem and solution: {out}"
    );
    for r in rows {
        assert!(
            matches!(r["kind"].as_str(), Some("problem") | Some("solution")),
            "type filter leaked: {out}"
        );
    }
}

/// `author:` works across kinds that previously used two different flag names.
#[test]
fn author_matches_whichever_person_field_the_kind_carries() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out = run_jjj_success(&dir, &["query", "author:ana", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let titles: Vec<&str> = v
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|r| r["title"].as_str())
        .collect();
    assert!(
        titles.contains(&"Parse floors at 60k"),
        "finding author missed: {titles:?}"
    );
    assert!(
        !titles.contains(&"Cache is unbounded"),
        "bo's critique matched ana: {titles:?}"
    );
}

/// `problem:<ref>` scopes a whole subtree, which no single command could do.
#[test]
fn a_problem_reference_scopes_every_kind_beneath_it() {
    if !jj_available() {
        return;
    }
    let (dir, p1) = populated();
    let out = run_jjj_success(&dir, &["query", &format!("problem:{p1}"), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let kinds: Vec<&str> = v
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|r| r["kind"].as_str())
        .collect();
    for want in ["problem", "solution", "critique", "finding"] {
        assert!(
            kinds.contains(&want),
            "{want} not scoped under the problem: {out}"
        );
    }
    assert!(
        !kinds.contains(&"milestone"),
        "a milestone is not under a problem: {out}"
    );
    // The unrelated problem must not appear.
    let titles: Vec<&str> = v
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|r| r["title"].as_str())
        .collect();
    assert!(
        !titles.contains(&"Docs are thin"),
        "scoping leaked: {titles:?}"
    );
}

#[test]
fn bare_words_match_the_title_and_limit_truncates() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out = run_jjj_success(&dir, &["query", "decode", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(
        v.as_array().map(|a| a.len()),
        Some(1),
        "expected one title match: {out}"
    );

    let capped = run_jjj_success(&dir, &["query", "limit:2", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&capped).expect("valid JSON");
    assert_eq!(
        v.as_array().map(|a| a.len()),
        Some(2),
        "limit ignored: {capped}"
    );
}

/// `sort:edit` orders by the Lamport clock, so "most recently edited" does not
/// depend on whose machine had the fastest wall clock.
#[test]
fn sort_edit_orders_by_the_causal_clock() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    // Touch the older problem so it becomes the most recently edited.
    run_jjj_success(
        &dir,
        &[
            "problem",
            "edit",
            "Docs are thin",
            "--title",
            "Docs, revised",
        ],
    );

    let out = run_jjj_success(&dir, &["query", "type:problem sort:edit", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(
        v[0]["title"], "Docs, revised",
        "the most recently edited problem should lead: {out}"
    );
    let first = v[0]["lamport"].as_u64().unwrap_or(0);
    let second = v[1]["lamport"].as_u64().unwrap_or(0);
    assert!(first > second, "clocks did not order the result: {out}");

    // And `-` reverses.
    let rev = run_jjj_success(&dir, &["query", "type:problem sort:-edit", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&rev).expect("valid JSON");
    assert_ne!(
        v[0]["title"], "Docs, revised",
        "sort:-edit did not reverse: {rev}"
    );
}

/// A typo must be an error, not a silently-empty title search.
#[test]
fn an_unknown_key_is_refused_with_the_valid_ones_named() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out = run_jjj(&dir, &["query", "statuss:open"]);
    assert!(!out.status.success(), "a misspelled key should fail");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown term"), "unhelpful: {err}");
    assert!(err.contains("status"), "should list the valid keys: {err}");
}

#[test]
fn mine_resolves_to_the_current_actor() {
    if !jj_available() {
        return;
    }
    let (dir, _) = populated();
    let out = run_jjj_env(&dir, &[("JJJ_USER", "ana")], &["query", "mine", "--json"]);
    let text = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    let titles: Vec<&str> = v
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|r| r["title"].as_str())
        .collect();
    assert!(
        titles.contains(&"Parse floors at 60k"),
        "ana's finding missing: {titles:?}"
    );
    assert!(
        !titles.contains(&"Cache is unbounded"),
        "bo's critique matched `mine`: {titles:?}"
    );
}
