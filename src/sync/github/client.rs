//! GitHub CLI (`gh`) subprocess wrapper.
//!
//! Mirrors the `JjClient` pattern from `src/jj.rs`, wrapping the `gh` binary
//! via `std::process::Command` for GitHub API access.

use crate::error::{JjjError, Result};
use crate::jj::find_executable;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Wrapper around the `gh` CLI for GitHub API operations.
#[derive(Debug, Clone)]
pub struct GhClient {
    /// Path to the gh executable
    gh_path: PathBuf,

    /// Repository root directory (for CWD context)
    repo_root: PathBuf,
}

impl GhClient {
    /// Create a new GhClient, discovering the `gh` executable.
    pub fn new(repo_root: &Path) -> Result<Self> {
        let gh_path = find_executable("gh").ok_or(JjjError::GhNotFound)?;
        Ok(Self {
            gh_path,
            repo_root: repo_root.to_path_buf(),
        })
    }

    /// Execute a gh command and return stdout.
    pub fn execute(&self, args: &[&str]) -> Result<String> {
        if std::env::var("JJJ_DEBUG").is_ok() {
            eprintln!("DEBUG: gh {}", args.join(" "));
        }

        let output = Command::new(&self.gh_path)
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| JjjError::GhIo {
                args: args.join(" "),
                source: e,
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Detect auth failures
            if stderr.contains("auth login") || stderr.contains("not logged") {
                return Err(JjjError::GhAuthFailed);
            }

            return Err(JjjError::GhCommandFailed {
                args: args.join(" "),
                stderr,
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Check authentication status. Returns the authenticated username.
    pub fn auth_status(&self) -> Result<String> {
        let output = self.execute(&["auth", "status"])?;
        // Parse "Logged in to github.com account USERNAME"
        for line in output.lines() {
            if let Some(rest) = line.strip_prefix("Logged in to") {
                if let Some(account) = rest.split("account ").nth(1) {
                    return Ok(account.split_whitespace().next().unwrap_or("").to_string());
                }
            }
        }
        // Fallback: if command succeeded, try to get username directly
        self.execute(&["api", "user", "--jq", ".login"])
    }

    /// Detect repository owner and name.
    pub fn detect_repo(&self) -> Result<(String, String)> {
        let output = self.execute(&[
            "repo",
            "view",
            "--json",
            "owner,name",
            "--jq",
            ".owner.login + \"/\" + .name",
        ])?;

        let parts: Vec<&str> = output.trim().splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(JjjError::GhRepoNotDetected);
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Create a GitHub issue. Returns the issue number.
    pub fn create_issue(&self, title: &str, body: &str, labels: &[&str]) -> Result<u64> {
        let mut args = vec!["issue", "create", "--title", title, "--body", body];
        for label in labels {
            args.push("--label");
            args.push(label);
        }
        args.extend_from_slice(&["--json", "number", "--jq", ".number"]);

        let output = self.execute(&args)?;
        output
            .trim()
            .parse()
            .map_err(|_| JjjError::Validation(format!("Failed to parse issue number: {}", output)))
    }

    /// Get issue details as JSON.
    pub fn get_issue(&self, number: u64) -> Result<serde_json::Value> {
        let num_str = number.to_string();
        let output = self.execute(&[
            "issue",
            "view",
            &num_str,
            "--json",
            "number,title,body,state,labels,author",
        ])?;
        serde_json::from_str(&output).map_err(|e| JjjError::Validation(e.to_string()))
    }

    /// List issues with optional label and state filter.
    pub fn list_issues(&self, label: Option<&str>, state: &str) -> Result<Vec<serde_json::Value>> {
        let mut args = vec![
            "issue",
            "list",
            "--state",
            state,
            "--json",
            "number,title,state,labels",
            "--limit",
            "100",
        ];
        if let Some(l) = label {
            args.push("--label");
            args.push(l);
        }
        let output = self.execute(&args)?;
        serde_json::from_str(&output).map_err(|e| JjjError::Validation(e.to_string()))
    }

    /// Close an issue.
    pub fn close_issue(&self, number: u64) -> Result<()> {
        let num_str = number.to_string();
        self.execute(&["issue", "close", &num_str])?;
        Ok(())
    }

    /// Reopen an issue.
    pub fn reopen_issue(&self, number: u64) -> Result<()> {
        let num_str = number.to_string();
        self.execute(&["issue", "reopen", &num_str])?;
        Ok(())
    }

    /// Create a pull request. Returns the PR number.
    pub fn create_pr(&self, title: &str, body: &str, head: &str, base: &str) -> Result<u64> {
        let output = self.execute(&[
            "pr", "create", "--title", title, "--body", body, "--head", head, "--base", base,
            "--json", "number", "--jq", ".number",
        ])?;
        output
            .trim()
            .parse()
            .map_err(|_| JjjError::Validation(format!("Failed to parse PR number: {}", output)))
    }

    /// Get PR details as JSON.
    pub fn get_pr(&self, number: u64) -> Result<serde_json::Value> {
        let num_str = number.to_string();
        let output = self.execute(&[
            "pr",
            "view",
            &num_str,
            "--json",
            "number,title,body,state,reviews,headRefName",
        ])?;
        serde_json::from_str(&output).map_err(|e| JjjError::Validation(e.to_string()))
    }

    /// Merge a pull request (squash merge).
    pub fn merge_pr(&self, number: u64) -> Result<()> {
        let num_str = number.to_string();
        self.execute(&["pr", "merge", &num_str, "--squash", "--delete-branch"])?;
        Ok(())
    }

    /// List reviews for a PR.
    pub fn list_reviews(&self, pr_number: u64) -> Result<serde_json::Value> {
        let num_str = pr_number.to_string();
        let output = self.execute(&[
            "pr", "view", &num_str, "--json", "reviews", "--jq", ".reviews",
        ])?;
        serde_json::from_str(&output).map_err(|e| JjjError::Validation(e.to_string()))
    }

    /// Get PR state (OPEN, MERGED, CLOSED).
    pub fn pr_state(&self, number: u64) -> Result<String> {
        let num_str = number.to_string();
        self.execute(&["pr", "view", &num_str, "--json", "state", "--jq", ".state"])
    }

    /// Get issue state (OPEN, CLOSED).
    pub fn issue_state(&self, number: u64) -> Result<String> {
        let num_str = number.to_string();
        self.execute(&[
            "issue", "view", &num_str, "--json", "state", "--jq", ".state",
        ])
    }
}
