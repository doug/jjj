//! Crash, lock, and corruption recovery.
//!
//! jjj has real durability machinery — `atomic_write` (tmp + rename), an
//! flock'd repo-wide write lock, WAL with a busy timeout, a dirty-cache heal
//! path — and none of it had a test that actually broke something. Engineering
//! for a failure you never induce is a hypothesis, not a guarantee.
//!
//! Each test here damages the repository the way a real incident would (kill a
//! writer mid-run, hold the lock from another process, truncate the cache) and
//! asserts that the next ordinary command still works.

mod test_helpers;

use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

/// Spawn jjj without waiting for it.
fn spawn_jjj(dir: &std::path::Path, args: &[&str]) -> std::process::Child {
    Command::new(test_helpers::jjj_binary())
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jjj")
}

// =============================================================================
// Crash mid-write
// =============================================================================

#[test]
fn a_writer_killed_mid_run_leaves_a_readable_store() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(
        repo.path(),
        &["problem", "new", "Established work", "--force"],
    );

    // Start writes and kill them at staggered points, so at least some land in
    // the middle of a save rather than all at the same safe moment.
    for (i, title) in ["Killed one", "Killed two", "Killed three"]
        .iter()
        .enumerate()
    {
        let mut child = spawn_jjj(repo.path(), &["problem", "new", title, "--force"]);
        std::thread::sleep(Duration::from_millis(2 + (i as u64) * 3));
        let _ = child.kill();
        let _ = child.wait();
    }

    // Whatever survived, the store must still be readable and must still
    // contain the entity written before the crashes.
    let listed = run_jjj_success(repo.path(), &["problem", "list"]);
    assert!(
        listed.contains("Established work"),
        "a killed writer destroyed previously-committed data: {listed}"
    );

    // And a subsequent write must succeed — a half-written temp file or a
    // stranded lock would block it.
    run_jjj_success(
        repo.path(),
        &["problem", "new", "After the crash", "--force"],
    );
    let after = run_jjj_success(repo.path(), &["problem", "list"]);
    assert!(after.contains("After the crash"));
}

#[test]
fn no_temp_files_are_left_behind_by_a_completed_write() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Tidy", "--force"]);

    let problems = repo.path().join(".jj/jjj-meta/problems");
    let strays: Vec<String> = fs::read_dir(&problems)
        .expect("problems dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".tmp"))
        .collect();

    assert!(
        strays.is_empty(),
        "atomic_write left temp files behind: {strays:?}"
    );
}

// =============================================================================
// The write lock
// =============================================================================

#[test]
fn a_second_writer_waits_for_the_lock_rather_than_corrupting() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();

    // Two writers racing on the same back-reference field is the classic
    // lost-update window the flock exists to close.
    let children: Vec<_> = (0..6)
        .map(|i| {
            spawn_jjj(
                repo.path(),
                &["problem", "new", &format!("Racer {i}"), "--force"],
            )
        })
        .collect();
    for mut child in children {
        let status = child.wait().expect("wait");
        assert!(status.success(), "a concurrent writer failed outright");
    }

    let listed = run_jjj_success(repo.path(), &["problem", "list"]);
    for i in 0..6 {
        assert!(
            listed.contains(&format!("Racer {i}")),
            "concurrent write lost: Racer {i} is missing from:\n{listed}"
        );
    }
}

#[test]
fn the_lock_is_released_when_its_holder_dies() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Before", "--force"]);

    // Kill a writer and immediately try again. flock is released by the kernel
    // on process death; a pid-file lock would strand here and need manual `rm`.
    let mut child = spawn_jjj(repo.path(), &["problem", "new", "Doomed", "--force"]);
    std::thread::sleep(Duration::from_millis(3));
    let _ = child.kill();
    let _ = child.wait();

    let started = Instant::now();
    let out = run_jjj(repo.path(), &["problem", "new", "After", "--force"]);
    assert!(
        out.status.success(),
        "the next writer was blocked by a dead holder's lock:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the next writer hung waiting on a stranded lock"
    );
}

// =============================================================================
// Cache corruption
// =============================================================================

#[test]
fn a_truncated_cache_heals_instead_of_reporting_an_empty_project() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(
        repo.path(),
        &["problem", "new", "Durable entity", "--force"],
    );
    run_jjj_success(repo.path(), &["db", "rebuild"]);

    let db = repo.path().join(".jj/jjj.db");
    assert!(db.exists(), "precondition: the cache exists");

    // Truncate to a header-sized prefix of garbage: a plausible outcome of a
    // power loss, and the shape that must never be mistaken for "no entities".
    fs::write(&db, b"SQLite format 3\0garbage").expect("corrupt the cache");

    let listed = run_jjj(repo.path(), &["problem", "list"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(
        combined.contains("Durable entity"),
        "a corrupt cache hid data that is still on disk in markdown — the cache \
         is derived and must never be the only source of truth. Got:\n{combined}"
    );
}

#[test]
fn a_deleted_cache_is_not_a_data_loss_event() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Survives", "--force"]);
    run_jjj_success(repo.path(), &["db", "rebuild"]);

    fs::remove_file(repo.path().join(".jj/jjj.db")).expect("remove cache");

    let listed = run_jjj_success(repo.path(), &["problem", "list"]);
    assert!(
        listed.contains("Survives"),
        "deleting the derived cache lost data: {listed}"
    );

    // And it can be rebuilt from the canonical markdown.
    run_jjj_success(repo.path(), &["db", "rebuild"]);
    let after = run_jjj_success(repo.path(), &["problem", "list"]);
    assert!(after.contains("Survives"));
}

#[test]
fn a_corrupt_entity_file_is_skipped_with_a_warning_not_a_crash() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Good entity", "--force"]);

    // One unparseable file must not take the whole listing down with it.
    fs::write(
        repo.path().join(".jj/jjj-meta/problems/broken.md"),
        "---\nthis: is: not: valid: yaml: [\n---\nbody\n",
    )
    .expect("write broken entity");

    let out = run_jjj(repo.path(), &["problem", "list"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("Good entity"),
        "one malformed file suppressed every other entity: {combined}"
    );
}
