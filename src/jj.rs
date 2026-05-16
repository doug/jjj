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

impl JjClient {
    /// Create a new JjClient, discovering the jj executable and repo root
    pub fn new() -> Result<Self> {
        let jj_path = find_executable("jj").ok_or(JjjError::JjNotFound)?;

        // Check jj version (warn if older than 0.25.0, but don't block)
        if let Ok(output) = Command::new(&jj_path).arg("version").output() {
            if let Ok(version_str) = std::str::from_utf8(&output.stdout) {
                // Expected format: "jj 0.25.0" or "jj 0.25.0-dev"
                if let Some(ver) = version_str.split_whitespace().nth(1) {
                    let parts: Vec<&str> = ver.split('.').collect();
                    if let (Some(Ok(major)), Some(Ok(minor))) = (
                        parts.first().map(|s| s.parse::<u32>()),
                        parts.get(1).map(|s| s.parse::<u32>()),
                    ) {
                        if major == 0 && minor < 25 {
                            eprintln!(
                                "Warning: jj version {} detected; jjj requires 0.25.0 or later",
                                ver
                            );
                        }
                    }
                }
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

    /// Check if a bookmark exists
    pub fn bookmark_exists(&self, bookmark: &str) -> Result<bool> {
        let output = self.execute(&["bookmark", "list"])?;
        Ok(output.lines().any(|line| {
            let name = line.split_whitespace().next().unwrap_or("");
            name == bookmark || name.trim_end_matches(':') == bookmark
        }))
    }

    /// Create a new bookmark
    pub fn create_bookmark(&self, name: &str, revision: &str) -> Result<()> {
        self.execute(&["bookmark", "create", name, "-r", revision])?;
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

    /// Get changed files for a specific change
    pub fn changed_files(&self, change_id: &str) -> Result<Vec<PathBuf>> {
        let output = self.execute(&["diff", "-r", change_id, "--summary"])?;

        let files: Vec<PathBuf> = output
            .lines()
            .filter_map(|line| {
                // Parse jj diff summary format
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(PathBuf::from(parts[1]))
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }

    /// Get file contents at a specific revision
    pub fn file_at_revision(&self, revision: &str, path: &str) -> Result<String> {
        self.execute(&["file", "show", "-r", revision, path])
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

    /// Check if a change ID exists in the repository
    pub fn change_exists(&self, change_id: &str) -> Result<bool> {
        match self.execute(&["log", "--no-graph", "-r", change_id, "-T", "change_id"]) {
            Ok(_) => Ok(true),
            Err(crate::error::JjjError::JjCommandFailed { .. }) => Ok(false),
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

/// Find an executable by name on the system PATH using stdlib only.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}
