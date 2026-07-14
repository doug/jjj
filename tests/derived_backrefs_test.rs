//! M3 (Pillar 4) — derived back-references.
//!
//! The reverse-reference lists (`Problem::solution_ids`, `Solution::critique_ids`,
//! `Milestone::problem_ids`) are derived at read time from the forward refs and
//! never stored on disk. Consequences these tests lock:
//!   1. Creating a child touches ONE file — no parent-rewrite amplification.
//!   2. The back-ref never appears in the parent's markdown, but IS present in
//!      `--json` output.
//! (Deletion cleanup — removing a child no longer rewrites its parent — is
//! covered by the storage-layer delete tests; entity deletion has no CLI.)

mod test_helpers;

use std::path::{Path, PathBuf};
use test_helpers::{jj_available, run_jjj, setup_test_repo};

fn problem_files(dir: &Path) -> Vec<PathBuf> {
    let pdir = dir.join(".jj").join("jjj-meta").join("problems");
    std::fs::read_dir(&pdir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
                .collect()
        })
        .unwrap_or_default()
}

fn stdout(out: std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The core write-amplification win: adding a second solution to a problem that
/// is already in progress must NOT rewrite the problem's markdown file. (The
/// first solution legitimately flips Open→InProgress; the second has no reason
/// to touch the problem at all once `solution_ids` is derived.)
#[test]
fn test_creating_solution_does_not_rewrite_problem_file() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let dir = repo.path();

    run_jjj(dir, &["problem", "new", "Parent problem", "--force"]);
    // First solution transitions the problem to in_progress (a legit change).
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "First fix",
            "--problem",
            "Parent problem",
            "--force",
        ],
    );

    let pfile = problem_files(dir).into_iter().next().expect("problem file");
    let before = std::fs::read_to_string(&pfile).expect("read problem before");
    let before_mtime = std::fs::metadata(&pfile).unwrap().modified().unwrap();

    // Second solution: problem is already in_progress, so nothing should write
    // the problem file — no parent rewrite to append a solution id.
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Second fix",
            "--problem",
            "Parent problem",
            "--force",
        ],
    );

    let after = std::fs::read_to_string(&pfile).expect("read problem after");
    let after_mtime = std::fs::metadata(&pfile).unwrap().modified().unwrap();
    assert_eq!(
        before, after,
        "creating a solution must not rewrite the parent problem's markdown"
    );
    assert_eq!(
        before_mtime, after_mtime,
        "the problem file must not even be touched (no atomic-write churn)"
    );

    // But both solutions ARE visible as derived back-refs in --json.
    let json = stdout(run_jjj(
        dir,
        &["problem", "show", "Parent problem", "--json"],
    ));
    assert!(
        json.contains("solution_ids"),
        "problem --json should include the derived solution_ids. Got:\n{json}"
    );
}

/// The derived list is never persisted to the parent markdown, yet is present in
/// `--json`. Storing it would re-introduce the merge-conflict / amplification
/// problem M3 removes.
#[test]
fn test_backref_absent_from_markdown_present_in_json() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let dir = repo.path();

    run_jjj(dir, &["problem", "new", "Has solutions", "--force"]);
    run_jjj(
        dir,
        &[
            "solution",
            "new",
            "A fix",
            "--problem",
            "Has solutions",
            "--force",
        ],
    );

    let pfile = problem_files(dir).into_iter().next().expect("problem file");
    let md = std::fs::read_to_string(&pfile).expect("read problem md");
    assert!(
        !md.contains("solution_ids"),
        "solution_ids is derived and must never be written to markdown. Got:\n{md}"
    );

    let json = stdout(run_jjj(
        dir,
        &["problem", "show", "Has solutions", "--json"],
    ));
    assert!(
        json.contains("solution_ids"),
        "the derived solution_ids must still appear in --json output. Got:\n{json}"
    );
}
