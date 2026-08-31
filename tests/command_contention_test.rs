mod test_helpers;

use test_helpers::{jj_available, run_jjj_env, run_jjj_success, setup_test_repo};

fn json_id(s: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(s).expect("valid JSON");
    v["id"].as_str().expect("id field").to_string()
}

/// Set up a queue where three actors are on one problem and two have nobody.
fn contended_repo() -> (tempfile::TempDir, String, String) {
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["milestone", "new", "Cycle"]);
    let ms = run_jjj_success(&dir, &["milestone", "list", "--json"]);
    let mv: serde_json::Value = serde_json::from_str(&ms).expect("valid JSON");
    let mid = mv[0]["id"].as_str().expect("milestone id").to_string();

    let hot = json_id(&run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "The contended one",
            "--milestone",
            &mid,
            "--json",
        ],
    ));
    run_jjj_success(
        &dir,
        &["problem", "new", "Untouched A", "--milestone", &mid],
    );
    run_jjj_success(
        &dir,
        &["problem", "new", "Untouched B", "--milestone", &mid],
    );

    for (actor, title) in [
        ("agent-a", "Approach one"),
        ("agent-b", "Approach two"),
        ("agent-c", "Approach three"),
    ] {
        run_jjj_env(
            &dir,
            &[("JJJ_USER", actor)],
            &["solution", "new", title, "--problem", &hot, "--force"],
        );
    }
    (dir, mid, hot)
}

#[test]
fn test_contention_names_the_pile_up_and_the_empty_queue() {
    if !jj_available() {
        return;
    }
    let (dir, _mid, _hot) = contended_repo();

    let json = run_jjj_success(&dir, &["contention", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(
        v["contended"].as_array().map(|a| a.len()),
        Some(1),
        "one problem has three actors on it: {json}"
    );
    assert_eq!(
        v["contended"][0]["actors"].as_array().map(|a| a.len()),
        Some(3),
        "all three authors should be counted: {json}"
    );
    assert_eq!(
        v["untouched"].as_array().map(|a| a.len()),
        Some(2),
        "two problems have nobody: {json}"
    );
    assert_eq!(
        v["should_rebalance"], true,
        "doubled up with somewhere else to go is the rebalance case: {json}"
    );
}

/// Contention is only waste when the fleet has somewhere else to be. Three
/// rival conjectures on the only open problem is the method working.
#[test]
fn test_rivalry_on_the_only_problem_is_not_a_misallocation() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let only = json_id(&run_jjj_success(
        &dir,
        &["problem", "new", "The only problem", "--json"],
    ));
    for (actor, title) in [("agent-a", "One"), ("agent-b", "Two")] {
        run_jjj_env(
            &dir,
            &[("JJJ_USER", actor)],
            &["solution", "new", title, "--problem", &only, "--force"],
        );
    }

    let json = run_jjj_success(&dir, &["contention", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(
        v["should_rebalance"], false,
        "there is nowhere else for them to go: {json}"
    );

    let text = run_jjj_success(&dir, &["contention"]);
    assert!(
        text.contains("nowhere else"),
        "the report should say why nothing is wrong: {text}"
    );
}

/// The report's entire output is commands meant to be pasted. UUID7 ids are
/// time-ordered, so sibling problems seeded in the same second share their
/// first six characters — a fixed-length prefix printed identical, ambiguous
/// `rank move` lines.
#[test]
fn test_suggested_commands_use_unambiguous_prefixes() {
    if !jj_available() {
        return;
    }
    let (dir, _mid, _hot) = contended_repo();

    let text = run_jjj_success(&dir, &["contention"]);
    let suggested: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("jjj rank move"))
        .collect();
    assert!(!suggested.is_empty(), "expected suggestions: {text}");

    let ids: Vec<&str> = suggested
        .iter()
        .filter_map(|l| l.split_whitespace().nth(3))
        .collect();
    let unique: std::collections::HashSet<&&str> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "two suggestions named the same prefix, so at most one could run: {text}"
    );
}

/// Following the advice must actually work. `rank move` used to refuse for
/// anyone who had not already written a full ordering, which made it unusable
/// in exactly the situation it is suggested for.
#[test]
fn test_a_first_time_ranker_can_follow_the_advice() {
    if !jj_available() {
        return;
    }
    let (dir, _mid, _hot) = contended_repo();

    let text = run_jjj_success(&dir, &["contention"]);
    let id = text
        .lines()
        .find(|l| l.trim_start().starts_with("jjj rank move"))
        .and_then(|l| l.split_whitespace().nth(3))
        .expect("a suggested command")
        .to_string();

    let out = run_jjj_env(
        &dir,
        &[("JJJ_USER", "integrator")],
        &["rank", "move", &id, "top"],
    );
    assert!(
        out.status.success(),
        "the suggested command must run: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // "Shift one, keep the rest" — the rest must still be there.
    let ranked = run_jjj_success(&dir, &["rank", "show", "--json"]);
    let rows: serde_json::Value = serde_json::from_str(&ranked).expect("valid JSON");
    let rows = rows.as_array().expect("array");
    assert_eq!(
        rows.len(),
        3,
        "moving one problem must not drop the other two: {ranked}"
    );
    assert!(
        rows[0]["title"]
            .as_str()
            .unwrap_or("")
            .starts_with("Untouched"),
        "the untouched problem should now lead: {ranked}"
    );
}

/// A first-time ranker inherits the fleet's aggregate rather than starting from
/// creation order, which would silently contradict everyone who has ranked.
#[test]
fn test_bootstrap_inherits_the_existing_consensus() {
    if !jj_available() {
        return;
    }
    let (dir, _mid, hot) = contended_repo();

    let problems = run_jjj_success(&dir, &["problem", "list", "--json"]);
    let pv: serde_json::Value = serde_json::from_str(&problems).expect("valid JSON");
    let ids: Vec<String> = pv
        .as_array()
        .expect("array")
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    let others: Vec<&String> = ids.iter().filter(|i| **i != hot).collect();

    // agent-a puts the contended problem last.
    let mut order: Vec<&str> = others.iter().map(|s| s.as_str()).collect();
    order.push(&hot);
    let mut args = vec!["rank", "set"];
    args.extend(order.iter().copied());
    run_jjj_env(&dir, &[("JJJ_USER", "agent-a")], &args);

    // agent-b, who has never ranked, moves the contended problem up one.
    let out = run_jjj_env(
        &dir,
        &[("JJJ_USER", "agent-b")],
        &["rank", "move", &hot, "up"],
    );
    assert!(out.status.success(), "the move should bootstrap");

    let by_user = run_jjj_env(
        &dir,
        &[("JJJ_USER", "agent-b")],
        &["rank", "show", "--by-user", "--json"],
    );
    let text = String::from_utf8_lossy(&by_user.stdout);
    assert!(
        text.contains("agent-b"),
        "agent-b should now have an ordering: {text}"
    );
    // Inherited, not invented: agent-b's ordering covers the whole milestone.
    let stdout = run_jjj_env(
        &dir,
        &[("JJJ_USER", "agent-b")],
        &["rank", "show", "--json"],
    );
    let rows: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&stdout.stdout)).expect("valid JSON");
    assert_eq!(
        rows.as_array().map(|a| a.len()),
        Some(3),
        "the bootstrapped ordering must cover the milestone: {text}"
    );
}
