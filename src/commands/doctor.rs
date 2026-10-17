//! Environment and repository diagnostics.
//!
//! jjj's failure modes are mostly environmental rather than logical: a stale
//! push lock, a cache that needs rebuilding, a `jj` version whose CLI moved, a
//! bookmark that never got tracked, an automation rule the user did not know
//! was active. Each is easy to see once you know where to look and invisible
//! otherwise, so this gathers all of them into one command — usable as a first
//! step in support and pasteable into a bug report.
//!
//! It is read-only. Nothing here mutates the repository.

use std::fs;
use std::path::Path;

use crate::context::CommandContext;
use crate::error::Result;
use crate::storage::AUTOMATION_FILE;

/// Severity of a single check.
///
/// `Problem` is the only level that fails the process. A `Warn` must not, or
/// `doctor` becomes unusable in CI for the opposite reason: a fresh clone has no
/// SQLite cache, which is a warning and entirely normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Ok,
    Warn,
    Problem,
}

impl Level {
    fn marker(self) -> &'static str {
        match self {
            Level::Ok => "✓",
            Level::Warn => "!",
            Level::Problem => "✗",
        }
    }
}

struct Check {
    level: Level,
    name: String,
    detail: String,
    /// What to do about it — only shown for warnings and problems, because a
    /// diagnostic that reports a fault without a next step just relocates the
    /// confusion.
    fix: Option<String>,
    /// Why running `fix` is a decision rather than a repair, when it is one.
    ///
    /// Most fixes here are safe to run unattended: `jjj db rebuild` derives the
    /// cache again from markdown that owns the truth, and losing it costs
    /// nothing. Four are not, and the difference is invisible in the text — they
    /// read like the others, one imperative sentence after an arrow. An
    /// unattended agent clearing `doctor` to green will run `jjj resolve
    /// --theirs` and throw away a colleague's edit, because nothing in the
    /// output said that command picks a loser.
    ///
    /// So the hazard is named per check rather than inferred. `None` means the
    /// fix is mechanical; `Some(why)` means a person decides, and `why` says
    /// what is at stake.
    judgement: Option<String>,
}

impl Check {
    fn ok(name: &str, detail: impl Into<String>) -> Self {
        Self {
            level: Level::Ok,
            name: name.into(),
            detail: detail.into(),
            fix: None,
            judgement: None,
        }
    }

    fn warn(name: &str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            name: name.into(),
            detail: detail.into(),
            fix: Some(fix.into()),
            judgement: None,
        }
    }

    fn problem(name: &str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            level: Level::Problem,
            name: name.into(),
            detail: detail.into(),
            fix: Some(fix.into()),
            judgement: None,
        }
    }

    /// Mark this check's fix as one a person decides, saying what is at stake.
    fn needs_judgement(mut self, why: impl Into<String>) -> Self {
        debug_assert!(
            self.fix.is_some(),
            "a check with no fix has no decision to gate"
        );
        self.judgement = Some(why.into());
        self
    }
}

pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let mut checks = vec![
        check_jjj_version(),
        check_jj_version(ctx),
        check_identity(ctx),
        check_meta_path(ctx),
        check_cache(ctx),
    ];
    checks.push(check_cache_drift(ctx));
    checks.extend(check_installed_skill());
    checks.push(check_format_skew(ctx));
    checks.extend(check_locks(ctx));
    checks.push(check_escalations(ctx));
    checks.push(check_conflicts(ctx));
    checks.extend(check_automation(ctx));
    checks.push(check_sync_state(ctx));

    let problems = checks.iter().filter(|c| c.level == Level::Problem).count();

    if json {
        let payload: Vec<_> = checks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "check": c.name,
                    "level": match c.level {
                        Level::Ok => "ok",
                        Level::Warn => "warn",
                        Level::Problem => "problem",
                    },
                    "detail": c.detail,
                    "fix": c.fix,
                    // Null when the fix is mechanical, a reason when it is not,
                    // so the caller's test is the obvious one:
                    // `if fix and not fix_needs_judgement: run it`.
                    "fix_needs_judgement": c.judgement,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("jjj doctor");
        println!("──────────────────────────────────────────────");
        for check in &checks {
            println!(
                "{} {:22} {}",
                check.level.marker(),
                check.name,
                check.detail
            );
            if let Some(fix) = &check.fix {
                println!("    → {}", fix);
                if let Some(why) = &check.judgement {
                    println!("      (your call: {})", why);
                }
            }
        }
        println!();

        let worst = checks.iter().map(|c| c.level).max().unwrap_or(Level::Ok);
        match worst {
            Level::Ok => println!("Everything looks healthy."),
            Level::Warn => println!("Usable, with warnings above."),
            // No summary line here: the error returned below carries it, and
            // printing both said the same thing twice.
            Level::Problem => {}
        }
    }

    // Fail the process when a check found a real fault.
    //
    // This used to return `Ok(())` whatever it found, so `jjj doctor` would
    // print `✗ conflicts  1 entit(ies) carry unresolved merge markers` and exit
    // 0. A diagnostic that reports a fault and then reports success cannot gate
    // anything — CI reads the exit code, not the glyphs — so the one place a
    // repository's health could have been enforced automatically was silently
    // inert. The `Level` doc comment had claimed this behaviour for some time
    // without it being implemented.
    if problems > 0 {
        return Err(crate::error::JjjError::Unhealthy(format!(
            "{problems} problem{} found — see the suggested fixes above",
            if problems == 1 { "" } else { "s" }
        )));
    }
    Ok(())
}

fn check_jjj_version() -> Check {
    Check::ok("jjj", format!("v{}", env!("CARGO_PKG_VERSION")))
}

fn check_jj_version(ctx: &CommandContext) -> Check {
    match ctx.jj().execute(&["--version"]) {
        Ok(out) => Check::ok("jj", out.trim().to_string()),
        Err(e) => Check::problem(
            "jj",
            format!("not usable: {e}"),
            "install jujutsu — jjj cannot resolve change IDs without it",
        ),
    }
}

fn check_identity(ctx: &CommandContext) -> Check {
    let actor = match ctx.store.get_current_user() {
        Ok(actor) => actor,
        Err(e) => {
            return Check::warn(
                "identity",
                format!("could not resolve an actor: {e}"),
                "set JJJ_USER, or configure jj: jj config set --user user.name \"Your Name\"",
            )
        }
    };
    if actor.trim().is_empty() {
        return Check::warn(
            "identity",
            "no actor could be resolved",
            "set JJJ_USER, or configure jj: jj config set --user user.name \"Your Name\"",
        );
    }
    Check::ok("identity", actor)
}

fn check_meta_path(ctx: &CommandContext) -> Check {
    let meta = ctx.store.meta_path();
    if !meta.exists() {
        return Check::problem(
            "metadata",
            format!("{} does not exist", meta.display()),
            "run `jjj init` in this repository",
        );
    }
    let counts = [
        "problems",
        "solutions",
        "critiques",
        "milestones",
        "findings",
    ]
    .iter()
    .map(|dir| count_md(&meta.join(dir)))
    .collect::<Vec<_>>();
    Check::ok(
        "metadata",
        format!(
            "{} — {} problems, {} solutions, {} critiques, {} milestones, {} findings",
            meta.display(),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            counts[4]
        ),
    )
}

fn count_md(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0)
}

fn check_cache(ctx: &CommandContext) -> Check {
    let db = ctx.jj().repo_root().join(".jj").join("jjj.db");
    if !db.exists() {
        return Check::warn(
            "cache",
            "no SQLite cache — reads fall back to walking the filesystem",
            "run `jjj db rebuild` (search and fast listings need it)",
        );
    }
    let size = fs::metadata(&db).map(|m| m.len()).unwrap_or(0);
    match crate::db::Database::open(&db) {
        Ok(_) => Check::ok(
            "cache",
            format!(
                "present, {} KB (schema v{})",
                size / 1024,
                crate::db::schema::SCHEMA_VERSION
            ),
        ),
        Err(e) => Check::warn(
            "cache",
            format!("cannot open: {e}"),
            "run `jjj db rebuild` — the cache is derived, so nothing is lost",
        ),
    }
}

/// The skill an agent actually loads, versus the one this binary ships.
///
/// `skills/jjj/SKILL.md` is installed by *copying* it into `.claude/skills/` or
/// `~/.claude/skills/`. Nothing then keeps the copy in step, so upgrading the
/// binary silently leaves agents reading an older workflow — and a stale skill
/// is worse than none, because it produces confident, wrong commands. The repo's
/// own drift test only guards the file in this repository; it cannot see the
/// copy that is actually being loaded.
///
/// This was not hypothetical. On the machine this check was written, the global
/// copy was 5,249 bytes against a shipped 37,480 — missing `jjj prime`, findings
/// and most of the workflow.
///
/// The shipped text is embedded rather than hashed at build time so that an
/// installed binary with no source tree can still answer the question, and
/// compared by content rather than a version marker because the file declares
/// no version. A warning, not a problem: a stale copy on one developer's
/// machine is a local setup fault, and failing CI over it would be wrong.
///
/// beads solves the same problem by writing a marker block with a content hash
/// into AGENTS.md and offering `--check`. jjj does not write into anyone's
/// instruction files, so it only needs the detection half.
fn check_installed_skill() -> Vec<Check> {
    const SHIPPED: &str = include_str!("../../skills/jjj/SKILL.md");

    let mut candidates: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from(".claude/skills/jjj/SKILL.md"),
        std::path::PathBuf::from(".codex/skills/jjj/SKILL.md"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(std::path::Path::new(&home).join(".claude/skills/jjj/SKILL.md"));
    }

    let mut checks = Vec::new();
    for path in candidates {
        let Ok(installed) = fs::read_to_string(&path) else {
            continue;
        };
        let name = "skill copy";
        if installed == SHIPPED {
            checks.push(Check::ok(name, format!("{} is current", path.display())));
        } else {
            checks.push(
                Check::warn(
                    name,
                    format!(
                        "{} differs from the skill this binary ships ({} bytes vs {})",
                        path.display(),
                        installed.len(),
                        SHIPPED.len()
                    ),
                    "re-copy skills/jjj/ over it — agents are reading the stale one",
                )
                .needs_judgement("overwrites that file, including edits made on purpose"),
            );
        }
    }
    checks
}

/// Does the derived cache still agree with the markdown that owns the truth?
///
/// `check_cache` only proves the database opens. That says nothing about whether
/// it still describes this repository: markdown edited behind jjj's back — by a
/// merge tool, a script, a hand-resolved conflict — leaves the cache openable
/// and wrong, and every fast listing and search then answers from it. The
/// failure is silent in the worst way, because entities simply do not appear.
///
/// Counts, not contents. A per-entity comparison would mean reading every file,
/// which is the cost `doctor` exists to avoid; counts catch the cases that
/// actually happen (entities added or removed outside jjj) for one cheap query
/// each. beads' equivalent check is narrower still — one flag, one count — and
/// is the thing in `bd doctor` worth copying.
///
/// Reported as a problem rather than a warning: a cache that disagrees with the
/// markdown gives *wrong* answers, which is worse than the missing-cache case
/// that only makes reads slow.
fn check_cache_drift(ctx: &CommandContext) -> Check {
    const KINDS: [&str; 5] = [
        "problems",
        "solutions",
        "critiques",
        "milestones",
        "findings",
    ];

    let db_path = ctx.jj().repo_root().join(".jj").join("jjj.db");
    if !db_path.exists() {
        // Absence is `check_cache`'s business, and it is not drift.
        return Check::ok("cache drift", "no cache to compare");
    }
    let db = match crate::db::Database::open(&db_path) {
        Ok(db) => db,
        Err(_) => return Check::ok("cache drift", "cache unreadable — see the cache check"),
    };

    let meta = ctx.store.meta_path();
    let mut drifted: Vec<String> = Vec::new();
    for kind in KINDS {
        let on_disk = count_md(&meta.join(kind));
        let cached: i64 =
            match db
                .conn()
                .query_row(&format!("SELECT COUNT(*) FROM {kind}"), [], |r| r.get(0))
            {
                Ok(n) => n,
                Err(e) => {
                    return Check::warn(
                        "cache drift",
                        format!("could not count {kind}: {e}"),
                        "run `jjj db rebuild`",
                    )
                }
            };
        if cached != on_disk as i64 {
            drifted.push(format!("{kind}: {cached} cached vs {on_disk} on disk"));
        }
    }

    if drifted.is_empty() {
        Check::ok("cache drift", "cache agrees with the markdown")
    } else {
        Check::problem(
            "cache drift",
            drifted.join("; "),
            "run `jjj db rebuild` — the markdown is authoritative, so nothing is lost",
        )
    }
}

/// Report frontmatter keys this binary does not recognise.
///
/// beads detects "migration content skew" — the case where this clone and its
/// remote applied different content for the same schema version — and treats it
/// as a read-only diagnostic that "never gates push/pull". jjj cannot have that
/// specific fault, since there is no schema to migrate and the markdown is the
/// authority. The underlying question still applies, though, and nothing was
/// answering it: *am I an old binary looking at records a newer one wrote?*
///
/// Unknown keys are the signal, and jjj already keeps them — `extra` preserves
/// anything the structs do not name so an older client cannot delete a newer
/// one's fields. That makes this purely informational, which is why it warns
/// rather than failing: nothing is broken, and the honest advice is "upgrade if
/// you want to see these fields", not "repair something".
///
/// It names the keys rather than counting them, because "4 unknown keys" tells
/// you nothing and `claim_epoch` tells you what shipped since your build.
fn check_format_skew(ctx: &CommandContext) -> Check {
    // Straight off the markdown, not the cache: see
    // `MetadataStore::unknown_frontmatter_keys` for why that distinction is the
    // whole check.
    let keys = ctx.store.unknown_frontmatter_keys();

    if keys.is_empty() {
        return Check::ok("format skew", "no unrecognised fields");
    }

    let shown: Vec<&str> = keys.iter().take(6).map(String::as_str).collect();
    let more = keys.len().saturating_sub(shown.len());
    let list = if more > 0 {
        format!("{} (+{more} more)", shown.join(", "))
    } else {
        shown.join(", ")
    };
    Check::warn(
        "format skew",
        format!("records carry fields this build does not know: {list}"),
        "a newer jjj wrote them; they are preserved on save, so upgrade only if you need to read them",
    )
}

fn check_locks(ctx: &CommandContext) -> Vec<Check> {
    let meta = ctx.store.meta_path();
    let mut checks = Vec::new();

    // The push lock is an O_EXCL pid-file, so a crashed push strands it and the
    // user has to remove it by hand — exactly the situation worth naming here.
    let push_lock = meta.join(".push.lock");
    if push_lock.exists() {
        let holder = fs::read_to_string(&push_lock).unwrap_or_default();
        let holder = holder.trim().to_string();
        checks.push(
            Check::warn(
                "push lock",
                format!(
                    "held by pid {}",
                    if holder.is_empty() {
                        "unknown"
                    } else {
                        &holder
                    }
                ),
                format!(
                    "if no jjj push is running, remove it: rm {}",
                    push_lock.display()
                ),
            )
            .needs_judgement(
                "removing a live lock lets two pushes race; check for a running push first",
            ),
        );
    } else {
        checks.push(Check::ok("push lock", "free"));
    }

    // The write lock is flock-based, so the kernel releases it on death and its
    // mere presence is not a fault — say so, rather than alarming the reader.
    let write_lock = meta.join(".write.lock");
    if write_lock.exists() {
        checks.push(Check::ok(
            "write lock",
            "present (flock — released automatically when its holder exits)",
        ));
    }

    checks
}

/// Is anyone blocked on a person?
///
/// `doctor` is where someone looks when a repository is behaving oddly, and an
/// open escalation is the one condition nothing in the system can clear by
/// itself. Reported as a problem rather than a warning: a fleet that has said
/// it needs a human is not degraded, it is stopped.
fn check_escalations(ctx: &CommandContext) -> Check {
    let events = match ctx.store.list_events_cached() {
        Ok(e) => e,
        Err(e) => {
            return Check::warn(
                "escalations",
                format!("could not read the event log: {e}"),
                "retry after `jjj db rebuild`",
            )
        }
    };
    let open = crate::escalation::open_escalations(&events);
    match open.len() {
        0 => Check::ok("escalations", "none open"),
        n => {
            let now = chrono::Utc::now();
            // The oldest one, because that is the one that has been waiting.
            let oldest = open
                .first()
                .map(|e| {
                    let age = e.age(now);
                    if age.num_hours() >= 1 {
                        format!("{}h", age.num_hours())
                    } else {
                        format!("{}m", age.num_minutes().max(0))
                    }
                })
                .unwrap_or_default();
            // A warning, not a problem, even though a person really is
            // waiting. An escalation is the system working: someone correctly
            // said a human is needed. Grading it a failure makes `doctor` red
            // for a healthy repo, and the remedy line right here tells an agent
            // exactly how to turn it green again — by clearing the request. The
            // worst outcome of this check is an agent discarding a human's
            // pending ask to pass a diagnostic, so it reports without pressing.
            Check::warn(
                "escalations",
                format!(
                    "{n} open — oldest {oldest} ago: {}",
                    crate::utils::truncate(&open[0].reason, 60)
                ),
                "see `jjj escalate`; a person clears it with `jjj escalate --clear <id>`",
            )
        }
    }
}

fn check_conflicts(ctx: &CommandContext) -> Check {
    match crate::commands::conflicts::scan(&ctx.store) {
        Ok(found) if found.is_empty() => Check::ok("conflicts", "none"),
        Ok(found) => Check::problem(
            "conflicts",
            format!("{} entit(ies) carry unresolved merge markers", found.len()),
            "run `jjj conflicts` to list them, then `jjj resolve <id> --ours|--theirs`",
        )
        .needs_judgement("--ours and --theirs each discard the other side's edit"),
        Err(e) => Check::warn(
            "conflicts",
            format!("could not check: {e}"),
            "retry after `jjj db rebuild`",
        ),
    }
}

fn check_automation(ctx: &CommandContext) -> Vec<Check> {
    let mut checks = Vec::new();

    let active = ctx
        .store
        .load_config()
        .map(|c| c.automation)
        .unwrap_or_default();
    let shell_rules = active
        .iter()
        .filter(|r| r.enabled && matches!(r.action, crate::models::AutomationAction::Shell))
        .count();

    if active.is_empty() {
        checks.push(Check::ok("automation", "no rules active"));
    } else {
        // Shell rules run arbitrary commands with the user's privileges, so
        // "how many, and where from" is worth stating plainly rather than
        // leaving someone to discover it when one fires.
        checks.push(Check::ok(
            "automation",
            format!(
                "{} rule(s) active from {} ({} shell)",
                active.len(),
                AUTOMATION_FILE,
                shell_rules
            ),
        ));
    }

    // Rules sitting in the synced config are inert but worth surfacing: either
    // the user expects them to run, or a remote put them there.
    if let Ok(legacy) = ctx.store.legacy_config_automation() {
        if !legacy.is_empty() {
            checks.push(Check::warn(
                "automation (ignored)",
                format!(
                    "{} rule(s) in config.toml are ignored — that file syncs from the remote",
                    legacy.len()
                ),
                "inspect with `jjj automation list`; adopt with `jjj automation migrate --force`",
            )
            .needs_judgement(
                "these rules arrived over sync and may run shell commands you did not write",
            ));
        }
    }

    checks
}

fn check_sync_state(ctx: &CommandContext) -> Check {
    let state = ctx.store.meta_path().join(".sync_state.json");
    if !state.exists() {
        return Check::ok("sync", "no sync state yet (nothing pushed from this clone)");
    }
    match fs::read_to_string(&state) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(value) => {
                let rev = value
                    .get("last_synced_rev")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                Check::ok("sync", format!("last synced rev {rev}"))
            }
            Err(e) => Check::warn(
                "sync",
                format!("state file is unreadable: {e}"),
                "the next fetch falls back to a full reconcile; safe to delete the file",
            ),
        },
        Err(e) => Check::warn(
            "sync",
            format!("state file unreadable: {e}"),
            "the next fetch falls back to a full reconcile",
        ),
    }
}
