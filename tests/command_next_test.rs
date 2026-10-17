mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

#[test]
fn test_next_basic() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Problem"]);

    let stdout = run_jjj_success(&dir, &["next"]);
    assert!(
        stdout.contains("TODO") || stdout.contains("Problem"),
        "Expected next action: {}",
        stdout
    );
}

#[test]
fn test_next_empty_repo() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next"]);
    assert!(
        stdout.contains("Nothing to do") || stdout.contains("all caught up"),
        "Expected empty message: {}",
        stdout
    );
}

#[test]
fn test_next_claim() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Claimable Problem"]);

    let stdout = run_jjj_success(&dir, &["next", "--claim"]);
    assert!(
        stdout.contains("Claimed"),
        "Expected 'Claimed' in output: {}",
        stdout
    );
    assert!(
        stdout.contains("Claimable Problem"),
        "Expected problem title in output: {}",
        stdout
    );

    // Verify assignment by checking the problem
    let show_stdout = run_jjj_success(&dir, &["problem", "show", "Claimable", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&show_stdout).expect("Failed to parse JSON");
    assert!(
        json["assignee"].is_string(),
        "Expected assignee to be set after claim: {:?}",
        json["assignee"]
    );
}

#[test]
fn test_next_claim_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "JSON Claim"]);

    let stdout = run_jjj_success(&dir, &["next", "--claim", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    assert!(
        json["title"].as_str().unwrap_or("").contains("JSON Claim"),
        "Expected title in JSON output: {}",
        stdout
    );
}

#[test]
fn test_next_claim_empty() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next", "--claim"]);
    assert!(
        stdout.contains("Nothing to claim"),
        "Expected 'Nothing to claim' on empty repo: {}",
        stdout
    );
}

#[test]
fn test_next_claim_empty_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let stdout = run_jjj_success(&dir, &["next", "--claim", "--json"]);
    assert!(
        stdout.trim() == "null",
        "Expected null JSON on empty repo: {}",
        stdout
    );
}

/// A problem waiting on unfinished sub-problems must not be offered as work.
///
/// Before this rule, `jjj next` ranked such a parent by its own priority and
/// handed it over with every child still open — demonstrably the top of the
/// queue in a three-problem repository, with nothing anywhere saying it was not
/// ready. The agent then either flounders or silently does the children's work
/// without recording it against them.
#[test]
fn next_does_not_offer_a_parent_waiting_on_its_children() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let parent = {
        let out = run_jjj_success(
            &dir,
            &[
                "problem",
                "new",
                "Ship the release",
                "--priority",
                "critical",
                "--json",
            ],
        );
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        v["id"].as_str().expect("id").to_string()
    };
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Fix the failing test",
            "--parent",
            &parent,
            "--priority",
            "low",
        ],
    );

    let queue = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        queue.contains("Fix the failing test"),
        "the child is actionable and must be offered: {queue}"
    );
    assert!(
        !queue.contains("Ship the release"),
        "the waiting parent must not be offered, even at critical priority: {queue}"
    );

    // And it comes back once the child is finished, or the parent is lost.
    run_jjj_success(
        &dir,
        &[
            "problem",
            "dissolve",
            "Fix the failing test",
            "--rationale",
            "moot",
        ],
    );
    let after = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        after.contains("Ship the release"),
        "the parent must return to the queue once nothing blocks it: {after}"
    );
}

/// Dropping work from the queue silently is its own failure, so it has to be
/// visible somewhere.
#[test]
fn a_waiting_parent_is_discoverable() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let parent = {
        let out = run_jjj_success(&dir, &["problem", "new", "Epic", "--json"]);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        v["id"].as_str().expect("id").to_string()
    };
    run_jjj_success(&dir, &["problem", "new", "Step one", "--parent", &parent]);

    let shown = run_jjj_success(&dir, &["problem", "show", &parent]);
    assert!(
        shown.contains("Waiting on 1 unfinished sub-problem"),
        "`problem show` must say why it is not offered: {shown}"
    );

    // `waiting` lists held-back parents, and names the children holding them.
    //
    // This assertion used to be `!waiting.contains("Step one")` — the child must
    // not appear at all. That was right when a row was the only thing the
    // listing could print, and it broke when blockers were added beneath each
    // parent. The property it was protecting is still real, so it is asserted
    // more precisely rather than relaxed: the child must not be offered as a
    // held-back parent (a row of its own), and must be named as the blocker
    // (an indented `←` line). A plain `contains` cannot tell those apart.
    let waiting = run_jjj_success(&dir, &["query", "waiting"]);
    let rows: Vec<&str> = waiting.lines().filter(|l| l.starts_with("p/")).collect();
    assert_eq!(
        rows.len(),
        1,
        "exactly one parent is held back, so exactly one row: {waiting}"
    );
    assert!(
        rows[0].contains("Epic"),
        "the row must be the parent, not the child: {waiting}"
    );
    let blockers: Vec<&str> = waiting.lines().filter(|l| l.contains('←')).collect();
    assert_eq!(
        blockers.len(),
        1,
        "one open child, one blocker line: {waiting}"
    );
    assert!(
        blockers[0].contains("Step one"),
        "the blocker line must name the child to go work: {waiting}"
    );
}

// ---------------------------------------------------------------------------
// Lanes, and work that is waiting on a person
// ---------------------------------------------------------------------------

/// Several agents sharing one queue collide on the same top item. A lane is the
/// cheap fix that needs no coordinator: tag the work, give each agent a tag.
#[test]
fn a_lane_tag_restricts_what_is_offered() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Alpha work", "--tags", "lane-a"]);
    run_jjj_success(&dir, &["problem", "new", "Beta work", "--tags", "lane-b"]);

    let both = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        both.contains("Alpha work") && both.contains("Beta work"),
        "{both}"
    );

    let a = run_jjj_success(&dir, &["next", "--top", "0", "--tag", "lane-a"]);
    assert!(a.contains("Alpha work"), "lane-a should offer Alpha: {a}");
    assert!(!a.contains("Beta work"), "lane-a offered another lane: {a}");

    let b = run_jjj_success(&dir, &["next", "--top", "0", "--tag", "lane-b"]);
    assert!(b.contains("Beta work") && !b.contains("Alpha work"), "{b}");

    let empty = run_jjj_success(&dir, &["next", "--top", "0", "--tag", "lane-nobody"]);
    assert!(
        !empty.contains("Alpha work") && !empty.contains("Beta work"),
        "an empty lane must offer nothing rather than everything: {empty}"
    );
}

/// A lane follows the problem, so work *under* a tagged problem is in its lane
/// even though the entity itself carries no tag. Otherwise an agent holding a
/// lane would be offered the problem but never the review that follows it.
#[test]
fn a_lane_covers_work_under_a_tagged_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(
        &dir,
        &["problem", "new", "Tagged parent", "--tags", "lane-a"],
    );
    run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Child work",
            "--problem",
            "Tagged parent",
        ],
    );
    // A proposed solution is not itself an action; an open critique against it
    // is (category `blocked`), and that item points at the solution — which
    // carries no tag of its own.
    run_jjj_success(
        &dir,
        &[
            "critique",
            "new",
            "Child work",
            "Validates the cache, not the content",
        ],
    );

    let all = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        all.contains("Child work"),
        "expected a blocked solution item: {all}"
    );

    let lane = run_jjj_success(&dir, &["next", "--top", "0", "--tag", "lane-a"]);
    assert!(
        lane.contains("Child work"),
        "an untagged solution under a lane-a problem fell out of the lane: {lane}"
    );

    let other = run_jjj_success(&dir, &["next", "--top", "0", "--tag", "lane-b"]);
    assert!(
        !other.contains("Child work"),
        "it leaked into an unrelated lane: {other}"
    );
}

/// An escalation naming an entity means a person is needed *there*. Offering it
/// again just sends another agent into the same wall.
#[test]
fn an_escalated_item_is_not_offered_but_its_neighbours_are() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Alpha work"]);
    run_jjj_success(&dir, &["problem", "new", "Beta work"]);
    run_jjj_success(
        &dir,
        &[
            "escalate",
            "Needs a production credential",
            "--about",
            "Alpha work",
        ],
    );

    let out = run_jjj(&dir, &["next", "--top", "0"]);
    let text = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !text.contains("Alpha work"),
        "escalated work was still offered: {text}"
    );
    assert!(
        text.contains("Beta work"),
        "unrelated work must keep flowing: {text}"
    );
    assert!(
        err.contains("hidden") || text.contains("hidden"),
        "hiding work silently is worse than not hiding it: {err}{text}"
    );
}

/// A bare `jjj escalate "reason"` names nothing. Treating that as "stop all
/// work" would halt a whole fleet over one credential — stopping the fleet is
/// the supervisor's decision, with its own grace period, not a side effect here.
#[test]
fn a_global_escalation_does_not_halt_the_queue() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Gamma work"]);
    run_jjj_success(&dir, &["escalate", "The deploy key is missing"]);

    let out = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        out.contains("Gamma work"),
        "an untargeted escalation emptied the queue: {out}"
    );
}

/// And the suppression has to lift, or an escalation nobody clears silently
/// retires that work forever.
#[test]
fn clearing_an_escalation_returns_the_work_to_the_queue() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Alpha work"]);
    run_jjj_success(
        &dir,
        &["escalate", "Needs a human", "--about", "Alpha work"],
    );

    let json = run_jjj_success(&dir, &["escalate", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let id = v[0]["id"].as_str().expect("id").to_string();
    run_jjj_success(&dir, &["escalate", "--clear", &id[..8]]);

    let out = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        out.contains("Alpha work"),
        "cleared escalation did not restore the work: {out}"
    );
}

/// A refuted conjecture is knowledge, and the queue is where it has to surface.
///
/// Before this, a problem whose only solution had been withdrawn was offered as
/// "No solutions proposed" — false on its face, since `problem show` listed the
/// attempt, and actively harmful: the obvious first conjecture is the refuted
/// one, so a fleet re-proposes it in parallel and pays again for a measurement
/// someone already made. Found by walking the scenario rather than reading the
/// code, which is the only way the wording is visible at all.
#[test]
fn a_refuted_attempt_is_reported_not_erased() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let id_of = |out: &str| -> String {
        let v: serde_json::Value = serde_json::from_str(out).expect("valid JSON");
        v["id"].as_str().expect("id").to_string()
    };

    let problem = id_of(&run_jjj_success(
        &dir,
        &["problem", "new", "cache invalidation is wrong", "--json"],
    ));
    let solution = id_of(&run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "use a TTL",
            "--problem",
            &problem,
            "--json",
        ],
    ));

    // While the attempt is live the problem is not a TODO at all.
    run_jjj_success(&dir, &["solution", "submit", &solution]);
    run_jjj_success(&dir, &["solution", "withdraw", &solution, "--no-rationale"]);

    let queue = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        !queue.contains("No solutions proposed"),
        "an attempt was made and refuted — claiming none was proposed is false: {queue}"
    );
    assert!(
        queue.contains("1 refuted attempt"),
        "the queue must say the attempt happened: {queue}"
    );

    // And it must point at the refutation, not at the next write: the command
    // that proposes is already obvious, the one that reads is what gets skipped.
    assert!(
        queue.contains(&format!("jjj solution show {solution}")),
        "the pointer must lead to the refuted work: {queue}"
    );

    // An agent parsing JSON gets the list, not just the prose.
    let json = run_jjj_success(&dir, &["next", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(v["refuted"][0]["id"].as_str(), Some(solution.as_str()));
    assert_eq!(v["refuted"][0]["status"].as_str(), Some("withdrawn"));
}

/// The converse, so the fix cannot be "always say something was refuted".
#[test]
fn a_problem_nobody_has_attempted_still_says_so() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "untouched work"]);

    let queue = run_jjj_success(&dir, &["next", "--top", "0"]);
    assert!(
        queue.contains("No solutions proposed"),
        "nothing has been tried here, and the suggestion is to propose: {queue}"
    );
    assert!(
        queue.contains("jjj solution new"),
        "with no prior attempt the pointer is the proposing command: {queue}"
    );
}
