//! Keeps `skills/jjj/SKILL.md` honest.
//!
//! A skill file is documentation an agent *acts on*, so a stale one is worse
//! than none: it produces confident, wrong commands. The skill this repository
//! ships replaced a hand-maintained one that had drifted badly — it taught
//! `jjj solution test/accept/refute`, `jjj milestone link` and `jjj submit`,
//! none of which exist.
//!
//! These tests extract every command the skill mentions and check it against the
//! real CLI, so the file cannot describe a jjj that isn't there.

mod test_helpers;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn skill_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("skills")
        .join("jjj")
        .join("SKILL.md")
}

fn skill_text() -> String {
    std::fs::read_to_string(skill_path()).expect("skills/jjj/SKILL.md must exist")
}

/// Ask the binary for the subcommands of `path` (empty = top level).
fn subcommands_of(path: &[&str]) -> BTreeSet<String> {
    let mut args: Vec<&str> = path.to_vec();
    args.push("--help");
    let out = Command::new(test_helpers::jjj_binary())
        .args(&args)
        .output()
        .expect("run jjj --help");
    let help = String::from_utf8_lossy(&out.stdout);

    let mut names = BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if in_commands {
            if line.trim().is_empty() {
                break;
            }
            if let Some(word) = line.split_whitespace().next() {
                // Continuation lines of a wrapped description are indented past
                // the command column; real entries start at two spaces.
                if line.starts_with("  ") && !line.starts_with("      ") {
                    names.insert(word.to_string());
                }
            }
        }
    }
    names
}

/// Ask the binary for the long flags accepted at `path`.
fn flags_of(path: &[&str]) -> BTreeSet<String> {
    let mut args: Vec<&str> = path.to_vec();
    args.push("--help");
    let out = Command::new(test_helpers::jjj_binary())
        .args(&args)
        .output()
        .expect("run jjj --help");
    let help = String::from_utf8_lossy(&out.stdout);

    let mut flags = BTreeSet::new();
    for line in help.lines() {
        for token in line.split_whitespace() {
            if !token.starts_with("--") || token.len() < 4 {
                continue;
            }
            // `--flag <VALUE>,` / `--flag=X` / `--flag,` all name the same flag.
            let name: String = token
                .trim_start_matches('-')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if !name.is_empty() {
                flags.insert(format!("--{name}"));
            }
        }
    }
    flags
}

/// Does this command actually accept this flag?
///
/// `flags_of` scrapes `--help`, and clap does not print aliases there: `problem
/// dissolve` takes `--rationale` as an alias for `--reason`, which help never
/// mentions. Scraping alone therefore reports a flag the CLI does accept, and
/// "fixing" the skill to match would have replaced correct documentation with
/// wrong documentation. Acceptance is what the skill's correctness rests on, so
/// ask clap directly: it answers from the parser, aliases included, and its
/// "unexpected argument" is the error an agent would actually hit.
fn accepts_flag(path: &[&str], flag: &str) -> bool {
    let mut args: Vec<&str> = path.to_vec();
    args.push(flag);
    let out = Command::new(test_helpers::jjj_binary())
        .args(&args)
        .output()
        .expect("run jjj to probe a flag");
    let err = String::from_utf8_lossy(&out.stderr);
    // A probe whose *command* did not parse produces no "unexpected argument"
    // either, so absence of that error would read as acceptance and pass this
    // test for every flag on a path that does not exist. Demand that the
    // command itself was understood before believing the answer. (Writing this
    // check by hand in the shell, I hit exactly that: zsh does not word-split
    // an unquoted variable, so `jjj $cmd --flag` ran `jjj "problem dissolve"`
    // and reported every flag accepted.)
    assert!(
        !err.contains("unrecognized subcommand"),
        "probe path `jjj {}` does not exist, so its answer means nothing",
        path.join(" ")
    );
    // Parsing stops at the first error, so a missing positional or a missing
    // value for the flag both mean the flag itself was recognised.
    !err.contains("unexpected argument")
}

/// Every `jjj ...` invocation the skill shows, with the long flags it passes.
///
/// Scanned the same way as [`commands_mentioned`] and with the same limits: a
/// flag written in prose is not something an agent runs.
fn flags_mentioned(text: &str) -> BTreeSet<(String, Option<String>, String)> {
    let mut found = BTreeSet::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            continue;
        }
        // A flag belongs to the invocation it follows, not to the line. The
        // command reference is laid out in two columns —
        //
        //     jjj problem solve ID                jjj problem dissolve ID --reason "..."
        //
        // — so taking every flag on the line gave `jjj problem solve --reason`,
        // `jjj problem reopen --of` and two more: four failures that were all
        // this parser's fault rather than the skill's. Segment at each `jjj`.
        let words: Vec<&str> = line.split_whitespace().collect();
        let starts: Vec<usize> = words
            .iter()
            .enumerate()
            .filter(|(_, w)| **w == "jjj")
            .map(|(i, _)| i)
            .collect();
        for (n, &start) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(words.len());
            let segment = &words[start + 1..end];

            let positional: Vec<&&str> = segment.iter().filter(|w| !w.starts_with('-')).collect();
            let Some(cmd) = positional.first() else {
                continue;
            };
            if !cmd.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                continue;
            }
            let sub = positional
                .get(1)
                .filter(|s| s.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
                .map(|s| s.to_string());

            for w in segment {
                if !w.starts_with("--") || w.len() < 4 {
                    continue;
                }
                let name: String = w
                    .trim_start_matches('-')
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                    .collect();
                if name.is_empty() {
                    continue;
                }
                found.insert((cmd.to_string(), sub.clone(), format!("--{name}")));
            }
        }
    }
    found
}

/// Every `jjj ...` invocation the skill shows, as (command, subcommand) pairs.
///
/// Only fenced code blocks are scanned. Prose mentions jjj in sentences
/// ("jjj is a project tracker"), and an agent does not run sentences.
fn commands_mentioned(text: &str) -> BTreeSet<(String, Option<String>)> {
    let mut found = BTreeSet::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            continue;
        }
        for (i, token) in line.split_whitespace().enumerate() {
            if token != "jjj" {
                continue;
            }
            let rest: Vec<&str> = line.split_whitespace().skip(i + 1).collect();
            let mut it = rest.iter().filter(|w| !w.starts_with('-'));
            let Some(cmd) = it.next() else { continue };
            // Words that are obviously not commands: placeholders and prose.
            if !cmd.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                continue;
            }
            let sub = it
                .next()
                .filter(|s| s.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
                .map(|s| s.to_string());
            found.insert((cmd.to_string(), sub));
        }
    }
    found
}

#[test]
fn the_skill_file_exists_and_declares_itself() {
    let text = skill_text();
    assert!(
        text.starts_with("---\nname: jjj\n"),
        "a skill needs frontmatter with its name so a harness can load it"
    );
    assert!(
        text.contains("description:"),
        "the description is what decides whether an agent loads the skill"
    );
}

#[test]
fn every_command_the_skill_teaches_exists() {
    let text = skill_text();
    let top_level = subcommands_of(&[]);
    assert!(
        top_level.contains("problem"),
        "sanity: could not parse `jjj --help`"
    );

    let mut unknown: Vec<String> = Vec::new();
    for (cmd, sub) in commands_mentioned(&text) {
        if !top_level.contains(&cmd) {
            unknown.push(format!("jjj {cmd}"));
            continue;
        }
        if let Some(sub) = sub {
            let subs = subcommands_of(&[&cmd]);
            // A command with no subcommands takes positional args instead; the
            // next word is a value, not a subcommand.
            if !subs.is_empty() && !subs.contains(&sub) {
                unknown.push(format!("jjj {cmd} {sub}"));
            }
        }
    }

    assert!(
        unknown.is_empty(),
        "skills/jjj/SKILL.md teaches commands that do not exist: {unknown:#?}\n\
         Update the skill — an agent will run these verbatim."
    );
}

#[test]
fn the_skill_states_the_identity_rule_first() {
    let text = skill_text();

    // Identity is the one thing an agent must do before writing anything; if it
    // drifts down the page or out of the file, every downstream claim about
    // per-agent queues becomes false.
    let identity = text
        .find("JJJ_USER")
        .expect("the skill must tell agents to set JJJ_USER");
    let patterns = text
        .find("## Multi-agent patterns")
        .expect("the multi-agent section is the point of this skill");
    assert!(
        identity < patterns,
        "the identity rule must come before the patterns that depend on it"
    );
}

#[test]
fn the_skill_warns_that_claim_is_not_a_lock() {
    let text = skill_text().to_lowercase();

    // This is the failure mode most likely to bite a fleet, and it is not
    // discoverable from `--help`. Verified in tests/identity_test.rs.
    assert!(
        text.contains("advisory") || text.contains("not a lock") || text.contains("not a mutex"),
        "the skill must say `--claim` does not exclude other agents"
    );
}

#[test]
fn the_skill_does_not_teach_the_removed_vocabulary() {
    let text = skill_text();

    // The previous hand-maintained skill taught all of these long after they
    // were renamed. Each would fail at the point an agent ran it.
    for stale in [
        "solution test",
        "solution accept",
        "solution refute",
        "milestone link",
        "jjj submit",
        ".jjj/",
    ] {
        assert!(
            !text.contains(stale),
            "the skill uses removed vocabulary `{stale}` — this is exactly the \
             drift that made the previous skill dangerous"
        );
    }
}

/// A flag the skill teaches must exist too.
///
/// The command-level test above cannot see this: `jjj problem edit` existing
/// says nothing about whether `--clear-body` does, and a renamed or removed flag
/// is exactly as wrong as a renamed command — an agent builds the invocation
/// from this file and gets a clap error it cannot interpret.
///
/// This gap was found while arguing about whether the skill should keep its
/// command reference at all. The argument for keeping it is that drift is
/// guarded here; that argument was only two-thirds true.
#[test]
fn every_flag_the_skill_teaches_exists() {
    let text = skill_text();
    let top_level = subcommands_of(&[]);
    assert!(
        top_level.contains("problem"),
        "sanity: could not parse `jjj --help`"
    );

    // Global flags are accepted everywhere but listed only at the top level.
    let global = flags_of(&[]);

    // Each `--help` is a subprocess, and the skill names a few dozen flags over
    // far fewer commands, so resolve each help page once.
    let mut subs_cache: std::collections::BTreeMap<String, BTreeSet<String>> = Default::default();
    let mut flag_cache: std::collections::BTreeMap<Vec<String>, BTreeSet<String>> =
        Default::default();

    let mut unknown: Vec<String> = Vec::new();
    for (cmd, sub, flag) in flags_mentioned(&text) {
        if !top_level.contains(&cmd) {
            continue; // the command-level test owns this failure
        }
        // Resolve the deepest path that actually exists, so a positional value
        // mistaken for a subcommand does not send us to a nonexistent help page.
        let subs = subs_cache
            .entry(cmd.clone())
            .or_insert_with(|| subcommands_of(&[&cmd]));
        let path: Vec<String> = match &sub {
            Some(s) if subs.contains(s) => vec![cmd.clone(), s.clone()],
            _ => vec![cmd.clone()],
        };
        let accepted = flag_cache.entry(path.clone()).or_insert_with(|| {
            let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();
            flags_of(&borrowed)
        });
        if accepted.contains(&flag) || global.contains(&flag) {
            continue;
        }
        // Not in help: it may still be an alias. Only reached on a would-be
        // failure, so the extra subprocess costs nothing in the green case.
        let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();
        if accepts_flag(&borrowed, &flag) {
            continue;
        }
        unknown.push(format!("jjj {} {flag}", path.join(" ")));
    }

    unknown.sort();
    unknown.dedup();
    assert!(
        unknown.is_empty(),
        "the skill teaches flags the CLI does not accept:\n  {}",
        unknown.join("\n  ")
    );
}
