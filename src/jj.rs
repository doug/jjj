use crate::error::{JjjError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Interface to the jj version control system
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

        let repo_root = Self::find_repo_root()?;

        Ok(Self { jj_path, repo_root })
    }

    pub fn with_root(root: PathBuf) -> Result<Self> {
        let jj_path = find_executable("jj").ok_or(JjjError::JjNotFound)?;
        Ok(Self {
            jj_path,
            repo_root: root,
        })
    }

    /// Find the repository root by looking for .jj directory
    fn find_repo_root() -> Result<PathBuf> {
        let current_dir = std::env::current_dir()?;

        let mut dir = current_dir.as_path();
        loop {
            let jj_dir = dir.join(".jj");
            if jj_dir.exists() && jj_dir.is_dir() {
                return Ok(dir.to_path_buf());
            }

            match dir.parent() {
                Some(parent) => dir = parent,
                None => return Err(JjjError::NotInRepository),
            }
        }
    }

    /// Get the repository root
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
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
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::JjjError::JjCommandFailed {
                args: args.join(" "),
                stderr: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get the current change ID
    pub fn current_change_id(&self) -> Result<String> {
        let output = self.execute(&["log", "--no-graph", "-r", "@", "-T", "change_id"])?;
        Ok(output.trim().to_string())
    }

    /// Check if a bookmark exists
    pub fn bookmark_exists(&self, bookmark: &str) -> Result<bool> {
        let output = self.execute(&["bookmark", "list"])?;
        Ok(output.lines().any(|line| line.contains(bookmark)))
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
            Err(_) => Ok(false),
        }
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
