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

    /// Map GitHub issue labels to jjj priority (e.g., "P0" → "critical", "P1" → "high")
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

    /// Automation rules — fire actions on jjj events
    #[serde(default)]
    pub automation: Vec<AutomationRule>,
}
