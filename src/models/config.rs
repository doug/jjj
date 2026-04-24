use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::event::EventType;

fn default_true() -> bool {
    true
}

fn default_problem_label() -> String {
    "jjj".to_string()
}

/// GitHub integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// Enable GitHub integration (None = auto-detect, Some(false) = disabled)
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Override repository: "owner/repo"
    #[serde(default)]
    pub repo: Option<String>,

    /// Opt-in: automatically create issues/PRs on local operations
    #[serde(default)]
    pub auto_push: bool,

    /// Import "Request Changes" reviews as critiques
    #[serde(default = "default_true")]
    pub sync_critiques: bool,

    /// Import "Approve" reviews
    #[serde(default = "default_true")]
    pub sync_lgtm: bool,

    /// Auto-close linked GitHub issue when `problem solve` runs.
    /// Finer-grained than `auto_push` (which also controls issue/PR creation).
    #[serde(default)]
    pub auto_close_on_solve: bool,

    /// Label applied to synced GitHub issues
    #[serde(default = "default_problem_label")]
    pub problem_label: String,

    /// Map GitHub issue labels to jjj priority (e.g., "urgent" → "critical", "important" → "high")
    #[serde(default)]
    pub label_priority: HashMap<String, String>,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            repo: None,
            auto_push: false,
            sync_critiques: true,
            sync_lgtm: true,
            auto_close_on_solve: false,
            problem_label: default_problem_label(),
            label_priority: HashMap::new(),
        }
    }
}

/// Built-in automation actions and the generic "shell" escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationAction {
    Shell,
    GithubPr,
    GithubMerge,
    GithubClose,
    GithubIssue,
    GithubSync,
}

/// A single automation rule: when event `on` fires, execute `action`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    /// Event type to match (e.g., `EventType::SolutionSubmitted`)
    pub on: EventType,

    /// Action to perform: built-in name or `Shell`
    pub action: AutomationAction,

    /// Shell command template (required when action = Shell)
    #[serde(default)]
    pub command: Option<String>,

    /// Enable/disable without removing the rule (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Sync configuration for push/fetch operations.
///
/// Commands are shell strings with template variables:
/// - `{remote}` — remote name (default: "origin")
/// - `{bookmark}` — bookmark name being pushed
///
/// When not configured, jjj auto-detects: uses `jj git push/fetch` for
/// git-backed repos, and errors for other backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Command to fetch metadata from remote.
    /// Example: "jj git fetch --remote {remote}"
    #[serde(default)]
    pub fetch: Option<String>,

    /// Command to push a bookmark to remote.
    /// Example: "jj git push -b {bookmark} --remote {remote} --allow-empty-description"
    #[serde(default)]
    pub push: Option<String>,

    /// Command to track a remote bookmark locally.
    /// Example: "jj bookmark track {bookmark} --remote {remote}"
    #[serde(default)]
    pub track: Option<String>,

    /// Workspace command prefix. Replaces `workspace` in all jj workspace
    /// subcommands (add, update-stale, etc.).
    /// Example for CitC: "citc workspace"
    /// Default: "workspace"
    #[serde(default)]
    pub workspace: Option<String>,

}

impl SyncConfig {
    /// Resolve the fetch command: explicit config > git default > None.
    pub fn resolve_fetch(&self, has_git: bool) -> Option<String> {
        if let Some(ref cmd) = self.fetch {
            return Some(cmd.clone());
        }
        if has_git {
            return Some("--ignore-working-copy git fetch --remote {remote}".to_string());
        }
        None
    }

    /// Resolve the push command: explicit config > git default > None.
    pub fn resolve_push(&self, has_git: bool) -> Option<String> {
        if let Some(ref cmd) = self.push {
            return Some(cmd.clone());
        }
        if has_git {
            return Some(
                "git push -b {bookmark} --remote {remote} --allow-empty-description".to_string(),
            );
        }
        None
    }

    /// Resolve the track command: explicit config > git default > None.
    pub fn resolve_track(&self, has_git: bool) -> Option<String> {
        if let Some(ref cmd) = self.track {
            return Some(cmd.clone());
        }
        if has_git {
            return Some("bookmark track {bookmark} --remote {remote}".to_string());
        }
        None
    }

    /// Resolve the workspace prefix: explicit config > "workspace".
    pub fn workspace_prefix(&self) -> &str {
        self.workspace.as_deref().unwrap_or("workspace")
    }


    /// Whether any sync mechanism is available.
    pub fn has_sync(&self, has_git: bool) -> bool {
        self.fetch.is_some() || self.push.is_some() || has_git
    }
}

/// Project-wide configuration stored in config.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    #[serde(default)]
    pub name: Option<String>,

    /// Default reviewers
    #[serde(default)]
    pub default_reviewers: Vec<String>,

    /// Custom settings
    #[serde(default)]
    pub settings: HashMap<String, String>,

    /// GitHub integration settings
    #[serde(default)]
    pub github: GitHubConfig,

    /// Sync configuration for push/fetch
    #[serde(default)]
    pub sync: SyncConfig,

    /// Automation rules — fire actions on jjj events
    #[serde(default)]
    pub automation: Vec<AutomationRule>,
}
