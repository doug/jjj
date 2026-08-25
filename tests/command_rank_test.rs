//! Ranking must be expressible without a terminal.
//!
//! `jjj rank` had only `show`: an ordering could be read from anywhere but
//! authored only through the TUI's nudge and gap keys. That left half of jjj's
//! model — which work matters most, and by how much — unusable by exactly the
//! callers it was designed to coordinate. Four swarm trials produced zero
//! rankings, not because agents did not need to prioritise but because they had
//! no way to say so.

mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

fn with_three_problems() -> tempfile::TempDir {
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["milestone", "new", "M1"]);
    for p in ["Alpha", "Beta", "Gamma"] {
        run_jjj_success(&dir, &["problem", "new", p, "--milestone", "M1", "--force"]);
    }
    dir
}

#[test]
fn an_ordering_can_be_set_from_the_cli() {
    if !jj_available() {
        return;
    }
    let dir = with_three_problems();
    let out = run_jjj_success(&dir, &["rank", "set", "Gamma", "Alpha", "Beta"]);
    assert!(out.contains("Ranked 3"), "{out}");

    let shown = run_jjj_success(&dir, &["rank", "show"]);
    let gamma = shown.find("Gamma").expect("Gamma ranked");
    let beta = shown.find("Beta").expect("Beta ranked");
    assert!(
        gamma < beta,
        "the authored order should drive the aggregate: {shown}"
    );
}

#[test]
fn a_gap_expresses_a_priority_cliff() {
    if !jj_available() {
        return;
    }
    let dir = with_three_problems();
    run_jjj_success(
        &dir,
        &["rank", "set", "Alpha", "Beta", "Gamma", "--gap", "Alpha:XL"],
    );
    let json = run_jjj_success(
        &dir,
        &[
            "rank", "set", "Alpha", "Beta", "--gap", "Alpha:XL", "--json",
        ],
    );
    assert!(
        json.contains("XL"),
        "the gap must survive into the stored ordering: {json}"
    );
}

#[test]
fn move_shifts_one_item_without_restating_the_rest() {
    if !jj_available() {
        return;
    }
    let dir = with_three_problems();
    run_jjj_success(&dir, &["rank", "set", "Alpha", "Beta", "Gamma"]);
    let out = run_jjj_success(&dir, &["rank", "move", "Gamma", "top"]);
    let g = out.find("Gamma").expect("Gamma listed");
    let a = out.find("Alpha").expect("Alpha listed");
    assert!(g < a, "Gamma should now outrank Alpha: {out}");
}

#[test]
fn a_repeated_problem_is_refused() {
    if !jj_available() {
        return;
    }
    let dir = with_three_problems();
    // An ordering is a sequence. Silently accepting a duplicate would make the
    // stored priority depend on which occurrence won.
    let out = run_jjj(&dir, &["rank", "set", "Alpha", "Alpha"]);
    assert!(!out.status.success(), "a duplicate must be refused");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("twice"),
        "the error should say what is wrong: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn moving_without_an_ordering_says_so() {
    if !jj_available() {
        return;
    }
    let dir = with_three_problems();
    let out = run_jjj(&dir, &["rank", "move", "Alpha", "top"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("rank set"),
        "the error should name the way forward: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
