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

/// Severity of a single check, which also decides the process exit code.
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
}

impl Check {
    fn ok(name: &str, detail: impl Into<String>) -> Self {
        Self {
            level: Level::Ok,
            name: name.into(),
            detail: detail.into(),
            fix: None,
        }
    }

    fn warn(name: &str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            name: name.into(),
            detail: detail.into(),
            fix: Some(fix.into()),
        }
    }

    fn problem(name: &str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            level: Level::Problem,
            name: name.into(),
            detail: detail.into(),
            fix: Some(fix.into()),
        }
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
    checks.extend(check_locks(ctx));
    checks.push(check_escalations(ctx));
    checks.push(check_conflicts(ctx));
    checks.extend(check_automation(ctx));
    checks.push(check_sync_state(ctx));

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
            }
        }
        println!();

        let worst = checks.iter().map(|c| c.level).max().unwrap_or(Level::Ok);
        match worst {
            Level::Ok => println!("Everything looks healthy."),
            Level::Warn => println!("Usable, with warnings above."),
            Level::Problem => println!("Problems found — see the suggested fixes above."),
        }
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

fn check_locks(ctx: &CommandContext) -> Vec<Check> {
    let meta = ctx.store.meta_path();
    let mut checks = Vec::new();

    // The push lock is an O_EXCL pid-file, so a crashed push strands it and the
    // user has to remove it by hand — exactly the situation worth naming here.
    let push_lock = meta.join(".push.lock");
    if push_lock.exists() {
        let holder = fs::read_to_string(&push_lock).unwrap_or_default();
        let holder = holder.trim().to_string();
        checks.push(Check::warn(
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
        ));
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
            Check::problem(
                "escalations",
                format!(
                    "{n} open — oldest {oldest} ago: {}",
                    crate::utils::truncate(&open[0].reason, 60)
                ),
                "see `jjj escalate`; clear with `jjj escalate --clear <id>`",
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
        ),
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
