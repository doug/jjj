//! Security regressions for the automation subsystem.
//!
//! Two distinct attacks, two distinct fixes, one test file so they stay
//! together:
//!
//! 1. **Value injection** (audit P0.2). A `{{title}}` template must not let an
//!    entity title like `$(touch X)` execute. Values travel through the process
//!    environment, never through the command text.
//! 2. **Rule injection** (F1). `config.toml` syncs through the shared `jjj`
//!    bookmark and fetch applies the remote copy wholesale. An `[[automation]]`
//!    block arriving that way must be inert — otherwise anyone who can push the
//!    bookmark has code execution on every clone.
//!
//! Each test asserts on a *marker file the payload would create*. A payload that
//! runs leaves evidence; asserting on stdout would pass even if the command ran.

mod test_helpers;

use std::fs;
use std::path::Path;
use test_helpers::{jj_available, run_jjj, run_jjj_success, setup_test_repo};

/// Write `automation.toml` (the trusted, machine-local file) with one shell rule.
fn write_local_rule(repo: &Path, event: &str, command: &str) {
    let path = repo.join(".jj/jjj-meta/automation.toml");
    fs::write(
        &path,
        format!(
            "[[automation]]\non = \"{}\"\naction = \"shell\"\ncommand = \"{}\"\nenabled = true\n",
            event,
            command.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    )
    .expect("write automation.toml");
}

/// Append an `[[automation]]` block to the synced `config.toml` (the untrusted
/// file) — simulating either a legacy config or one rewritten by a remote.
fn write_config_rule(repo: &Path, event: &str, command: &str) {
    let path = repo.join(".jj/jjj-meta/config.toml");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    // `automation = []` and `[[automation]]` cannot coexist in one document.
    let existing = existing.replace("automation = []\n", "");
    fs::write(
        &path,
        format!(
            "{}\n[[automation]]\non = \"{}\"\naction = \"shell\"\ncommand = \"{}\"\nenabled = true\n",
            existing,
            event,
            command.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    )
    .expect("write config.toml");
}

// =============================================================================
// 1. Value injection — the command template is trusted, its inputs are not
// =============================================================================

#[test]
fn untrusted_title_cannot_execute_through_a_template() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let marker = repo.path().join("value_injection_marker");

    // The documented template style: no hand-added quotes around the placeholder.
    write_local_rule(
        repo.path(),
        "problem_created",
        &format!("echo {{{{title}}}} > {}/echoed", repo.path().display()),
    );

    // A title that is a command substitution, a quote break, and a chained
    // command all at once.
    let hostile = format!(
        "$(touch {}); '; touch {}; '",
        marker.display(),
        marker.display()
    );
    run_jjj_success(repo.path(), &["problem", "new", &hostile]);

    assert!(
        !marker.exists(),
        "an entity title executed as a shell command — value injection is back"
    );

    // The rule itself must still have run, with the title delivered verbatim.
    let echoed = fs::read_to_string(repo.path().join("echoed")).expect("rule should have run");
    assert!(
        echoed.contains("$(touch"),
        "the title should reach the command as literal text, got: {echoed}"
    );
}

// =============================================================================
// 2. Rule injection — config.toml is remote-controlled and must be inert
// =============================================================================

#[test]
fn automation_rules_in_config_toml_do_not_execute() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let marker = repo.path().join("config_rule_marker");

    write_config_rule(
        repo.path(),
        "problem_created",
        &format!("touch {}", marker.display()),
    );

    let out = run_jjj(repo.path(), &["problem", "new", "routine work"]);
    assert!(out.status.success());

    assert!(
        !marker.exists(),
        "a rule in the synced config.toml executed — this is the F1 RCE path"
    );

    // The user must be told, not silently protected: a rule they think is live
    // being quietly dropped is its own failure mode.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("automation") && combined.contains("config.toml"),
        "ignoring config.toml rules should be reported, got: {combined}"
    );
}

#[test]
fn automation_toml_rules_do_execute() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let marker = repo.path().join("local_rule_marker");

    write_local_rule(
        repo.path(),
        "problem_created",
        &format!("touch {}", marker.display()),
    );

    run_jjj_success(repo.path(), &["problem", "new", "routine work"]);

    assert!(
        marker.exists(),
        "a rule in the machine-local automation.toml should run — the fix must \
         not have disabled automation outright"
    );
}

#[test]
fn push_strips_automation_from_the_shared_config() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    write_config_rule(
        repo.path(),
        "problem_created",
        "touch /tmp/should_never_sync",
    );

    // Exercise the exact function push uses to produce the shared copy, so the
    // assertion holds without needing a configured remote.
    let config = fs::read_to_string(repo.path().join(".jj/jjj-meta/config.toml")).unwrap();
    let (sanitized, stripped) = jjj::commands::push::sanitize_shared_config(&config);

    assert!(stripped, "precondition: the legacy config carries rules");
    assert!(
        !sanitized.contains("[[automation]]"),
        "the pushed copy must not carry rules"
    );
    assert!(
        sanitized.contains("[github]"),
        "stripping automation must preserve every other key"
    );
}

#[test]
fn sanitizing_a_config_without_rules_is_a_no_op() {
    let clean = "name = \"demo\"\n\n[github]\nauto_push = false\n";
    let (out, stripped) = jjj::commands::push::sanitize_shared_config(clean);
    assert!(!stripped);
    assert_eq!(out, clean, "an untouched config must round-trip byte-exact");

    // An unparseable config is copied verbatim rather than silently rewritten.
    let broken = "this is not [ valid toml";
    let (out, stripped) = jjj::commands::push::sanitize_shared_config(broken);
    assert!(!stripped);
    assert_eq!(out, broken);
}

#[test]
fn migrate_moves_rules_to_the_local_file_only_with_force() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    write_config_rule(repo.path(), "problem_created", "echo migrated");

    // Without --force the rules are shown for review and nothing moves.
    let preview = run_jjj_success(repo.path(), &["automation", "migrate"]);
    assert!(preview.contains("--force"), "should ask for confirmation");
    assert!(
        !repo.path().join(".jj/jjj-meta/automation.toml").exists(),
        "a preview must not activate anything"
    );

    let applied = run_jjj_success(repo.path(), &["automation", "migrate", "--force"]);
    assert!(applied.contains("automation.toml"));

    let local = fs::read_to_string(repo.path().join(".jj/jjj-meta/automation.toml")).unwrap();
    assert!(local.contains("echo migrated"));

    // …and the synced file no longer carries them.
    let config = fs::read_to_string(repo.path().join(".jj/jjj-meta/config.toml")).unwrap();
    assert!(
        !config.contains("[[automation]]"),
        "migrate must remove the key so it stops syncing"
    );
}

#[test]
fn automation_list_reports_both_active_and_ignored_rules() {
    if !jj_available() {
        return;
    }
    let repo = setup_test_repo();
    write_local_rule(repo.path(), "problem_created", "echo active");
    write_config_rule(repo.path(), "solution_created", "echo ignored");

    let out = run_jjj_success(repo.path(), &["automation", "list"]);
    assert!(out.contains("echo active"), "active rule should be listed");
    assert!(
        out.contains("echo ignored"),
        "ignored rule should be listed"
    );
    assert!(
        out.contains("Ignored"),
        "the ignored rules need a clear heading, got: {out}"
    );
}
