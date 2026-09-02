mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

/// The failure this exists for: an agent blocked on something only a person can
/// fix, with no way to say so. It must raise, and it must be visible afterwards.
#[test]
fn test_escalate_raises_and_lists() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let raised = run_jjj_success(&dir, &["escalate", "OAuth token expired; needs re-auth"]);
    assert!(
        raised.contains("Escalated"),
        "Expected confirmation: {raised}"
    );

    // The bare form answers "is anyone blocked", so a supervisor does not have
    // to remember a flag.
    let listed = run_jjj_success(&dir, &["escalate"]);
    assert!(
        listed.contains("OAuth token expired"),
        "Expected the escalation to be listed: {listed}"
    );
}

/// Ranked above everything in `status`: a fleet that has stopped making progress
/// must not have that fact sorted below its next actions. That is exactly how a
/// 6.8-hour outage went unnoticed.
#[test]
fn test_escalations_lead_the_status_output() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    run_jjj_success(&dir, &["problem", "new", "Some ordinary work"]);
    run_jjj_success(&dir, &["escalate", "Credential is dead"]);

    let status = run_jjj_success(&dir, &["status"]);
    let escalation_at = status
        .find("Credential is dead")
        .expect("escalation must appear in status");
    let actions_at = status.find("Next actions").unwrap_or(usize::MAX);
    assert!(
        escalation_at < actions_at,
        "The escalation must come before the work queue:\n{status}"
    );
    assert!(
        status.contains("a person is needed"),
        "Expected the escalation banner to say what it wants: {status}"
    );
}

#[test]
fn test_escalations_are_first_in_the_status_json() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["escalate", "Needs a human"]);

    let json = run_jjj_success(&dir, &["status", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(parsed["escalations"][0]["reason"], "Needs a human");
    assert_eq!(
        parsed["summary"]["open_escalations"], 1,
        "a supervisor script should be able to read one number: {json}"
    );
}

/// Clearing is what a person does after acting. Without it the banner is
/// permanent and stops meaning anything.
#[test]
fn test_clearing_removes_it() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let json = run_jjj_success(&dir, &["escalate", "Blocked on a secret", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let id = parsed["id"].as_str().expect("id in JSON");

    run_jjj_success(&dir, &["escalate", "--clear", &id[..8]]);

    let listed = run_jjj_success(&dir, &["escalate"]);
    assert!(
        listed.contains("No open escalations"),
        "Expected the escalation to be cleared: {listed}"
    );

    let status = run_jjj_success(&dir, &["status"]);
    assert!(
        !status.contains("a person is needed"),
        "A cleared escalation must stop leading status: {status}"
    );
}

/// Clearing something that is not open is almost always a typo, and silently
/// succeeding would leave the real escalation standing while the operator
/// believes they handled it.
#[test]
fn test_clearing_an_unknown_escalation_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["escalate", "Real blocker"]);

    let out = run_jjj(&dir, &["escalate", "--clear", "deadbeef"]);
    assert!(!out.status.success(), "Expected an unknown id to fail");

    let listed = run_jjj_success(&dir, &["escalate"]);
    assert!(
        listed.contains("Real blocker"),
        "The real escalation must survive a failed clear: {listed}"
    );
}

/// An escalation with no reason is noise. The point of the channel is that
/// someone reading it knows what to do.
#[test]
fn test_an_empty_reason_is_refused() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj(&dir, &["escalate", "   "]);
    assert!(!out.status.success(), "Expected a blank reason to fail");
}

/// `--about` lets an escalation name what it concerns, resolved across kinds
/// because what blocks an agent is rarely known to be one kind in advance.
#[test]
fn test_escalation_can_name_what_it_is_about() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Deploy the thing"]);

    let json = run_jjj_success(
        &dir,
        &[
            "escalate",
            "Needs production credentials",
            "--about",
            "Deploy the thing",
            "--json",
        ],
    );
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(
        parsed["about"].as_array().map(|a| a.len()),
        Some(1),
        "Expected the referenced problem: {json}"
    );
}

/// `doctor` is where someone looks when a repository is misbehaving, so the one
/// condition nothing in the system can clear by itself has to appear there.
#[test]
fn test_doctor_reports_an_open_escalation() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let clean = run_jjj_success(&dir, &["doctor"]);
    assert!(
        clean.contains("none open"),
        "a healthy repo should say so: {clean}"
    );

    run_jjj_success(&dir, &["escalate", "The deploy key is missing"]);

    let out = run_jjj(&dir, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("The deploy key is missing"),
        "doctor must surface the escalation: {text}"
    );
    assert!(
        text.contains("jjj escalate --clear"),
        "and say how to clear it: {text}"
    );

    // Cleared escalations must stop being reported, or the signal decays into
    // noise nobody reads.
    let json = run_jjj_success(&dir, &["escalate", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let id = v[0]["id"].as_str().expect("id").to_string();
    run_jjj_success(&dir, &["escalate", "--clear", &id[..8]]);

    let after = run_jjj_success(&dir, &["doctor"]);
    assert!(
        after.contains("none open"),
        "a cleared escalation must stop being reported: {after}"
    );
}
