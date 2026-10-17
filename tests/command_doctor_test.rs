//! `jjj doctor` must fail the process when it finds a real fault.
//!
//! It used to return success whatever it found: a repository with unresolved
//! merge markers printed `✗ conflicts  1 entit(ies) carry unresolved merge
//! markers`, then `Problems found — see the suggested fixes above`, and exited
//! **0**. CI reads the exit code rather than the glyphs, so the one check that
//! could have gated a repository's health automatically was inert, and the
//! `Level` doc comment had claimed the opposite behaviour for some time.
//!
//! The boundary matters as much as the failure. Only `Problem` fails; a `Warn`
//! must not, because a fresh clone has no SQLite cache — a warning, and
//! completely normal. A doctor that failed on warnings would be turned off.

mod test_helpers;

use test_helpers::{jj_available, run_jjj, run_jjj_env, run_jjj_success, setup_test_repo};

/// Append conflict markers to an entity, the way an unresolved three-way merge
/// leaves them. `check_conflicts` is the clearest `Problem`-level check: markers
/// on disk are never acceptable and never self-clearing.
fn plant_conflict_markers(dir: &std::path::Path) {
    let problems = dir.join(".jj/jjj-meta/problems");
    let file = std::fs::read_dir(&problems)
        .expect("problems dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .expect("a problem file");
    let mut text = std::fs::read_to_string(&file).expect("read problem");
    text.push_str("\n<<<<<<< local\nmine\n=======\ntheirs\n>>>>>>> remote\n");
    std::fs::write(&file, text).expect("write problem");
}

#[test]
fn a_healthy_repo_exits_zero() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj(&dir, &["doctor"]);
    assert!(
        out.status.success(),
        "a healthy repo must exit 0:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_warning_alone_does_not_fail_the_process() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // Removing the SQLite cache is the cheapest genuine warning: reads fall
    // back to walking the filesystem, which is slower but entirely correct —
    // the cache is derived and `db rebuild` restores it. A doctor that failed
    // on this would fail on every fresh clone, and would be switched off.
    let db = dir.path().join(".jj/jjj.db");
    if db.exists() {
        std::fs::remove_file(&db).expect("remove cache");
    }

    let out = run_jjj(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no SQLite cache"),
        "expected the cache warning: {stdout}"
    );
    assert!(
        out.status.success(),
        "a warning must not fail the process:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn unresolved_conflict_markers_fail_the_process() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Has a conflict"]);
    plant_conflict_markers(dir.path());

    let out = run_jjj(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "doctor found a conflict and still reported success:\n{stdout}"
    );
    assert!(
        stdout.contains("conflicts"),
        "the fault itself must still be named: {stdout}"
    );
    assert!(
        stderr.contains("problem found"),
        "and the failure must say how many: {stderr}"
    );
    // The report ran correctly and found something wrong — it is not itself
    // invalid, and must not be described as such.
    assert!(
        !stderr.contains("Validation failed"),
        "a diagnostic result should not be framed as a validation error: {stderr}"
    );
}

#[test]
fn json_output_still_parses_when_the_check_fails() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Has a conflict"]);
    plant_conflict_markers(dir.path());

    // A non-zero exit must not cost a scripted caller its output: the report
    // goes to stdout and the failure to stderr, so `--json | jq` still works
    // on exactly the run that matters.
    let out = run_jjj(&dir, &["doctor", "--json"]);
    assert!(!out.status.success(), "should still fail with --json");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let checks: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not JSON ({e}): {stdout}"));
    let problems: Vec<&str> = checks
        .as_array()
        .expect("an array of checks")
        .iter()
        .filter(|c| c["level"].as_str() == Some("problem"))
        .filter_map(|c| c["check"].as_str())
        .collect();
    assert_eq!(
        problems,
        vec!["conflicts"],
        "the JSON must name the failing check: {stdout}"
    );
}

/// An open escalation is reported, and does not fail `doctor`.
///
/// It was a `problem` first, on the argument that "a fleet that has said it
/// needs a human is not degraded, it is stopped". Once severity decided the
/// exit code that became the wrong trade: an escalation is the system working
/// correctly, so a healthy repo went red, and the remedy line printed beside it
/// tells an agent exactly how to turn it green — by clearing the request. The
/// worst thing this check could cause is an agent discarding a human's pending
/// ask to make a diagnostic pass, so it warns instead of failing. Pinned in
/// both directions, because either reading is defensible and the next person to
/// touch it should have to change a test that says why.
#[test]
fn an_open_escalation_is_reported_without_failing_doctor() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["escalate", "The deploy key is missing"]);

    let out = run_jjj(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "a pending human request is not a broken repo:\n{stdout}"
    );
    assert!(
        stdout.contains("The deploy key is missing"),
        "but the reason must still be named, or escalating is pointless: {stdout}"
    );
    // Specifically a warning: silently OK would be the other failure.
    assert!(
        stdout.contains('!') || stdout.to_lowercase().contains("warn"),
        "it must read as a warning, not as a clean check: {stdout}"
    );

    // Clearing it removes the notice entirely.
    let json = run_jjj_success(&dir, &["escalate", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let id = v[0]["id"].as_str().expect("id").to_string();
    run_jjj_success(&dir, &["escalate", "--clear", &id[..8]]);

    let after = run_jjj(&dir, &["doctor"]);
    let after_out = String::from_utf8_lossy(&after.stdout);
    assert!(
        after.status.success(),
        "doctor must stay green: {after_out}"
    );
    assert!(
        !after_out.contains("The deploy key is missing"),
        "a cleared escalation must stop being reported: {after_out}"
    );
}

/// The cache is derived, so it can disagree with the markdown that owns the
/// truth — and `check_cache` only ever proved the database *opens*. A cache that
/// has silently lost an entity makes every fast listing and search answer wrong,
/// which is worse than the missing-cache case that merely makes reads slow.
#[test]
fn a_cache_that_disagrees_with_the_markdown_is_a_problem() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "One"]);
    run_jjj_success(&dir, &["problem", "new", "Two"]);

    let clean = run_jjj(&dir, &["doctor"]);
    assert!(
        clean.status.success(),
        "a consistent cache must pass:\n{}",
        String::from_utf8_lossy(&clean.stdout)
    );

    // Write an entity behind jjj's back, the way a merge tool or a hand-resolved
    // conflict does. The cache stays openable and stops describing the repo.
    let problems = dir.path().join(".jj/jjj-meta/problems");
    let existing = std::fs::read_dir(&problems)
        .expect("problems dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .expect("a problem file");
    let smuggled = "01a14599-dead-7000-8000-000000000001";
    let text = std::fs::read_to_string(&existing)
        .expect("read")
        .lines()
        .map(|l| {
            if l.starts_with("id: ") {
                format!("id: {smuggled}")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(problems.join(format!("{smuggled}.md")), text).expect("write");

    let out = run_jjj(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "a drifted cache reported success:\n{stdout}"
    );
    assert!(
        stdout.contains("cache drift") && stdout.contains("on disk"),
        "the drift must be named with both counts: {stdout}"
    );

    // And `db rebuild` must actually clear it, or the check is just noise.
    run_jjj_success(&dir, &["db", "rebuild"]);
    let healed = run_jjj(&dir, &["doctor"]);
    assert!(
        healed.status.success(),
        "db rebuild did not clear the drift:\n{}",
        String::from_utf8_lossy(&healed.stdout)
    );
}

/// A skill is installed by *copying* it, and nothing keeps the copy in step, so
/// upgrading the binary silently leaves agents reading an older workflow. The
/// repo's own drift test guards the file in this repository; it cannot see the
/// copy actually being loaded. On the machine this was written, the global copy
/// was 5,249 bytes against a shipped 37,480.
#[test]
fn a_stale_installed_skill_copy_is_reported() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();
    // Isolate HOME so the result does not depend on this machine's own copy.
    let home = p.join("fake-home");
    std::fs::create_dir_all(home.join(".claude/skills/jjj")).expect("home");
    let env: [(&str, &str); 1] = [("HOME", home.to_str().unwrap())];

    let shipped = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/jjj/SKILL.md"),
    )
    .expect("shipped skill");

    // A current copy is reported as current.
    std::fs::write(home.join(".claude/skills/jjj/SKILL.md"), &shipped).expect("write current");
    let out = run_jjj_env(p, &env, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("skill copy") && text.contains("is current"),
        "a matching copy should be reported current: {text}"
    );
    assert!(out.status.success(), "a current copy must not fail doctor");

    // A stale one is named, with both sizes so the gap is obvious.
    std::fs::write(
        home.join(".claude/skills/jjj/SKILL.md"),
        "# an old copy\nnothing else\n",
    )
    .expect("write stale");
    let out = run_jjj_env(p, &env, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("differs from the skill this binary ships"),
        "a stale copy was not reported: {text}"
    );
    assert!(
        text.contains("re-copy"),
        "and it must say what to do: {text}"
    );
    // Only a warning: a stale copy on one machine is a local setup fault, and
    // failing CI over it would be wrong.
    assert!(
        out.status.success(),
        "a stale skill copy must not fail the process:\n{text}"
    );

    // No copy installed at all is not a fault — most repos have none.
    std::fs::remove_file(home.join(".claude/skills/jjj/SKILL.md")).expect("remove");
    let out = run_jjj_env(p, &env, &["doctor"]);
    assert!(out.status.success());
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("skill copy"),
        "doctor should stay quiet when nothing is installed"
    );
}

/// A field a newer jjj wrote is reported, and does not fail the process.
///
/// This is the one question beads' migration-skew check asks that jjj can also
/// ask: am I an old binary reading records a newer one wrote? jjj's answer is
/// already correct — `extra` preserves unknown keys verbatim, so nothing is lost
/// — but until now nothing *said* so, and an agent seeing a field it cannot
/// query had no way to learn why.
#[test]
fn fields_from_a_newer_build_are_reported_without_failing() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Written by a later jjj"]);

    // The shape a newer release would leave behind: a key the structs do not
    // name, in otherwise valid frontmatter.
    let problems = dir.path().join(".jj/jjj-meta/problems");
    let file = std::fs::read_dir(&problems)
        .expect("problems dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .expect("a problem file");
    let text = std::fs::read_to_string(&file).expect("read problem");
    let (head, rest) = text
        .split_once("status:")
        .expect("frontmatter with a status");
    std::fs::write(&file, format!("{head}claim_epoch: 7\nstatus:{rest}")).expect("write problem");

    let out = run_jjj(&dir, &["doctor"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("claim_epoch"),
        "the unknown field was not named: {combined}"
    );
    assert!(
        out.status.success(),
        "an unknown field is preserved, not broken — it must not fail doctor: {combined}"
    );
}

/// And the clean case says so, rather than staying silent.
#[test]
fn a_repo_with_no_unknown_fields_reports_none() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Ordinary"]);
    let out = run_jjj_success(&dir, &["doctor"]);
    assert!(
        out.contains("no unrecognised fields"),
        "doctor should state the clean verdict: {out}"
    );
}

/// `measured_at` must survive the SQLite cache, because reads are DB-primary.
///
/// A column missing from the `findings` table is invisible to every caller even
/// when the markdown has it — the same trap that made `check_format_skew`
/// unable to fire when it read through the cache. Adding the field without the
/// column would have made this kind's `CACHE_FAITHFUL = true` a lie.
///
/// It also pins the column *order*: `row_to_finding` decodes by position, so
/// the DDL, the `COLUMNS` constant and the decoder indexes are one fact written
/// three times. Getting it wrong shifted every later column by one and parsed
/// the evidence body as `updated_at`, which is how this test came to exist.
#[test]
fn a_measurement_time_survives_the_cache() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj_success(&dir, &["problem", "new", "decode is slow", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let pid = v["id"].as_str().expect("id").to_string();

    run_jjj_success(
        &dir,
        &[
            "finding",
            "new",
            &pid,
            "120k-op floor",
            "--body",
            "measured on the bench rig",
            "--measured-at",
            "2026-10-10T09:30:00Z",
        ],
    );

    // Force the DB-primary path to be the one that answers.
    run_jjj_success(&dir, &["db", "rebuild"]);

    let listed = run_jjj_success(&dir, &["finding", "list", "--problem", &pid, "--json"]);
    let arr: serde_json::Value = serde_json::from_str(&listed).expect("valid JSON");
    let f = if arr.is_array() { &arr[0] } else { &arr };

    assert_eq!(
        f["measured_at"].as_str(),
        Some("2026-10-10T09:30:00Z"),
        "the measurement time must come back through the cache: {listed}"
    );
    // The neighbouring columns prove the order is right, not just the value.
    assert_eq!(
        f["evidence"].as_str(),
        Some("measured on the bench rig"),
        "evidence must not be read out of the wrong column: {listed}"
    );
    assert!(
        f["updated_at"].as_str().unwrap_or("").starts_with("20"),
        "updated_at must still be a timestamp: {listed}"
    );
}

/// A time nobody can parse is refused before anything is written, rather than
/// recording a finding without the time it belongs to.
#[test]
fn an_unparseable_measurement_time_is_refused() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj_success(&dir, &["problem", "new", "a problem", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let pid = v["id"].as_str().expect("id").to_string();

    let bad = run_jjj(
        &dir,
        &[
            "finding",
            "new",
            &pid,
            "a claim",
            "--body",
            "x",
            "--measured-at",
            "last tuesday",
        ],
    );
    assert!(!bad.status.success(), "a vague date must not be accepted");
    assert_eq!(
        bad.status.code(),
        Some(1),
        "the caller must change the command, so this is not a retryable precondition"
    );

    // And nothing was recorded.
    let listed = run_jjj_success(&dir, &["finding", "list", "--problem", &pid, "--json"]);
    assert!(
        listed.trim() == "[]" || !listed.contains("a claim"),
        "the refused finding must not exist: {listed}"
    );
}

/// `--json` must not return less than the text output.
///
/// `show_problem` returned early on `--json` with just the frontmatter, while
/// the text path went on to print solutions, findings, what the problem waits
/// on, and its children. The machine-readable path was the impoverished one, so
/// an agent paid three extra calls to rebuild what `show` already had in hand.
#[test]
fn problem_show_json_carries_what_the_text_output_shows() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let out = run_jjj_success(&dir, &["problem", "new", "parent problem", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let pid = v["id"].as_str().expect("id").to_string();

    run_jjj_success(&dir, &["solution", "new", "an approach", "--problem", &pid]);
    run_jjj_success(
        &dir,
        &["finding", "new", &pid, "a measurement", "--body", "42"],
    );
    run_jjj_success(&dir, &["problem", "new", "child problem", "--parent", &pid]);

    let json = run_jjj_success(&dir, &["problem", "show", &pid, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    for key in ["solutions", "findings", "subproblems", "waiting_on"] {
        assert!(
            v.get(key).map(|x| x.is_array()).unwrap_or(false),
            "`{key}` must be present as an array: {json}"
        );
    }
    assert_eq!(v["solutions"][0]["title"].as_str(), Some("an approach"));
    assert_eq!(v["findings"][0]["title"].as_str(), Some("a measurement"));
    assert_eq!(v["subproblems"][0]["title"].as_str(), Some("child problem"));
    // The open child also holds the parent back, which the text path says too.
    assert_eq!(v["waiting_on"][0]["title"].as_str(), Some("child problem"));

    // The frontmatter keys stay exactly where callers already expect them.
    assert_eq!(v["id"].as_str(), Some(pid.as_str()));
    assert_eq!(v["title"].as_str(), Some("parent problem"));
}

/// A fix that discards someone's work is marked; a mechanical one is not.
///
/// Every remedy line in `doctor` reads the same: one imperative sentence after
/// an arrow. Most are mechanical — `jjj db rebuild` derives the cache again
/// from the markdown that owns the truth, so running it unattended costs
/// nothing. Four are not, and `jjj resolve <id> --ours|--theirs` is the worst
/// of them: whichever flag you pass throws away the other side's edit. An agent
/// working a checklist to green has no way to tell those apart from the text,
/// and the obvious loop — read `fix`, run `fix` — silently picks a loser.
///
/// So the hazard is data, not prose: `fix_needs_judgement` is null when the fix
/// is mechanical and a reason when a person decides. The mechanical half is
/// asserted too, because a flag set on every check would be a flag that says
/// nothing, and that failure mode is invisible from the hazardous side alone.
#[test]
fn a_fix_that_destroys_work_says_so_and_a_safe_one_does_not() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    run_jjj_success(&dir, &["problem", "new", "Has a conflict"]);
    plant_conflict_markers(dir.path());

    // Removing the derived cache puts a known-safe fix in the same report, so
    // one run shows both sides of the distinction.
    let _ = std::fs::remove_file(dir.path().join(".jj/jjj.db"));

    let out = run_jjj(&dir, &["doctor", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let checks: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not JSON ({e}): {stdout}"));
    let checks = checks.as_array().expect("an array of checks");

    let find = |name: &str| {
        checks
            .iter()
            .find(|c| c["check"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("no `{name}` check in: {stdout}"))
    };

    let conflicts = find("conflicts");
    assert!(
        conflicts["fix"]
            .as_str()
            .unwrap_or_default()
            .contains("--ours"),
        "expected the destructive remedy on the conflicts check: {conflicts}"
    );
    let why = conflicts["fix_needs_judgement"]
        .as_str()
        .unwrap_or_else(|| {
            panic!("the conflicts fix discards an edit and must be gated: {conflicts}")
        });
    assert!(
        why.contains("discard"),
        "the gate must say what is at stake, not just that it exists: {why:?}"
    );

    let cache = find("cache");
    assert!(
        cache["fix"]
            .as_str()
            .unwrap_or_default()
            .contains("db rebuild"),
        "expected the cache rebuild remedy: {cache}"
    );
    assert!(
        cache["fix_needs_judgement"].is_null(),
        "rebuilding a derived cache is mechanical; gating it would train callers \
         to ignore the gate: {cache}"
    );

    // The human-readable path has to carry it too — an agent reading the
    // default output is the case that went wrong.
    let text = run_jjj(&dir, &["doctor"]);
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(
        text.contains("(your call:"),
        "the text report must mark the decision as well: {text}"
    );
}
