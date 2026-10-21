//! Acceptance for opt-in command telemetry (plan unit A1).
//!
//! The claim being tested is the one the swarm-core cut list rests on: that a
//! period of ordinary `jjj` use with `JJJ_TELEMETRY=1` yields a file from which a
//! histogram over the ~102 leaf operations can be computed. A week cannot be run
//! here, so a scripted series of varied invocations stands in for it, and the
//! histogram is computed and printed:
//!
//! ```text
//! cargo test --test telemetry_test -- --nocapture
//! ```
//!
//! The negative half matters as much: with the variable unset, the file must not
//! exist at all.

mod test_helpers;
use test_helpers::{run_jjj, run_jjj_env, setup_test_repo};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ON: &[(&str, &str)] = &[("JJJ_TELEMETRY", "1")];

fn telemetry_path(repo: &Path) -> PathBuf {
    repo.join(".jj/jjj-meta/.telemetry.jsonl")
}

/// Every leaf operation in the file, with its invocation count.
///
/// Mirrors the documented one-liner:
/// `jq -r '.command + ...' .telemetry.jsonl | sort | uniq -c | sort -rn`
fn histogram(repo: &Path) -> BTreeMap<String, usize> {
    let text = std::fs::read_to_string(telemetry_path(repo)).unwrap_or_default();
    let mut counts = BTreeMap::new();
    for line in text.lines() {
        // A reader skips what it cannot parse rather than failing; see the
        // concurrency note on `telemetry::append`.
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let leaf = match v.get("subcommand").and_then(|s| s.as_str()) {
            Some(sub) => format!("{} {}", v["command"].as_str().unwrap(), sub),
            None => v["command"].as_str().unwrap().to_string(),
        };
        *counts.entry(leaf).or_insert(0) += 1;
    }
    counts
}

#[test]
fn a_scripted_run_yields_a_histogram_over_leaf_operations() {
    let repo = setup_test_repo();
    let dir = repo.path();

    // A varied session: five entity kinds, reads and writes, a query, an
    // orientation call, diagnostics, and two commands that fail.
    let script: &[&[&str]] = &[
        &["problem", "new", "Telemetry has no evidence"],
        &["problem", "new", "Ranking is unexercised"],
        &["problem", "list"],
        &["problem", "list"],
        &["problem", "show", "Telemetry has no evidence"],
        &["problem", "tree"],
        &[
            "solution",
            "new",
            "Append a record per invocation",
            "--problem",
            "Telemetry has no evidence",
        ],
        &["solution", "list"],
        &[
            "critique",
            "new",
            "Append a record per invocation",
            "Records could leak titles",
        ],
        &["critique", "list"],
        &["finding", "new", "Telemetry has no evidence", "32 commands"],
        &["finding", "list"],
        &["milestone", "new", "Phase A"],
        &["milestone", "list"],
        &["prime"],
        &["next"],
        &["next"],
        &["next"],
        &["status"],
        &["query", "status:open"],
        &["events"],
        &["insights"],
        &["overlaps"],
        &["contention"],
        &["tags"],
        &["whoami"],
        &["doctor"],
        // Failures are invocations too, and must be counted: a command an agent
        // keeps reaching for and keeps failing at is the most interesting row in
        // the histogram.
        &["problem", "show", "no-such-problem-anywhere"],
        &["solution", "approve", "no-such-solution-anywhere"],
    ];

    for args in script {
        run_jjj_env(dir, ON, args);
    }

    let hist = histogram(dir);

    println!("\n--- leaf-operation histogram ({} rows) ---", hist.len());
    let mut rows: Vec<_> = hist.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (leaf, n) in &rows {
        println!("{n:5}  {leaf}");
    }
    let total: usize = hist.values().sum();
    println!("{total:5}  TOTAL\n");

    // Every invocation landed, including the two that exited non-zero.
    assert_eq!(total, script.len(), "histogram: {hist:?}");

    // Counts are per leaf operation, not per top-level command: `problem list`
    // and `problem show` are distinct rows.
    assert_eq!(hist.get("problem list"), Some(&2));
    assert_eq!(hist.get("problem show"), Some(&2));
    assert_eq!(hist.get("next"), Some(&3));
    assert_eq!(hist.get("solution approve"), Some(&1));
    assert_eq!(hist.get("doctor"), Some(&1));

    // Nothing from the command lines survives but the two static names.
    let raw = std::fs::read_to_string(telemetry_path(dir)).unwrap();
    for leaked in [
        "Telemetry has no evidence",
        "Ranking is unexercised",
        "Append a record per invocation",
        "Records could leak titles",
        "32 commands",
        "Phase A",
        "status:open",
        "no-such-problem-anywhere",
    ] {
        assert!(!raw.contains(leaked), "leaked {leaked:?} into {raw}");
    }
    // Nor any absolute path, which would name the machine and the user.
    assert!(!raw.contains('/'), "a path reached the record: {raw}");

    // Each record is exactly `{ts, command, subcommand?, ms}`.
    for line in raw.lines() {
        let obj = serde_json::from_str::<serde_json::Value>(line).unwrap();
        // `serde_json::Value` keys an object by a sorted map, so compare the
        // field *set* rather than the on-disk order.
        let keys: Vec<&str> = obj
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert!(
            keys == ["command", "ms", "subcommand", "ts"] || keys == ["command", "ms", "ts"],
            "unexpected fields {keys:?} in {line}"
        );
        assert!(obj["ms"].is_u64(), "{line}");
    }
}

#[test]
fn nothing_is_written_without_the_flag() {
    let repo = setup_test_repo();
    let dir = repo.path();

    for args in [
        &["problem", "new", "Unrecorded"][..],
        &["problem", "list"][..],
        &["prime"][..],
        &["doctor"][..],
    ] {
        run_jjj(dir, args);
    }

    assert!(
        !telemetry_path(dir).exists(),
        "telemetry file exists with JJJ_TELEMETRY unset"
    );
}

#[test]
fn only_the_value_one_enables_it() {
    let repo = setup_test_repo();
    let dir = repo.path();

    for value in ["0", "true", "yes", ""] {
        run_jjj_env(dir, &[("JJJ_TELEMETRY", value)], &["problem", "list"]);
        assert!(
            !telemetry_path(dir).exists(),
            "JJJ_TELEMETRY={value:?} enabled telemetry"
        );
    }

    run_jjj_env(dir, ON, &["problem", "list"]);
    assert_eq!(histogram(dir).get("problem list"), Some(&1));
}

#[test]
fn concurrent_invocations_each_append_one_intact_line() {
    // The swarm case: many `jjj` processes appending at once. An `O_APPEND`
    // file plus one `write_all` per record must give one well-formed line per
    // invocation, with none lost and none interleaved.
    let repo = setup_test_repo();
    let dir = repo.path().to_path_buf();

    const WRITERS: usize = 16;
    let handles: Vec<_> = (0..WRITERS)
        .map(|_| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                run_jjj_env(&dir, ON, &["problem", "list"]);
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    let raw = std::fs::read_to_string(telemetry_path(&dir)).unwrap();
    assert_eq!(raw.lines().count(), WRITERS, "lost or merged lines:\n{raw}");
    for line in raw.lines() {
        let obj: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("torn line {line:?}: {e}\nfile:\n{raw}"));
        assert_eq!(obj["command"], "problem");
        assert_eq!(obj["subcommand"], "list");
    }
    assert!(raw.ends_with('\n'), "a record lost its newline:\n{raw}");
}

/// End-to-end overhead, measured the way a user would feel it.
///
/// Not an assertion — a stopwatch, `#[ignore]`d so a timing number never fails
/// the gate:
///
/// ```text
/// cargo test --release --test telemetry_test \
///     overhead_per_invocation -- --ignored --nocapture
/// ```
#[test]
#[ignore = "measurement, not an assertion"]
fn overhead_per_invocation() {
    use std::time::{Duration, Instant};

    let repo = setup_test_repo();
    let dir = repo.path();
    run_jjj_env(dir, ON, &["problem", "new", "Something to list"]);

    const N: usize = 60;
    let mut on = Vec::with_capacity(N);
    let mut off = Vec::with_capacity(N);
    // Interleaved, so machine drift lands on both arms equally.
    for _ in 0..N {
        let t = Instant::now();
        run_jjj_env(dir, ON, &["problem", "list"]);
        on.push(t.elapsed());
        let t = Instant::now();
        run_jjj(dir, &["problem", "list"]);
        off.push(t.elapsed());
    }

    let median = |mut v: Vec<Duration>| {
        v.sort();
        v[v.len() / 2]
    };
    let (on, off) = (median(on), median(off));
    println!("\n--- `jjj problem list`, {N} runs each ---");
    println!("JJJ_TELEMETRY=1 median : {on:?}");
    println!("unset median           : {off:?}");
    println!("delta                  : {:?}\n", on.saturating_sub(off));
}

#[test]
fn the_file_is_outside_the_synced_set() {
    // The file is machine-local by construction, not by convention: `push`
    // copies an allowlist into `.jj/jjj-sync` and `fetch` classifies incoming
    // paths with an allowlist ending in `Skip`. This asserts the half that is
    // observable without a remote — a push leaves it behind.
    let repo = setup_test_repo();
    let dir = repo.path();

    run_jjj_env(dir, ON, &["problem", "new", "Pushed"]);
    assert!(telemetry_path(dir).exists());

    // No remote is configured, so the push may fail at the transport step; what
    // matters is what the sync workspace contains once the local bookmark has
    // been built.
    run_jjj_env(dir, ON, &["push", "--no-prompt"]);

    let sync = dir.join(".jj/jjj-sync");
    if sync.exists() {
        assert!(
            !sync.join(".telemetry.jsonl").exists(),
            "telemetry was copied into the sync workspace"
        );
    }
}
