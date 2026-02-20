pub mod github;
pub mod hooks;

use crate::error::Result;
use crate::models::{Critique, Problem};

/// State of a GitHub review
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Commented,
    Dismissed,
}

/// Information about a pull request review
#[derive(Debug, Clone)]
pub struct ReviewInfo {
    pub id: u64,
    pub author: String,
    pub state: ReviewState,
    pub body: String,
}

/// Status of a pull request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrStatus {
    Open,
    Merged,
    Closed,
}

/// Status of an issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueStatus {
    Open,
    Closed,
}

/// Backend-agnostic sync provider trait.
///
/// Implementations provide bidirectional sync between jjj entities
/// and external issue/PR tracking systems.
pub trait SyncProvider {
    /// Name of this provider (e.g., "github")
    fn name(&self) -> &str;

    /// Check authentication status. Returns the authenticated username.
    fn check_auth(&self) -> Result<String>;

    /// Detect the remote repository. Returns (owner, repo).
    fn detect_repo(&self) -> Result<(String, String)>;

    // -- Import (remote -> jjj) --

    /// Import a single issue by number as a Problem.
    fn import_issue(&self, number: u64) -> Result<Problem>;

    /// List remote issues not yet linked to local problems.
    /// `existing` contains (problem_id, github_issue_number) pairs.
    fn list_unlinked_issues(&self, existing: &[(String, u64)]) -> Result<Vec<(u64, String)>>;

    // -- Push (jjj -> remote) --

    /// Create a remote issue from a Problem. Returns the issue number.
    fn create_issue(&self, problem: &Problem) -> Result<u64>;

    /// Create a pull request for a solution. Returns the PR number.
    fn create_pr(
        &self,
        solution: &crate::models::Solution,
        problem: &Problem,
        branch: &str,
    ) -> Result<u64>;

    /// Update an existing pull request.
    fn update_pr(
        &self,
        number: u64,
        solution: &crate::models::Solution,
        problem: &Problem,
    ) -> Result<()>;

    /// Merge a pull request.
    fn merge_pr(&self, number: u64) -> Result<()>;

    /// Close an issue.
    fn close_issue(&self, number: u64) -> Result<()>;

    /// Reopen an issue.
    fn reopen_issue(&self, number: u64) -> Result<()>;

    // -- Sync (pull status) --

    /// Pull reviews for a PR.
    fn pull_reviews(&self, pr_number: u64) -> Result<Vec<ReviewInfo>>;

    /// Get the status of a PR.
    fn pr_status(&self, pr_number: u64) -> Result<PrStatus>;

    /// Get the status of an issue.
    fn issue_status(&self, number: u64) -> Result<IssueStatus>;
}

/// Create a critique from a review.
pub fn review_to_critique(review: &ReviewInfo, solution_id: &str, critique_id: String) -> Critique {
    let mut critique = Critique::new(
        critique_id,
        format!("GitHub review from {}", review.author),
        solution_id.to_string(),
    );
    critique.author = Some(review.author.clone());
    critique.argument = review.body.clone();
    critique.github_review_id = Some(review.id);

    match review.state {
        ReviewState::ChangesRequested => {
            critique.severity = crate::models::CritiqueSeverity::High;
        }
        ReviewState::Commented => {
            critique.severity = crate::models::CritiqueSeverity::Medium;
        }
        _ => {}
    }

    critique
}
