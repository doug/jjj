use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::error::{JjjError, Result};
use crate::jj::JjClient;
use crate::models::CritiqueStatus;
use crate::storage::MetadataStore;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A simple cross-process advisory lock backed by an O_EXCL pid-file.
///
/// On creation we attempt to atomically create `<path>` (failing if it
/// exists). On drop we remove the file. This is racy compared to flock but
/// has no MSRV implications and works on macOS+Linux.
///
/// If the file exists at construction time we assume another process is
/// active and refuse to proceed. Stale locks (process died mid-sync) require
/// the user to `rm` the file manually; the message we emit tells them how.
struct PidLock {
    path: PathBuf,
}

impl PidLock {
    fn acquire(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                let _ = writeln!(f, "{}", std::process::id());
                let _ = f.sync_data();
                Ok(Self { path })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let holder = std::fs::read_to_string(&path).unwrap_or_default();
                let holder = holder.trim();
                Err(JjjError::Validation(format!(
                    "Another jjj push is in progress (lock held by pid {}). \
                     If you're sure no other jjj process is running, remove:\n  {}",
                    if holder.is_empty() { "unknown" } else { holder },
                    path.display(),
                )))
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Filter for files that the sync workspace owns.
///
/// Only `.md` files are deleted during the prep step; anything else in the
/// destination directory (editor backups, manual edits) is left alone.
fn is_jjj_owned_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("md")
}

/// Create or update the jjj bookmark from the current metadata files.
///
/// Creates an on-demand workspace if needed, copies all metadata files into it,
/// commits, and updates the bookmark. This is the only place that creates jj
/// commits for metadata — all local operations use plain files.
///
/// Holds a PidLock for the duration so two concurrent pushes can't race on
/// the workspace.
fn sync_meta_to_bookmark(jj_client: &JjClient, store: &MetadataStore) -> Result<()> {
    use crate::storage::META_BOOKMARK;
    use std::fs;

    let sync_config = store.load_config().unwrap_or_default().sync;
    let meta_path = store.meta_path();
    let sync_path = jj_client.repo_root().join(".jj").join("jjj-sync");

    let _lock = PidLock::acquire(meta_path.join(".push.lock"))?;

    // Create sync workspace on demand using configured or default command
    if !sync_path.join(".jj").exists() {
        let sync_str = sync_path
            .to_str()
            .ok_or_else(|| crate::error::JjjError::PathError(sync_path.clone()))?;

        let revision = if jj_client.bookmark_exists(META_BOOKMARK)? {
            META_BOOKMARK
        } else {
            "root()"
        };

        let ws_prefix = Some(sync_config.workspace_prefix());
        jj_client.execute_workspace(ws_prefix, "add", &[sync_str, "-r", revision])?;
    }

    let sync_client = JjClient::with_root(sync_path.clone())?;
    let ws_prefix = Some(sync_config.workspace_prefix());
    let _ = sync_client.execute_workspace(ws_prefix, "update-stale", &[]);

    // Copy all metadata files into the sync workspace
    for dir in &["problems", "solutions", "critiques", "milestones"] {
        let src_dir = meta_path.join(dir);
        let dst_dir = sync_path.join(dir);
        fs::create_dir_all(&dst_dir)?;

        // Clean destination first to handle deletions, but only touch files
        // we own (`.md`). Editor backups, swap files, and anything else the
        // user might have left in the sync workspace is preserved.
        if dst_dir.exists() {
            for entry in (fs::read_dir(&dst_dir)?).flatten() {
                let path = entry.path();
                if is_jjj_owned_file(&path) {
                    let _ = fs::remove_file(&path);
                }
            }
        }

        // Copy source files
        if src_dir.exists() {
            for entry in (fs::read_dir(&src_dir)?).flatten() {
                let dst = dst_dir.join(entry.file_name());
                fs::copy(entry.path(), dst)?;
            }
        }
    }

    // Copy config.toml and events.jsonl
    let config_src = meta_path.join("config.toml");
    if config_src.exists() {
        fs::copy(&config_src, sync_path.join("config.toml"))?;
    }
    let events_src = meta_path.join("events.jsonl");
    if events_src.exists() {
        fs::copy(&events_src, sync_path.join("events.jsonl"))?;
    }

    // Commit and update bookmark
    sync_client.describe("jjj: sync metadata")?;
    sync_client.execute(&["new"])?;

    let commit_id = sync_client
        .execute(&["log", "--no-graph", "-r", "@-", "-T", "commit_id"])?
        .trim()
        .to_string();

    if jj_client.bookmark_exists(META_BOOKMARK)? {
        jj_client.execute(&[
            "--ignore-working-copy",
            "bookmark",
            "set",
            META_BOOKMARK,
            "-r",
            &commit_id,
            "--allow-backwards",
        ])?;
    } else {
        jj_client.execute(&[
            "--ignore-working-copy",
            "bookmark",
            "create",
            META_BOOKMARK,
            "-r",
            &commit_id,
        ])?;
    }

    Ok(())
}

fn prompt_yes_no(message: &str) -> bool {
    print!("{} [Y/n] ", message);
    if io::stdout().flush().is_err() {
        return false;
    }

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}

pub fn execute(
    ctx: &CommandContext,
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();
    let sync_config = store.load_config().unwrap_or_default().sync;
    let has_git = jj_client.has_git_backend();

    let push_cmd = match sync_config.resolve_push(has_git) {
        Some(cmd) => cmd,
        None => {
            println!("No sync backend configured and no git backend detected.");
            println!("Configure [sync] push in config.toml for custom sync commands.");
            return Ok(());
        }
    };

    // Sync SQLite to markdown and validate before pushing
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");
    if db_path.exists() {
        let db = Database::open(&db_path)?;

        println!("Syncing database to files...");
        db::dump_to_markdown(&db, store)?;

        println!("Validating metadata...");
        let errors = db::validate(&db)?;
        if !errors.is_empty() {
            println!("Validation errors:");
            for error in &errors {
                println!("  \u{2717} {}", error);
            }
            return Err(crate::error::JjjError::Validation(
                "Push aborted. Fix errors and retry.".to_string(),
            ));
        }
        println!("  \u{2713} All checks passed");

        // Flush any pending events
        store.commit_changes()?;
    }

    // Create/update the jjj bookmark from the metadata files.
    // This creates an on-demand workspace, copies files in, and commits.
    sync_meta_to_bookmark(jj_client, store)?;

    if dry_run {
        println!("Would push to {}:", remote);
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj");
        return Ok(());
    }

    // 1. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        let vars = [("bookmark", bookmark.as_str()), ("remote", remote)];
        if jj_client.execute_sync_command(&push_cmd, &vars).is_err() {
            // Retry with --allow-new for new bookmarks
            let retry = format!("{} --allow-new", push_cmd);
            jj_client.execute_sync_command(&retry, &vars)?;
        }
    }

    // 2. Always push jjj bookmark
    println!("Pushing jjj...");
    let vars = [("bookmark", "jjj"), ("remote", remote)];
    if jj_client.execute_sync_command(&push_cmd, &vars).is_err() {
        let retry = format!("{} --allow-new", push_cmd);
        jj_client.execute_sync_command(&retry, &vars)?;
    }

    println!("Pushed to {}.", remote);

    // Clear dirty flag after successful push
    if db_path.exists() {
        let db = Database::open(&db_path)?;
        db::set_dirty(&db, false)?;
    }

    // 3. Smart prompts (unless --no-prompt)
    if !no_prompt {
        check_and_prompt_approve_solve(ctx)?;
    }

    Ok(())
}

fn check_and_prompt_approve_solve(ctx: &CommandContext) -> Result<()> {
    let store = &ctx.store;

    // Find user's active solutions
    let solutions = store.list_solutions()?;
    let user = store.jj_client.user_name().unwrap_or_default();

    for solution in solutions
        .iter()
        .filter(|s| s.is_submitted() && s.assignee.as_deref() == Some(&user))
    {
        // Check if all critiques are resolved
        let critiques = store.list_critiques_for_solution(&solution.id)?;
        let open_critiques: Vec<_> = critiques
            .iter()
            .filter(|c| c.status == CritiqueStatus::Open)
            .collect();

        if open_critiques.is_empty() && !critiques.is_empty() {
            // All critiques resolved - prompt to approve
            if prompt_yes_no(&format!(
                "All critiques on {} \"{}\" resolved. Approve solution?",
                solution.id, solution.title
            )) {
                crate::domain::approve_solution(&ctx.store, &solution.id, false, None)?;
                println!("  Solution {} approved.", solution.id);
            }
        }
    }

    Ok(())
}
