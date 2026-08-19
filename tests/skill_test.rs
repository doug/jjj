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
