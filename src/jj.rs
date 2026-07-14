use crate::error::{JjjError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Thin wrapper around `jj` subprocess calls.
///
/// Discovers the `jj` executable on `PATH` and the repository root via
/// `jj root`. All operations invoke `jj` as a child process and parse
/// stdout/stderr.
#[derive(Debug, Clone)]
pub struct JjClient {
    /// Path to the jj executable
    jj_path: PathBuf,

    /// Repository root directory
    repo_root: PathBuf,
}

/// Minimum supported jj version (major, minor).
pub const MIN_JJ_VERSION: (u32, u32) = (0, 25);

/// Context size passed to `jj diff --git` for the delta-fetch content primitive.
///
/// jj caps the emitted context at each file's length, so any value larger than
/// the longest metadata file forces whole-file hunks (both sides reconstructable
/// by [`crate::storage::delta`]). A million lines comfortably exceeds any real
/// entity file while costing nothing — jj never pads beyond the file.
const FULL_CONTEXT_LINES: &str = "1000000";

use crate::storage::sync_state::BOOKMARK_PREFIX;

impl JjClient {
    /// Create a new JjClient, discovering the jj executable and repo root.
    ///
    /// Enforces a minimum jj version: older versions fail early with a clear
    /// message rather than producing cryptic errors deeper in command flow.
    /// Version parse failures are tolerated (e.g., custom builds).
    pub fn new() -> Result<Self> {
        let jj_path = find_executable("jj").ok_or(JjjError::JjNotFound)?;

        if let Some((major, minor)) = jj_version(&jj_path) {
            if (major, minor) < MIN_JJ_VERSION {
                return Err(JjjError::Validation(format!(
                    "jj version {}.{} is too old; jjj requires {}.{} or later. \
                     Upgrade jj: https://github.com/jj-vcs/jj",
                    major, minor, MIN_JJ_VERSION.0, MIN_JJ_VERSION.1
                )));
            }
        }

        let repo_root = Self::find_repo_root(&jj_path)?;

        Ok(Self { jj_path, repo_root })
    }

    /// Create a `JjClient` rooted at an arbitrary directory instead of CWD.
    ///
    /// Used by [`MetadataStore`](crate::storage::MetadataStore) to construct a
    /// client for the metadata workspace (`.jj/jjj-meta/`) that runs `jj`
    /// commands there without affecting the user's main working copy.
    pub fn with_root(root: PathBuf) -> Result<Self> {
        let jj_path = find_executable("jj").ok_or(JjjError::JjNotFound)?;
        Ok(Self {
            jj_path,
            repo_root: root,
        })
    }

    /// Find the repository root using `jj root`.
    ///
    /// This delegates to jj's own repo discovery, which handles colocated repos,
    /// custom store paths, and symlinked `.jj` directories that a manual
    /// directory walk would miss.
    fn find_repo_root(jj_path: &Path) -> Result<PathBuf> {
        let output = Command::new(jj_path)
            .arg("root")
            .output()
            .map_err(|e| JjjError::JjIo {
                args: "root".to_string(),
                source: e,
            })?;
        if !output.status.success() {
            return Err(JjjError::NotInRepository);
        }
        Ok(PathBuf::from(
            String::from_utf8_lossy(&output.stdout).trim(),
        ))
    }

    /// Get the repository root
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Check whether this repository is backed by a git backend.
    ///
    /// Returns `false` for native jj backends or when the store type cannot
    /// be determined. Used to gate `jj git push/fetch` operations.
    pub fn has_git_backend(&self) -> bool {
        let type_file = self.repo_root.join(".jj/repo/store/type");
        std::fs::read_to_string(type_file)
            .map(|s| s.trim() == "git")
            .unwrap_or(false)
    }

    /// Execute a jj command and return the output
    pub fn execute(&self, args: &[&str]) -> Result<String> {
        if std::env::var("JJJ_DEBUG").is_ok() {
            eprintln!("DEBUG: jj {}", args.join(" "));
        }
        let output = Command::new(&self.jj_path)
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| crate::error::JjjError::JjIo {
                args: args.join(" "),
                source: e,
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(crate::error::JjjError::JjCommandFailed {
                args: args.join(" "),
                stderr,
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Get the current change ID
    pub fn current_change_id(&self) -> Result<String> {
        let output = self.execute(&["log", "--no-graph", "-r", "@", "-T", "change_id"])?;
        Ok(output.trim().to_string())
    }

    /// Check if a bookmark exists.
    ///
    /// Uses a templated `jj bookmark list` so the parser doesn't depend on
    /// jj's display formatting (which has changed across releases).
    pub fn bookmark_exists(&self, bookmark: &str) -> Result<bool> {
        let output = self.execute(&["bookmark", "list", "-T", r#"name ++ "\n""#])?;
        Ok(output.lines().any(|line| line.trim() == bookmark))
    }

    /// Create a new bookmark
    pub fn create_bookmark(&self, name: &str, revision: &str) -> Result<()> {
        self.execute(&["bookmark", "create", name, "-r", revision])?;
        Ok(())
    }

    /// Track every metadata bookmark (bare `jjj` and per-pod `jjj/*`) from
    /// `remote`, so their remote counterparts are local-tracking.
    ///
    /// A freshly fetched bookmark is *non-tracking* by default, and jj refuses
    /// to `git push -b jjj/{pod}` when the local ref doesn't track the remote
    /// one that already exists (it would "unexpectedly" move it). Tracking the
    /// whole `jjj*` glob makes this pod's own push fast-forward and is a
    /// harmless no-op for bookmarks already tracked or absent. Best-effort: a
    /// failure here is not fatal to the fetch.
    pub fn track_meta_bookmarks(&self, remote: &str) -> Result<()> {
        let glob = format!("glob:{}*", BOOKMARK_PREFIX);
        self.execute(&["bookmark", "track", &glob, "--remote", remote])?;
        Ok(())
    }

    /// Checkout a specific revision
    pub fn checkout(&self, revision: &str) -> Result<()> {
        self.execute(&["new", revision])?;
        Ok(())
    }

    /// Create a new empty change and set description
    pub fn new_empty_change(&self, message: &str) -> Result<String> {
        self.execute(&["new"])?;
        self.describe(message)?;
        self.current_change_id()
    }

    /// Create a new empty change whose parent is root(), producing an orphan branch.
    pub fn new_orphan_change(&self, message: &str) -> Result<String> {
        self.execute(&["new", "-r", "root()"])?;
        self.describe(message)?;
        self.current_change_id()
    }

    /// Set the description of the current change
    pub fn describe(&self, message: &str) -> Result<()> {
        self.execute(&["describe", "-m", message])?;
        Ok(())
    }

    /// Get the description of a change
    pub fn change_description(&self, change_id: &str) -> Result<String> {
        let output = self.execute(&["log", "--no-graph", "-r", change_id, "-T", "description"])?;
        Ok(output.trim().to_string())
    }

    /// Return the commit description strings for every commit matched by `revset`.
    ///
    /// Descriptions are NUL-delimited in the raw `jj log` output so that
    /// multi-line descriptions are returned intact as single entries.
    pub fn log_descriptions(&self, revset: &str) -> Result<Vec<String>> {
        // NUL byte as record separator — safe because commit messages never
        // contain NUL bytes.
        let output = self.execute(&[
            "log",
            "--no-graph",
            "-r",
            revset,
            "-T",
            r#"description ++ "\x00""#,
        ])?;
        Ok(output
            .split('\x00')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Get the author of a change
    pub fn change_author(&self, change_id: &str) -> Result<String> {
        let output = self.execute(&["log", "--no-graph", "-r", change_id, "-T", "author"])?;
        Ok(output.trim().to_string())
    }

    /// Show the diff for a change
    pub fn show_diff(&self, change_id: &str) -> Result<String> {
        self.execute(&["diff", "-r", change_id])
    }

    /// Get changed files for a specific change.
    ///
    /// `jj diff --summary` emits one line per file: `<STATUS> <PATH>`. Use
    /// `split_once` on the first whitespace so paths containing spaces are
    /// preserved verbatim. Lines without a path are skipped.
    pub fn changed_files(&self, change_id: &str) -> Result<Vec<PathBuf>> {
        let output = self.execute(&["diff", "-r", change_id, "--summary"])?;

        let files: Vec<PathBuf> = output
            .lines()
            .filter_map(|line| {
                line.split_once(char::is_whitespace)
                    .map(|(_, path)| PathBuf::from(path.trim_start()))
            })
            .collect();

        Ok(files)
    }

    /// Get file contents at a specific revision
    pub fn file_at_revision(&self, revision: &str, path: &str) -> Result<String> {
        self.execute(&["file", "show", "-r", revision, path])
    }

    /// Resolve a revision to its full commit id, or `None` when the revision
    /// is unreachable (GC'd, rewritten, or never existed).
    ///
    /// Used by the delta-fetch state machine to decide between a delta
    /// (`last_synced_rev` still reachable) and a cold-start full reconcile (it
    /// isn't). Distinguishes "no such revision" from real jj failures by
    /// inspecting stderr — auth/corruption errors propagate.
    pub fn resolve_commit(&self, revision: &str) -> Result<Option<String>> {
        match self.execute(&["log", "--no-graph", "-r", revision, "-T", "commit_id"]) {
            Ok(out) => Ok(Some(out.trim().to_string())),
            Err(crate::error::JjjError::JjCommandFailed { ref stderr, .. })
                if stderr.contains("No such revision")
                    || stderr.contains("doesn't exist")
                    || stderr.contains("does not exist")
                    || stderr.contains("unknown revision")
                    || stderr.contains("Revision") =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Resolve the head commit ids of every metadata bookmark (`jjj`, `jjj/*`),
    /// **including remote-tracking refs** (`jjj/*@origin`).
    ///
    /// These are the per-pod single-writer refs (Break #5): fetch reads them
    /// all and unions their deltas. A freshly fetched pod bookmark arrives as a
    /// remote-tracking ref before it is tracked locally, so the union must span
    /// both `bookmarks()` and `remote_bookmarks()` — otherwise a clone that has
    /// never pushed sees "no jjj bookmark" right after fetching one. `heads(...)`
    /// drops any ref that is an ancestor of another (e.g. a stale shared `jjj`
    /// base under newer pod refs), so the result is exactly the set of distinct
    /// tips to merge.
    ///
    /// Returns an empty vec when no metadata bookmark exists yet (fresh remote).
    pub fn meta_head_commits(&self) -> Result<Vec<String>> {
        let glob = format!("{}*", BOOKMARK_PREFIX);
        let revset = format!(
            "heads(bookmarks(glob:{0:?}) | remote_bookmarks(glob:{0:?}))",
            glob
        );
        let out = self.execute(&[
            "log",
            "--no-graph",
            "-r",
            &revset,
            "-T",
            r#"commit_id ++ "\n""#,
        ])?;
        Ok(out
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    /// Greatest common ancestor of two revisions — the **true** three-way merge
    /// base for a delta fetch.
    ///
    /// ⚠️ Keystone correctness (highest-risk, audit 0.1). With per-pod bookmarks
    /// each pod advances its own parallel branch, so `last_synced_rev` (this
    /// pod's last-pushed commit) is **not** an ancestor of another pod's head.
    /// Diffing straight from `last_synced_rev` would reconstruct a base that
    /// already folds in *our own* unpushed edits, and the merge would then
    /// silently revert them to the other pod's stale value. The merge base must
    /// be the genuine common ancestor of the two tips: `heads(::a & ::b)`.
    ///
    /// Returns `None` when the two share no reachable ancestor (e.g. an
    /// unreachable `last_synced_rev`); callers fall back to `root()`, which is
    /// always a safe — if more verbose — ancestor (every file shows as changed
    /// and is three-way merged, never lost).
    pub fn merge_base(&self, a: &str, b: &str) -> Result<Option<String>> {
        let revset = format!("heads(::{} & ::{})", a, b);
        match self.execute(&[
            "log",
            "--no-graph",
            "-r",
            &revset,
            "-T",
            r#"commit_id ++ "\n""#,
        ]) {
            Ok(out) => Ok(out
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(String::from)),
            // An unreachable operand makes the revset itself fail to resolve;
            // treat that as "no common ancestor" so the caller cold-starts.
            Err(crate::error::JjjError::JjCommandFailed { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Full-context git diff between two revisions — the delta-fetch content
    /// primitive (Pillar 1, keystone).
    ///
    /// One subprocess returns every changed file's *entire* content on both
    /// sides: `--context <huge>` forces jj to emit whole-file hunks, so
    /// [`crate::storage::delta::parse_git_diff`] can reconstruct the base side
    /// (context + `-` lines) and the remote side (context + `+` lines) for a
    /// per-file three-way merge — no per-file `jj file show` loop (which
    /// measured ~94ms/file @25K and re-resolves the whole tree each call).
    /// Adds and deletes are explicit in the same diff.
    pub fn delta_git(&self, from: &str, to: &str) -> Result<String> {
        self.execute(&[
            "diff",
            "--from",
            from,
            "--to",
            to,
            "--git",
            "--context",
            FULL_CONTEXT_LINES,
        ])
    }

    /// Squash current change into parent.
    /// If `message` is provided, uses it as the combined description (avoids opening an editor).
    pub fn squash(&self, message: Option<&str>) -> Result<()> {
        match message {
            Some(msg) => self.execute(&["squash", "-m", msg])?,
            None => self.execute(&["squash"])?,
        };
        Ok(())
    }

    /// Edit a specific change
    pub fn edit(&self, change_id: &str) -> Result<()> {
        self.execute(&["edit", change_id])?;
        Ok(())
    }

    /// Check if a change ID exists in the repository.
    ///
    /// Distinguishes "revision not found" from other jj errors by inspecting
    /// stderr — other failures (auth, repo corruption) propagate.
    pub fn change_exists(&self, change_id: &str) -> Result<bool> {
        match self.execute(&["log", "--no-graph", "-r", change_id, "-T", "change_id"]) {
            Ok(_) => Ok(true),
            Err(crate::error::JjjError::JjCommandFailed { ref stderr, .. })
                if stderr.contains("No such revision")
                    || stderr.contains("Revision")
                    || stderr.contains("does not exist") =>
            {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Execute a workspace subcommand using a configurable prefix.
    ///
    /// `workspace_prefix` defaults to `"workspace"` but can be overridden
    /// (e.g., `"citc workspace"`) to support custom jj extensions.
    pub fn execute_workspace(
        &self,
        workspace_prefix: Option<&str>,
        subcommand: &str,
        extra_args: &[&str],
    ) -> Result<String> {
        let prefix = workspace_prefix.unwrap_or("workspace");
        let mut args: Vec<&str> = prefix.split_whitespace().collect();
        args.push(subcommand);
        args.extend_from_slice(extra_args);
        self.execute(&args)
    }

    /// Execute a shell command string with template variable expansion.
    ///
    /// Used for config-driven sync commands. The command is split on whitespace
    /// and executed as a `jj` subprocess (the `jj` prefix is implied — the
    /// command should start with the subcommand, e.g., `"git push -b {bookmark}"`).
    pub fn execute_sync_command(
        &self,
        command_template: &str,
        vars: &[(&str, &str)],
    ) -> Result<String> {
        let mut expanded = command_template.to_string();
        for (key, value) in vars {
            expanded = expanded.replace(&format!("{{{}}}", key), value);
        }
        let args: Vec<&str> = expanded.split_whitespace().collect();
        self.execute(&args)
    }

    /// Get user name from config
    pub fn user_name(&self) -> Result<String> {
        let output = self.execute(&["config", "get", "user.name"])?;
        Ok(output.trim().trim_matches('"').to_string())
    }

    /// Get user email from config
    pub fn user_email(&self) -> Result<String> {
        let output = self.execute(&["config", "get", "user.email"])?;
        Ok(output.trim().trim_matches('"').to_string())
    }

    /// Get formatted user identity (Name <email>)
    pub fn user_identity(&self) -> Result<String> {
        let name = self.user_name()?;
        let email = self.user_email()?;
        Ok(format!("{} <{}>", name, email))
    }
}

/// Find an executable by name on the system PATH using stdlib only.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// Parse `(major, minor)` from `jj version` output.
///
/// Expected format: `jj 0.25.0` or `jj 0.25.0-dev`. Returns `None` if the
/// version string can't be parsed.
fn jj_version(jj_path: &Path) -> Option<(u32, u32)> {
    let output = Command::new(jj_path).arg("version").output().ok()?;
    let s = std::str::from_utf8(&output.stdout).ok()?;
    let ver = s.split_whitespace().nth(1)?;
    let mut parts = ver.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_detection() {
        // This test will fail if jj is not installed
        match find_executable("jj") {
            Some(_) => println!("jj found in PATH"),
            None => println!("jj not found - some tests will be skipped"),
        }
    }
}
