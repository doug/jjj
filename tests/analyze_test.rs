//! `tools/swarm/analyze.py` must work on a plain jjj repository.
//!
//! That is M3's whole point. Every misreading across six trials came through a
//! side channel — the invocation shim, container logs, an agent-local score —
//! so the coordination figures now come from jjj entities and the event log,
//! which is what any participant sees.
//!
//! The test that keeps it honest is therefore: build an ordinary jjj repository
//! with no swarm instrumentation whatsoever, and check the analyzer produces
//! real numbers from it. If a figure silently starts needing the shim again,
//! this goes quiet in exactly the way the shim-derived numbers used to.

mod test_helpers;

use std::path::PathBuf;
use std::process::Command;
use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

fn analyze_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join("swarm")
        .join("analyze.py")
}

fn json_id(s: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(s).expect("valid JSON");
    v["id"].as_str().expect("id field").to_string()
}

#[test]
fn analyze_reads_a_plain_jjj_repository() {
    if !jj_available() {
        return;
    }
    if Command::new("python3").arg("--version").output().is_err() {
        return;
    }

    let dir = setup_test_repo();

    // An ordinary repository: a milestone, a problem, a sub-problem, evidence,
    // two rival solutions, a critique, a withdrawal, an escalation.
    run_jjj_success(&dir, &["milestone", "new", "Make it fast"]);
    let milestones = run_jjj_success(&dir, &["milestone", "list", "--json"]);
    let mv: serde_json::Value = serde_json::from_str(&milestones).expect("valid JSON");
    let mid = mv[0]["id"].as_str().expect("milestone id").to_string();

    let pid = json_id(&run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Decode is slow",
            "--milestone",
            &mid,
            "--json",
        ],
    ));
    run_jjj_success(
        &dir,
        &[
            "problem",
            "new",
            "Header parse dominates",
            "--parent",
            &pid,
            "--milestone",
            &mid,
            "--json",
        ],
    );
    let fid = json_id(&run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            &pid,
            "The parse floors at a fixed cost",
            "--method",
            "counted, three runs",
            "--json",
        ],
    ));
    run_jjj_success(
        &dir,
        &[
            "solution", "new", "Memoize it", "--problem", &pid, "--cites", &fid, "--json",
        ],
    );
    let rival = json_id(&run_jjj_success(
        &dir,
        &[
            "solution",
            "new",
            "Precompute the table instead",
            "--problem",
            &pid,
            "--force",
            "--json",
        ],
    ));
    run_jjj_success(
        &dir,
        &[
            "solution",
            "withdraw",
            &rival,
            "--rationale",
            "superseded by Memoize: theirs reaches 120,004 ops against my 200,004",
        ],
    );
    run_jjj_success(&dir, &["escalate", "Needs production credentials"]);

    let out = Command::new("python3")
        .arg(analyze_path())
        .arg(dir.path())
        .env("JJJ_BIN", test_helpers::jjj_binary())
        .output()
        .expect("run analyze.py");
    let text = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "analyze.py failed on a plain jjj repository\nstdout:\n{text}\nstderr:\n{err}"
    );

    // It must recognise this as a repository, not a swarm root, and must not
    // complain about a missing invocation log.
    assert!(
        text.contains("jjj repository"),
        "Expected the plain-repository mode: {text}"
    );

    // Each of these is a coordination figure that used to come from the shim.
    assert!(
        text.contains("Participation") && text.contains("events from"),
        "participation must come from the event log: {text}"
    );
    assert!(
        text.contains("sub-problems"),
        "problem design must be derived: {text}"
    );
    assert!(
        text.contains("lost on the merits"),
        "the withdrawal classifier must run off event rationales: {text}"
    );
    assert!(
        text.contains("Evidence") && text.contains("cited by later work"),
        "citation coverage is M1's success criterion: {text}"
    );
    assert!(
        text.contains("1 raised"),
        "escalations must be derived from events: {text}"
    );

    // Every ratio names its denominator. The "137% withdrawn" reading came from
    // two different bases, and the fix is that the basis is always printed.
    assert!(
        text.contains("denominator:"),
        "ratios must state their basis: {text}"
    );

    // Questions jjj cannot answer are listed, not approximated from a channel
    // only the harness can see.
    assert!(
        text.contains("Not answerable from jjj"),
        "gaps must be reported rather than filled in from the shim: {text}"
    );

    // Harness sections are for swarm roots only — a plain repository has no
    // invocation log, and inventing one would be the exact regression this
    // milestone removes.
    assert!(
        !text.contains("invocation shim"),
        "harness sections must not appear for a plain repository: {text}"
    );
}
