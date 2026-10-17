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

/// `waiting` names the child holding the parent, not just the parent.
///
/// The predicate walks the open sub-problems to decide the parent belongs in
/// the listing at all, then threw that set away — so the answer to "what do I
/// work instead?", which is the only reason to run `waiting`, cost a
/// `problem show` per row to recover something already computed.
#[test]
fn waiting_names_what_is_holding_each_parent() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let parent = id_of(&run_jjj_success(
        &dir,
        &["problem", "new", "Ship the release", "--json"],
    ));
    run_jjj_success(
        &dir,
        &["problem", "new", "Fix the flaky test", "--parent", &parent],
    );

    let json = run_jjj_success(&dir, &["query", "waiting", "--json"]);
    let rows: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let rows = rows.as_array().expect("an array");
    assert_eq!(rows.len(), 1, "one parent is held back: {json}");
    let blockers = rows[0]["blocked_by"]
        .as_array()
        .unwrap_or_else(|| panic!("waiting must say what blocks the parent: {json}"));
    let titles: Vec<&str> = blockers
        .iter()
        .filter_map(|b| b["title"].as_str())
        .collect();
    assert_eq!(titles, vec!["Fix the flaky test"], "{json}");
    assert!(
        blockers[0]["id"].as_str().is_some_and(|i| i.len() > 6),
        "the blocker must be addressable by id, not only nameable: {json}"
    );

    // The text path too — that is what a person reads.
    let text = run_jjj_success(&dir, &["query", "waiting"]);
    assert!(
        text.contains("Fix the flaky test"),
        "the listing must name the blocker: {text}"
    );

    // And it stays off every other listing: a field computed per row by
    // scanning all problems should not be paid for by queries that never asked.
    let all = run_jjj_success(&dir, &["query", "type:problem", "--json"]);
    assert!(
        !all.contains("blocked_by"),
        "blocked_by belongs to `waiting` alone: {all}"
    );
}

/// A limit that hid matches says so on the JSON path, not only the text one.
///
/// `limit:3` returning three rows is indistinguishable from a repository that
/// holds exactly three matches. The text path has always printed "Showing 3 of
/// 8 matches"; the JSON path returned before reaching it, so the human was told
/// and the agent was not — and "nothing else is open" is the sort of wrong
/// conclusion that ends a search early.
///
/// The notice goes to stderr because stdout is a bare array of rows, pinned by
/// the tests above and taught by SKILL.md. That is asserted here as well: an
/// envelope would be the more obvious design and would silently break every
/// caller parsing the array today.
#[test]
fn a_limit_that_hid_matches_says_so_on_both_paths() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    for i in 0..8 {
        run_jjj_success(&dir, &["problem", "new", &format!("Problem {i}")]);
    }

    let out = run_jjj(&dir, &["query", "type:problem limit:3", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let rows: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must stay a bare JSON array ({e}): {stdout}"));
    assert_eq!(rows.as_array().expect("an array").len(), 3, "{stdout}");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("3 of 8"),
        "the JSON path must report the cap: {stderr}"
    );

    // Unlimited: no notice, or it would cry wolf on every call.
    let clean = run_jjj(&dir, &["query", "type:problem", "--json"]);
    assert!(
        !String::from_utf8_lossy(&clean.stderr).contains("hidden by the limit"),
        "an uncapped query must not warn: {}",
        String::from_utf8_lossy(&clean.stderr)
    );
}

/// `--count-by` answers "how much of what" in one call, over the whole set.
///
/// `insights` reports fixed aggregates and the listings report rows, so "how
/// many open problems per assignee" meant one call per assignee. Two properties
/// are worth pinning beyond the arithmetic.
///
/// A `limit` must not shrink a count. A capped count is a different number
/// wearing the same name, and the caller cannot see the difference — the same
/// failure idea 32 exists to prevent.
///
/// And the overlap caption must be computed, not assumed. It first read
/// `multi = true` for any tag query, so a set where every entity was untagged
/// printed "these sum higher" above numbers that summed exactly — a caption
/// contradicting the data beside it.
#[test]
fn counting_by_a_field_covers_the_whole_match_set() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &["problem", "new", "Flaky test", "--tags", "ci,flake"],
    );
    run_jjj_success(&dir, &["problem", "new", "Slow build", "--tags", "ci"]);
    run_jjj_success(&dir, &["problem", "new", "Untagged one"]);

    let parse = |s: &str| -> serde_json::Value {
        serde_json::from_str(s).unwrap_or_else(|e| panic!("not JSON ({e}): {s}"))
    };

    let by_tag = parse(&run_jjj_success(
        &dir,
        &["query", "type:problem", "--count-by", "tag", "--json"],
    ));
    assert_eq!(by_tag["matches"], 3, "{by_tag}");
    let counts: std::collections::HashMap<&str, u64> = by_tag["counts"]
        .as_array()
        .expect("counts array")
        .iter()
        .map(|c| (c["key"].as_str().unwrap(), c["count"].as_u64().unwrap()))
        .collect();
    assert_eq!(counts.get("ci"), Some(&2), "{by_tag}");
    assert_eq!(counts.get("flake"), Some(&1), "{by_tag}");
    assert_eq!(counts.get("(untagged)"), Some(&1), "{by_tag}");
    assert_eq!(
        by_tag["overlapping"], true,
        "four tag slots over three entities really do overlap: {by_tag}"
    );

    // Same data, a field where each entity lands in exactly one group.
    let by_status = parse(&run_jjj_success(
        &dir,
        &["query", "type:problem", "--count-by", "status", "--json"],
    ));
    assert_eq!(
        by_status["overlapping"], false,
        "one status per entity cannot overlap: {by_status}"
    );

    // A tag count over an all-untagged set must not claim overlap either.
    let untagged = parse(&run_jjj_success(
        &dir,
        &["query", "Untagged one", "--count-by", "tag", "--json"],
    ));
    assert_eq!(untagged["matches"], 1, "{untagged}");
    assert_eq!(
        untagged["overlapping"], false,
        "one untagged entity sums to exactly one: {untagged}"
    );

    // The limit applies to listings, not to a count of the whole set.
    let capped = parse(&run_jjj_success(
        &dir,
        &[
            "query",
            "type:problem limit:1",
            "--count-by",
            "status",
            "--json",
        ],
    ));
    assert_eq!(
        capped["matches"], 3,
        "a limit must not silently shrink a count: {capped}"
    );

    // An unsupported field is refused rather than silently counted as zero.
    let out = run_jjj(&dir, &["query", "type:problem", "--count-by", "colour"]);
    assert!(!out.status.success(), "an unknown field must be refused");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("status") && err.contains("tag"),
        "the refusal must name the fields that do work: {err}"
    );
}
