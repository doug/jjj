//! Per-agent identity across the commands a fleet actually uses.
//!
//! jjj resolves an actor from `JJJ_USER` → pod → jj `user.name`, so several
//! agents can share one checkout and still be distinct writers. That resolution
//! existed for event authorship but the assignment and attribution paths read
//! the raw jj identity instead, which meant every agent on a machine appeared as
//! the same person: `--claim` assigned to the machine, `--mine` returned
//! everyone's work, and critiques were credited to the human who set up the repo.
//!
//! These tests pin the behaviour a multi-agent setup depends on.

mod test_helpers;

use test_helpers::{jj_available, run_jjj_env, run_jjj_success, setup_test_repo};

/// Parse `next --json`, which emits `null`, one object, or an array.
fn queue_titles(json: &str) -> Vec<String> {
    let value: serde_json::Value =
        serde_json::from_str(json.trim()).unwrap_or(serde_json::Value::Null);
    let items = match value {
        serde_json::Value::Null => vec![],
        serde_json::Value::Array(items) => items,
        object => vec![object],
    };
    items
        .iter()
        .filter_map(|i| i["title"].as_str().map(str::to_string))
        .collect()
}

fn as_agent(dir: &std::path::Path, agent: &str, args: &[&str]) -> String {
    let out = run_jjj_env(dir, &[("JJJ_USER", agent)], args);
    assert!(
        out.status.success(),
        "jjj {} failed as {agent}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn claim_assigns_to_the_env_identity_not_the_machine() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Only job", "--force"]);

    as_agent(repo.path(), "agent-a", &["next", "--claim"]);

    let json = run_jjj_success(repo.path(), &["problem", "list", "--json"]);
    let listed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        listed[0]["assignee"].as_str(),
        Some("agent-a"),
        "a claim must be attributed to the acting agent, not the jj user"
    );
}

#[test]
fn mine_is_scoped_per_agent() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    for title in ["Job A", "Job B", "Job C"] {
        run_jjj_success(repo.path(), &["problem", "new", title, "--force"]);
    }
    run_jjj_success(
        repo.path(),
        &["problem", "assign", "Job A", "--to", "agent-a"],
    );
    run_jjj_success(
        repo.path(),
        &["problem", "assign", "Job B", "--to", "agent-b"],
    );

    let a = queue_titles(&as_agent(
        repo.path(),
        "agent-a",
        &["next", "--top", "0", "--mine", "--json"],
    ));
    let b = queue_titles(&as_agent(
        repo.path(),
        "agent-b",
        &["next", "--top", "0", "--mine", "--json"],
    ));
    let c = queue_titles(&as_agent(
        repo.path(),
        "agent-c",
        &["next", "--top", "0", "--mine", "--json"],
    ));

    assert_eq!(a, vec!["Job A"], "each agent sees only its own work");
    assert_eq!(b, vec!["Job B"]);
    assert!(
        c.is_empty(),
        "an agent with no assignments has an empty queue"
    );

    // Unowned work ("Job C") is nobody's queue but is still visible without the
    // flag — otherwise a fleet could never pick up new work.
    let all = queue_titles(&as_agent(
        repo.path(),
        "agent-c",
        &["next", "--top", "0", "--json"],
    ));
    assert!(
        all.iter().any(|t| t == "Job C"),
        "unassigned work must remain claimable: {all:?}"
    );
}

#[test]
fn critiques_are_credited_to_the_acting_agent() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Needs review", "--force"]);
    run_jjj_success(
        repo.path(),
        &[
            "solution",
            "new",
            "An approach",
            "--problem",
            "Needs review",
            "--force",
        ],
    );

    as_agent(
        repo.path(),
        "critic-1",
        &[
            "critique",
            "new",
            "An approach",
            "Missing a test",
            "--severity",
            "high",
        ],
    );

    let json = run_jjj_success(repo.path(), &["critique", "list", "--json"]);
    let listed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        listed[0]["author"].as_str(),
        Some("critic-1"),
        "critique authorship is the record of who objected — it must name the agent"
    );
}

#[test]
fn a_pod_gets_its_own_identity_and_push_bookmark() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();

    // JJJ_POD alone is enough: a pod that sets only its pod id still needs a
    // stable identity, and its own single-writer bookmark so parallel pushes
    // never contend for one ref.
    let out = run_jjj_env(repo.path(), &[("JJJ_POD", "pod-7")], &["whoami", "--json"]);
    let who: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();

    assert_eq!(who["actor"].as_str(), Some("pod-7"));
    assert_eq!(who["pod"].as_str(), Some("pod-7"));
    assert_eq!(who["push_bookmark"].as_str(), Some("jjj/pod-7"));
}

#[test]
fn jjj_user_takes_precedence_over_the_pod() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();

    let out = run_jjj_env(
        repo.path(),
        &[("JJJ_POD", "pod-7"), ("JJJ_USER", "reviewer")],
        &["whoami", "--json"],
    );
    let who: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();

    // A supervisor naming an agent should win over the pod it happens to run in.
    assert_eq!(who["actor"].as_str(), Some("reviewer"));
    assert_eq!(who["pod"].as_str(), Some("pod-7"));
}

#[test]
fn events_record_the_agent_that_acted() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    as_agent(
        repo.path(),
        "agent-a",
        &["problem", "new", "From A", "--force"],
    );
    as_agent(
        repo.path(),
        "agent-b",
        &["problem", "new", "From B", "--force"],
    );

    let events = run_jjj_success(repo.path(), &["events", "--json"]);
    assert!(
        events.contains("agent-a") && events.contains("agent-b"),
        "the log is the record of who did what across a fleet: {events}"
    );
}

#[test]
fn a_legacy_assignee_still_matches_its_owner() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Old work", "--force"]);

    // Pre-0.5.1 wrote the full jj identity as the assignee. After the upgrade
    // the actor resolves to a bare name, and `--mine` must still find it —
    // otherwise upgrading silently empties everyone's queue.
    run_jjj_success(
        repo.path(),
        &[
            "problem",
            "assign",
            "Old work",
            "--to",
            "Alice <alice@example.com>",
        ],
    );

    let mine = queue_titles(&as_agent(
        repo.path(),
        "Alice",
        &["next", "--top", "0", "--mine", "--json"],
    ));
    assert_eq!(
        mine,
        vec!["Old work"],
        "a bare actor name must match an assignee stored as `Name <email>`"
    );
}
