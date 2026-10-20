//! Opt-in local command telemetry: which of jjj's leaf operations are actually
//! invoked.
//!
//! jjj has 32 top-level commands and roughly 102 leaf operations. Any argument
//! about which of those should survive is an opinion until someone counts. This
//! module does the counting, and nothing else.
//!
//! # What is recorded
//!
//! One JSON object per invocation, appended to `.jj/jjj-meta/.telemetry.jsonl`:
//!
//! ```text
//! {"ts":"2026-10-19T18:04:11.902Z","command":"problem","subcommand":"list","ms":7}
//! ```
//!
//! Four fields, no more. `command` and `subcommand` come from
//! [`clap::ArgMatches::subcommand`], which returns the **canonical name from the
//! command definition** — a `&'static str` baked into the binary, never a slice
//! of argv. So no title, body, id, path, revset or any other user-supplied value
//! can reach this file even in principle: the code below never reads an argument
//! value. That matters because the file sits inside a repository, and sooner or
//! later somebody will cat it or hand it to someone else.
//!
//! The deliberate consequence: a subcommand name that could only be recovered by
//! reading an argument is not recorded. Less data beats leaked data.
//!
//! An invocation that `clap` rejects — a misspelled subcommand, a missing
//! argument — is **not** recorded, because `clap` prints its error and exits
//! before `main` runs. A command that parses and then fails *is* recorded: an
//! operation an agent keeps reaching for and keeps failing at is the most
//! interesting row in the histogram.
//!
//! # Where it lives, and why it never syncs
//!
//! `.jj/jjj-meta/.telemetry.jsonl` — same class as `.lamport`,
//! `.sync_state.json`, `.events_offsets.json` and `automation.toml`, and
//! dot-prefixed for the same reason. Two independent code paths keep it local:
//!
//! - **Push** copies an explicit allowlist into the `.jj/jjj-sync` workspace
//!   (`sync_meta_to_bookmark` in `src/commands/push.rs`: the five entity dirs,
//!   `config.toml`, `events.jsonl`, `events/*.jsonl`, `rankings/**/*.json`).
//!   Nothing else is ever copied, so nothing else can be committed to the `jjj`
//!   bookmark.
//! - **Fetch** classifies every incoming path with `classify` in
//!   `src/commands/fetch.rs`, an allowlist ending in `_ => Target::Skip`.
//!
//! So the file is unsyncable by construction in both directions, not by a rule
//! someone has to remember.
//!
//! # Off by default
//!
//! Nothing happens unless `JJJ_TELEMETRY=1`. The disabled path is a single
//! `getenv`: no stat, no open, no file created, no directory created.
//!
//! # Reading it
//!
//! A histogram over the leaf operations, most-used first:
//!
//! ```text
//! jq -rR 'fromjson? | .command + (if .subcommand then " " + .subcommand else "" end)' \
//!     .jj/jjj-meta/.telemetry.jsonl | sort | uniq -c | sort -rn
//! ```
//!
//! `-R` plus `fromjson?` rather than plain `-r`, so a line truncated by a
//! crash mid-append is skipped instead of aborting the count.
//!
//! No reader command ships with this: the point of the measurement is to shrink
//! the command surface, so growing it to read the measurement would be an odd
//! way to start.

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Enable flag. Exactly `1`; anything else — including `true` or `yes` — is off,
/// so there is one spelling to grep for and no ambiguity in a swarm's env.
const ENV_VAR: &str = "JJJ_TELEMETRY";

/// Filename under `.jj/jjj-meta/`, dot-prefixed to match the rest of the
/// machine-local state.
const TELEMETRY_FILE: &str = ".telemetry.jsonl";

/// One invocation's record. Exactly the four fields and no room for a fifth.
#[derive(serde::Serialize)]
struct Record<'a> {
    ts: chrono::DateTime<chrono::Utc>,
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    subcommand: Option<&'a str>,
    ms: u64,
}

/// An invocation being timed. Created by [`start`], consumed by [`finish`].
pub struct Probe {
    command: String,
    subcommand: Option<String>,
    meta_dir: PathBuf,
    started: Instant,
}

/// Whether telemetry is enabled for this process.
fn enabled() -> bool {
    std::env::var_os(ENV_VAR).as_deref() == Some(OsStr::new("1"))
}

/// The `.jj/jjj-meta/` directory for the repository containing `from`, if it
/// already exists.
///
/// A directory walk rather than `jj root`, because spawning a subprocess to
/// decide where to write a telemetry line would cost more than the command being
/// measured. The cases a walk misses (a custom store path) simply go unrecorded,
/// which is the right trade for a measurement that is allowed to be incomplete.
///
/// Deliberately does **not** create anything: enabling telemetry outside a jjj
/// repository must not leave a directory behind.
fn meta_dir_from(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    loop {
        let meta = dir.join(".jj").join("jjj-meta");
        if meta.is_dir() {
            return Some(meta);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// The canonical command path of a parsed invocation: the top-level verb, then
/// any deeper levels joined by `/`. `("problem", Some("list"))`,
/// `("github", Some("import"))`, `("prime", None)`.
///
/// Every string here is `clap`'s own name for a subcommand, so the result is
/// drawn from the fixed set of names declared in `cli.rs` rather than from user
/// input. Deeper levels are joined rather than dropped so that the leaf
/// operation stays identifiable if the surface ever nests three deep.
fn names_from_matches(matches: &clap::ArgMatches) -> Option<(String, Option<String>)> {
    let (top, mut cursor) = matches.subcommand()?;
    let mut rest: Vec<&str> = Vec::new();
    while let Some((name, sub)) = cursor.subcommand() {
        rest.push(name);
        cursor = sub;
    }
    let sub = if rest.is_empty() {
        None
    } else {
        Some(rest.join("/"))
    };
    Some((top.to_string(), sub))
}

/// Re-derive the command path from an argv-like sequence.
///
/// `main` holds a parsed `Cli`, not the `ArgMatches` it came from, and the enum
/// carries no name for its own variant. Rather than hand-maintain a 102-arm
/// match that would drift from `cli.rs` on the first new subcommand, the matcher
/// is simply run again — but only when telemetry is enabled, so the disabled
/// path pays nothing for it.
fn names_from_args<I, T>(args: I) -> Option<(String, Option<String>)>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use clap::CommandFactory;
    let matches = crate::cli::Cli::command().try_get_matches_from(args).ok()?;
    names_from_matches(&matches)
}

/// Begin timing this invocation. `None` — and no work at all — unless
/// `JJJ_TELEMETRY=1`.
pub fn start() -> Option<Probe> {
    if !enabled() {
        return None;
    }
    let (command, subcommand) = names_from_args(std::env::args_os())?;
    let meta_dir = meta_dir_from(&std::env::current_dir().ok()?)?;
    Some(Probe {
        command,
        subcommand,
        meta_dir,
        started: Instant::now(),
    })
}

/// Append the record for a finished invocation. A no-op for `None`.
pub fn finish(probe: Option<Probe>) {
    let Some(probe) = probe else {
        return;
    };
    let record = Record {
        ts: chrono::Utc::now(),
        command: &probe.command,
        subcommand: probe.subcommand.as_deref(),
        ms: probe.started.elapsed().as_millis() as u64,
    };
    append(&probe.meta_dir, &record);
}

/// Append one record, silently.
///
/// **The errors here are swallowed on purpose. Do not "fix" this.** Telemetry is
/// a measurement of the tool, not part of it: a read-only filesystem, a full
/// disk or a vanished metadata directory must cost the user at most one missing
/// line in a histogram. Failing — or even warning — would make an opt-in
/// counter able to break the command it was counting, which is a far worse
/// outcome than an incomplete count. This is the one place in jjj where
/// discarding an error is the correct behaviour.
///
/// # Concurrent writers
///
/// A swarm runs many `jjj` processes at once, all appending here. The record is
/// serialized into a single buffer and written with one `write_all` to a file
/// opened `O_APPEND`, so the offset lookup and the write are one atomic step per
/// process and two concurrent records cannot interleave. There is no lock: the
/// same technique the event shards use, minus their per-writer sharding, which
/// is unnecessary because a torn read would cost one malformed line in a counter
/// rather than a lost event. A reader should skip lines it cannot parse.
fn append(meta_dir: &Path, record: &Record<'_>) {
    let Ok(mut line) = serde_json::to_vec(record) else {
        return;
    };
    line.push(b'\n');
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(meta_dir.join(TELEMETRY_FILE))
    {
        let _ = file.write_all(&line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_lines(meta: &Path) -> Vec<String> {
        std::fs::read_to_string(meta.join(TELEMETRY_FILE))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    #[test]
    fn a_leaf_operation_is_command_plus_subcommand() {
        assert_eq!(
            names_from_args(["jjj", "problem", "list"]),
            Some(("problem".to_string(), Some("list".to_string())))
        );
        assert_eq!(
            names_from_args(["jjj", "prime"]),
            Some(("prime".to_string(), None))
        );
    }

    #[test]
    fn argument_values_never_reach_the_record() {
        // The whole privacy claim in one assertion: a command line carrying a
        // title, a body, a path and an id yields only the two static names.
        let (command, subcommand) = names_from_args([
            "jjj",
            "problem",
            "new",
            "SECRET-TITLE",
            "--body",
            "SECRET-BODY",
            "--parent",
            "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a",
        ])
        .unwrap();
        assert_eq!(command, "problem");
        assert_eq!(subcommand.as_deref(), Some("new"));

        let tmp = tempfile::tempdir().unwrap();
        append(
            tmp.path(),
            &Record {
                ts: chrono::Utc::now(),
                command: &command,
                subcommand: subcommand.as_deref(),
                ms: 3,
            },
        );
        let written = read_lines(tmp.path()).join("\n");
        assert!(!written.contains("SECRET"), "{written}");
        assert!(!written.contains("01957d"), "{written}");
    }

    #[test]
    fn a_record_has_exactly_the_four_fields() {
        let tmp = tempfile::tempdir().unwrap();
        append(
            tmp.path(),
            &Record {
                ts: chrono::Utc::now(),
                command: "query",
                subcommand: None,
                ms: 11,
            },
        );
        let lines = read_lines(tmp.path());
        assert_eq!(lines.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        let obj = v.as_object().unwrap();
        // `subcommand` is omitted when absent, so a top-level verb has three.
        assert_eq!(obj.len(), 3, "{obj:?}");
        assert!(obj.contains_key("ts"));
        assert_eq!(obj["command"], "query");
        assert_eq!(obj["ms"], 11);
    }

    #[test]
    fn appends_accumulate_rather_than_truncate() {
        let tmp = tempfile::tempdir().unwrap();
        for ms in 0..5u64 {
            append(
                tmp.path(),
                &Record {
                    ts: chrono::Utc::now(),
                    command: "next",
                    subcommand: None,
                    ms,
                },
            );
        }
        assert_eq!(read_lines(tmp.path()).len(), 5);
    }

    #[test]
    fn a_missing_directory_is_silent_and_creates_nothing() {
        // The failure mode that must never surface: telemetry cannot break the
        // command it is measuring.
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join("not-a-directory");
        append(
            &gone,
            &Record {
                ts: chrono::Utc::now(),
                command: "doctor",
                subcommand: None,
                ms: 1,
            },
        );
        assert!(!gone.exists());
    }

    #[test]
    fn the_metadata_dir_is_found_by_walking_up_and_never_created() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("repo");
        let deep = root.join("src").join("commands");
        std::fs::create_dir_all(&deep).unwrap();
        assert_eq!(meta_dir_from(&deep), None);
        assert!(!root.join(".jj").exists());

        let meta = root.join(".jj").join("jjj-meta");
        std::fs::create_dir_all(&meta).unwrap();
        assert_eq!(meta_dir_from(&deep), Some(meta));
    }

    /// The cost of the work telemetry adds, in isolation.
    ///
    /// Not an assertion — a stopwatch. `#[ignore]`d because a timing threshold
    /// on shared CI is a flaky test, and because the number is only ever read by
    /// a person:
    ///
    /// ```text
    /// cargo test --release --lib telemetry::tests::cost_of_the_enabled_path \
    ///     -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "measurement, not an assertion"]
    fn cost_of_the_enabled_path() {
        const N: u32 = 1000;
        let tmp = tempfile::tempdir().unwrap();

        let t = Instant::now();
        for _ in 0..N {
            std::hint::black_box(names_from_args(["jjj", "problem", "list"]));
        }
        let naming = t.elapsed() / N;

        let t = Instant::now();
        for ms in 0..N {
            append(
                tmp.path(),
                &Record {
                    ts: chrono::Utc::now(),
                    command: "problem",
                    subcommand: Some("list"),
                    ms: ms as u64,
                },
            );
        }
        let appending = t.elapsed() / N;

        let t = Instant::now();
        for _ in 0..N {
            std::hint::black_box(enabled());
        }
        let probing = t.elapsed() / N;

        println!("\n--- telemetry cost per invocation ---");
        println!("re-deriving the command path : {naming:?}");
        println!("appending one record         : {appending:?}");
        println!("total, enabled               : {:?}", naming + appending);
        println!("getenv only, disabled        : {probing:?}\n");
    }

    #[test]
    fn disabled_unless_the_flag_is_exactly_one() {
        // Not a test of `enabled()` itself — the env is process-global and other
        // tests run in parallel — but of the comparison it makes.
        for (value, want) in [("1", true), ("0", false), ("true", false), ("", false)] {
            assert_eq!(OsStr::new(value) == OsStr::new("1"), want);
        }
    }
}
