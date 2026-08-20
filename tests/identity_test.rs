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
    assert_eq!(who["push_bookmark"].as_str(), Some("jjj-pod-7"));
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

// =============================================================================
// Claims are leases, not locks (design decision 15)
// =============================================================================

/// Work another agent is actively holding must not be offered to anyone else.
///
/// Before this, `jjj next` showed every open item regardless of who held it, so
/// a fleet starting together was handed the same top item and all of them
/// claimed it — measured in the first swarm trial, where four of four agents
/// claimed one problem.
#[test]
fn live_claims_are_not_offered_to_other_agents() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Only job", "--force"]);

    as_agent(repo.path(), "agent-a", &["next", "--claim"]);

    let others = queue_titles(&as_agent(
        repo.path(),
        "agent-b",
        &["next", "--top", "0", "--json"],
    ));
    assert!(
        others.is_empty(),
        "agent-b was offered work agent-a is actively holding: {others:?}"
    );

    // The holder still sees it — losing sight of your own claim would be worse.
    let mine = queue_titles(&as_agent(
        repo.path(),
        "agent-a",
        &["next", "--top", "0", "--json"],
    ));
    assert_eq!(
        mine,
        vec!["Only job"],
        "an agent must still see its own claim"
    );
}

/// A lapsed claim returns to the pool, so a dead agent cannot strand work.
#[test]
fn a_stale_claim_is_reclaimable() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Abandoned", "--force"]);

    as_agent(repo.path(), "agent-a", &["next", "--claim"]);

    // Age the claim past the lease by rewriting it on disk, which is what an
    // agent dying an hour ago looks like.
    let problems = repo.path().join(".jj/jjj-meta/problems");
    let file = std::fs::read_dir(&problems)
        .expect("problems dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        .expect("a problem file");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("claimed_at"),
        "a claim must record when it was taken, or it can never expire:\n{text}"
    );
    let long_ago = (chrono::Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
    let aged: String = text
        .lines()
        .map(|l| {
            if l.starts_with("claimed_at:") {
                format!("claimed_at: '{long_ago}'")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&file, aged).unwrap();
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(repo.path().join(format!(".jj/jjj.db{suffix}")));
    }

    let others = queue_titles(&as_agent(
        repo.path(),
        "agent-b",
        &["next", "--top", "0", "--json"],
    ));
    assert_eq!(
        others,
        vec!["Abandoned"],
        "a lapsed claim must return to the pool, or a dead agent holds work forever"
    );

    // And it can actually be taken.
    as_agent(repo.path(), "agent-b", &["next", "--claim"]);
    let json = run_jjj_success(repo.path(), &["problem", "list", "--json"]);
    let listed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(listed[0]["assignee"].as_str(), Some("agent-b"));
}

/// A deliberate hand-off is not a claim and must never expire out from under
/// the person it was given to.
#[test]
fn an_assignment_without_a_claim_does_not_expire() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Handed over", "--force"]);
    run_jjj_success(
        repo.path(),
        &["problem", "assign", "Handed over", "--to", "alice"],
    );

    let others = queue_titles(&as_agent(
        repo.path(),
        "agent-b",
        &["next", "--top", "0", "--json"],
    ));
    assert!(
        others.is_empty(),
        "an explicit assignment has no lease and must stay with its owner: {others:?}"
    );
}

/// A fleet of agents looking at the same fresh backlog must not all pick the
/// same item.
///
/// Everything `next` sorts by — category, priority, age — is identical for every
/// agent, so with a uniformly-seeded backlog they were all handed the same head
/// of the list and all claimed it: nine agents, twenty-three claim attempts,
/// three distinct items. The final tiebreak is per-actor so they spread out
/// without coordinating.
#[test]
fn different_agents_prefer_different_work_when_everything_ties() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    for i in 0..8 {
        run_jjj_success(
            repo.path(),
            &[
                "problem",
                "new",
                &format!("Task {i}"),
                "--priority",
                "high",
                "--force",
            ],
        );
    }

    // What each agent would be handed first, with nothing claimed yet.
    let mut heads = std::collections::HashSet::new();
    for agent in [
        "agent-01", "agent-02", "agent-03", "agent-04", "agent-05", "agent-06",
    ] {
        let titles = queue_titles(&as_agent(repo.path(), agent, &["next", "--json"]));
        if let Some(first) = titles.first() {
            heads.insert(first.clone());
        }
    }

    assert!(
        heads.len() > 1,
        "every agent was offered the same item, so a fleet stampedes one problem \
         and leaves the rest untouched; got {heads:?}"
    );
}

/// The spread must be stable: an agent's own view cannot reshuffle between
/// turns, or it abandons work half-done.
#[test]
fn an_agents_preference_is_stable_across_turns() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    for i in 0..6 {
        run_jjj_success(
            repo.path(),
            &[
                "problem",
                "new",
                &format!("Job {i}"),
                "--priority",
                "high",
                "--force",
            ],
        );
    }

    let first = queue_titles(&as_agent(repo.path(), "agent-01", &["next", "--json"]));
    for _ in 0..3 {
        let again = queue_titles(&as_agent(repo.path(), "agent-01", &["next", "--json"]));
        assert_eq!(
            first, again,
            "an agent's preferred item changed between turns; it would keep \
             switching tasks and finish nothing"
        );
    }
}

// =============================================================================
// CLI consistency — flags agents reasonably expect to exist
// =============================================================================

/// `--mine` must work everywhere it reads as if it should.
///
/// A nine-agent trial produced 274 failing invocations, and roughly 150 were
/// agents calling flags that do not exist but plausibly ought to: `solution
/// list --mine` 76 times, because `next --mine` and `critique list --mine` both
/// exist. Agents probe an API far more systematically than people do, and they
/// had inferred the consistent surface jjj did not have.
#[test]
fn mine_works_on_every_listing() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Owned", "--force"]);
    run_jjj_success(
        repo.path(),
        &["problem", "assign", "Owned", "--to", "agent-a"],
    );
    run_jjj_success(
        repo.path(),
        &[
            "solution",
            "new",
            "An approach",
            "--problem",
            "Owned",
            "--force",
        ],
    );
    run_jjj_success(
        repo.path(),
        &["solution", "assign", "An approach", "--to", "agent-a"],
    );

    let problems = as_agent(
        repo.path(),
        "agent-a",
        &["problem", "list", "--mine", "--json"],
    );
    assert!(
        problems.contains("Owned"),
        "problem list --mine: {problems}"
    );

    let solutions = as_agent(
        repo.path(),
        "agent-a",
        &["solution", "list", "--mine", "--json"],
    );
    assert!(
        solutions.contains("An approach"),
        "solution list --mine: {solutions}"
    );

    // And it must actually filter, not just be accepted and ignored.
    let others = as_agent(
        repo.path(),
        "agent-b",
        &["problem", "list", "--mine", "--json"],
    );
    assert!(
        !others.contains("Owned"),
        "--mine returned another actor's work: {others}"
    );
}

/// The log must answer "what did that agent do?" — the first question anyone
/// asks of a swarm, and one `jjj events` could not answer.
#[test]
fn events_can_be_filtered_by_actor() {
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

    let a_only = as_agent(
        repo.path(),
        "agent-a",
        &["events", "--user", "agent-a", "--json"],
    );
    assert!(a_only.contains("agent-a"), "expected agent-a's events");
    assert!(
        !a_only.contains("agent-b"),
        "--user leaked another actor's events: {a_only}"
    );

    let mine = as_agent(repo.path(), "agent-b", &["events", "--mine", "--json"]);
    assert!(mine.contains("agent-b"));
    assert!(!mine.contains("agent-a"), "--mine leaked others: {mine}");
}

/// A sign-off asserts the work is correct, so it must be able to carry the
/// evidence — reviewers reach for this by analogy with `approve --rationale`.
#[test]
fn lgtm_records_a_rationale() {
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
    run_jjj_success(
        repo.path(),
        &[
            "critique",
            "new",
            "An approach",
            "Please review",
            "--reviewer",
            "reviewer-1",
        ],
    );

    as_agent(
        repo.path(),
        "reviewer-1",
        &[
            "solution",
            "lgtm",
            "An approach",
            "--rationale",
            "ran the suite; all cases pass",
        ],
    );

    let critiques = run_jjj_success(repo.path(), &["critique", "list", "--json"]);
    assert!(
        critiques.contains("all cases pass"),
        "the sign-off's evidence was discarded: {critiques}"
    );
}

// =============================================================================
// Pull-based review: signing off on work nobody assigned you
// =============================================================================

/// A reviewer must be able to sign off on work they picked up themselves.
///
/// `lgtm` originally required a review critique *already assigned* to the caller,
/// which only fits a push model — an author names a reviewer, the reviewer signs
/// off. Review in a fleet is pull-based: critics take whatever is submitted, so
/// nobody assigns them and the command was unusable. 49 of 92 calls failed in one
/// trial, and the error advised filing a critique against work believed correct,
/// which would fill the objection record with fake objections.
#[test]
fn a_reviewer_can_sign_off_on_self_selected_work() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    as_agent(
        repo.path(),
        "builder-1",
        &["problem", "new", "Add collatz", "--force"],
    );
    as_agent(
        repo.path(),
        "builder-1",
        &[
            "solution",
            "new",
            "Iterative collatz",
            "--problem",
            "Add collatz",
            "--force",
        ],
    );
    as_agent(
        repo.path(),
        "builder-1",
        &["solution", "submit", "Iterative collatz"],
    );

    // Nobody assigned critic-1 anything; it simply took work off the queue.
    let out = as_agent(
        repo.path(),
        "critic-1",
        &[
            "solution",
            "lgtm",
            "Iterative collatz",
            "--rationale",
            "ran the suite; all cases pass",
        ],
    );
    assert!(out.contains("Signed off"), "sign-off failed: {out}");

    // The review must be on the record, with its evidence.
    let critiques = run_jjj_success(repo.path(), &["critique", "list", "--json"]);
    assert!(
        critiques.contains("critic-1"),
        "no record of who reviewed: {critiques}"
    );
    assert!(
        critiques.contains("all cases pass"),
        "the evidence behind the sign-off was lost: {critiques}"
    );

    // And the solution is now approvable.
    let approved = as_agent(
        repo.path(),
        "builder-1",
        &["solution", "approve", "Iterative collatz", "--no-rationale"],
    );
    assert!(approved.contains("approved"), "{approved}");
}

/// An agent must never sign off its own conjecture.
///
/// Making self-selected review easy must not make self-approval easy. The
/// critique gate is the one thing stopping a fleet approving its own homework,
/// and it is worth more than the convenience.
#[test]
fn an_author_cannot_sign_off_their_own_solution() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    as_agent(
        repo.path(),
        "builder-1",
        &["problem", "new", "Add gcd", "--force"],
    );
    as_agent(
        repo.path(),
        "builder-1",
        &[
            "solution",
            "new",
            "Euclid",
            "--problem",
            "Add gcd",
            "--force",
        ],
    );
    as_agent(repo.path(), "builder-1", &["solution", "submit", "Euclid"]);

    let out = run_jjj_env(
        repo.path(),
        &[("JJJ_USER", "builder-1")],
        &["solution", "lgtm", "Euclid"],
    );
    assert!(
        !out.status.success(),
        "an agent signed off its own work — the review gate is bypassable"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("you wrote"),
        "the refusal should say why: {stderr}"
    );
}

/// Only submitted work can be signed off; a proposal is not up for review yet.
#[test]
fn unsubmitted_work_cannot_be_signed_off() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    as_agent(
        repo.path(),
        "builder-1",
        &["problem", "new", "Add luhn", "--force"],
    );
    as_agent(
        repo.path(),
        "builder-1",
        &[
            "solution",
            "new",
            "Checksum",
            "--problem",
            "Add luhn",
            "--force",
        ],
    );

    let out = run_jjj_env(
        repo.path(),
        &[("JJJ_USER", "critic-1")],
        &["solution", "lgtm", "Checksum"],
    );
    assert!(
        !out.status.success(),
        "signed off work that was never submitted"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("submitted"),
        "the refusal should name the problem"
    );
}
