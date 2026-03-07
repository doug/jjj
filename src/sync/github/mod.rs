//! GitHub implementation of the SyncProvider trait.

pub mod client;
pub mod mapping;

use crate::error::{JjjError, Result};
use crate::models::{Problem, Solution};
use crate::sync::{IssueStatus, PrStatus, ReviewInfo, ReviewThread, SyncProvider};

use self::client::GhClient;

/// GitHub sync provider using the `gh` CLI.
pub struct GitHubProvider {
    client: GhClient,
    /// Override repo from config (owner/repo)
    repo_override: Option<String>,
    /// Label to apply to synced issues
    problem_label: String,
    /// Label-to-priority mapping from config
    label_priority: std::collections::HashMap<String, String>,
}

impl GitHubProvider {
    /// Create a new GitHubProvider.
    pub fn new(
        client: GhClient,
        repo_override: Option<String>,
        problem_label: String,
        label_priority: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            client,
            repo_override,
            problem_label,
            label_priority,
        }
    }

    /// Create from a config and repo root path.
    pub fn from_config(
        repo_root: &std::path::Path,
        config: &crate::models::GitHubConfig,
    ) -> Result<Self> {
        let client = GhClient::new(repo_root)?;
        Ok(Self::new(
            client,
            config.repo.clone(),
            config.problem_label.clone(),
            config.label_priority.clone(),
        ))
    }
}

impl SyncProvider for GitHubProvider {
    fn name(&self) -> &str {
        "github"
    }

    fn check_auth(&self) -> Result<String> {
        self.client.auth_status()
    }

    fn detect_repo(&self) -> Result<(String, String)> {
        if let Some(ref repo) = self.repo_override {
            let parts: Vec<&str> = repo.splitn(2, '/').collect();
            if parts.len() == 2 {
                return Ok((parts[0].to_string(), parts[1].to_string()));
            }
            return Err(JjjError::Validation(format!(
                "Invalid repo format '{}', expected 'owner/repo'",
                repo
            )));
        }
        self.client.detect_repo()
    }

    fn import_issue(&self, number: u64) -> Result<Problem> {
        let json = self.client.get_issue(number)?;
        let mut problem = mapping::issue_to_problem(&json, number)?;
        // Apply label → priority mapping if configured
        if !self.label_priority.is_empty() {
            if let Some(labels) = json["labels"].as_array() {
                for label in labels {
                    if let Some(name) = label["name"].as_str() {
                        if let Some(priority_str) = self.label_priority.get(name) {
                            if let Ok(priority) = priority_str.parse::<crate::models::Priority>() {
                                problem.priority = priority;
                                break;
                            }
                        }
                    }
                }
            }
        }
        Ok(problem)
    }

    fn list_unlinked_issues(
        &self,
        existing: &[(String, u64)],
        label: Option<&str>,
    ) -> Result<Vec<(u64, String)>> {
        let issues = self.client.list_issues(label, "open")?;
        let linked_numbers: std::collections::HashSet<u64> =
            existing.iter().map(|(_, n)| *n).collect();

        let mut unlinked = Vec::new();
        for issue in issues {
            if let (Some(number), Some(title)) = (issue["number"].as_u64(), issue["title"].as_str())
            {
                if !linked_numbers.contains(&number) {
                    unlinked.push((number, title.to_string()));
                }
            }
        }
        Ok(unlinked)
    }

    fn create_issue(&self, problem: &Problem) -> Result<u64> {
        let body = mapping::problem_to_issue_body(problem);
        let labels: Vec<&str> = if self.problem_label.is_empty() {
            vec![]
        } else {
            vec![&self.problem_label]
        };
        self.client.create_issue(&problem.title, &body, &labels)
    }

    fn create_pr(&self, solution: &Solution, problem: &Problem, branch: &str) -> Result<u64> {
        let critiques = Vec::new(); // Critiques loaded separately by caller
        let body = mapping::format_pr_body(solution, problem, &critiques);
        let title = format!("{}: {}", problem.title, solution.title);
        self.client.create_pr(&title, &body, branch, "main")
    }

    fn merge_pr(&self, number: u64) -> Result<()> {
        self.client.merge_pr(number)
    }

    fn close_issue(&self, number: u64) -> Result<()> {
        self.client.close_issue(number)
    }

    fn reopen_issue(&self, number: u64) -> Result<()> {
        self.client.reopen_issue(number)
    }

    fn pull_reviews(&self, pr_number: u64) -> Result<Vec<ReviewInfo>> {
        let json = self.client.list_reviews(pr_number)?;
        Ok(mapping::parse_reviews(&json))
    }

    fn pull_review_threads(&self, pr_number: u64) -> Result<Vec<ReviewThread>> {
        let json = self.client.list_review_threads(pr_number)?;
        Ok(mapping::parse_review_threads(&json))
    }

    fn pr_status(&self, pr_number: u64) -> Result<PrStatus> {
        let state = self.client.pr_state(pr_number)?;
        Ok(mapping::parse_pr_state(&state))
    }

    fn issue_status(&self, number: u64) -> Result<IssueStatus> {
        let state = self.client.issue_state(number)?;
        Ok(mapping::parse_issue_state(&state))
    }
}

impl GitHubProvider {
    /// Get the current body text of a PR.
    pub fn get_pr_body(&self, number: u64) -> Result<String> {
        self.client.get_pr_body(number)
    }

    /// Update the body of an existing PR.
    pub fn update_pr_body(
        &self,
        number: u64,
        solution: &crate::models::Solution,
        problem: &crate::models::Problem,
        critiques: &[crate::models::Critique],
    ) -> Result<()> {
        let body = mapping::format_pr_body(solution, problem, critiques);
        self.client.update_pr_body(number, &body)
    }
}
