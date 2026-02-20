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
    /// `label` optionally filters to issues with a specific label.
    fn list_unlinked_issues(&self, existing: &[(String, u64)], label: Option<&str>) -> Result<Vec<(u64, String)>>;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    fn make_review(state: ReviewState, author: &str, body: &str) -> ReviewInfo {
        ReviewInfo {
            id: 5001,
            author: author.to_string(),
            state,
            body: body.to_string(),
        }
    }

    #[test]
    fn test_review_to_critique_changes_requested() {
        let review = make_review(
            ReviewState::ChangesRequested,
            "alice",
            "The error handling needs improvement.",
        );

        let critique = review_to_critique(&review, "S-10", "C-100".to_string());

        assert_eq!(critique.id, "C-100");
        assert_eq!(critique.solution_id, "S-10");
        assert_eq!(critique.title, "GitHub review from alice");
        assert_eq!(critique.author, Some("alice".to_string()));
        assert_eq!(critique.argument, "The error handling needs improvement.");
        assert_eq!(critique.github_review_id, Some(5001));
        assert_eq!(critique.severity, CritiqueSeverity::High);
        assert_eq!(critique.status, CritiqueStatus::Open);
    }

    #[test]
    fn test_review_to_critique_commented() {
        let review = make_review(
            ReviewState::Commented,
            "bob",
            "Consider using a match statement here.",
        );

        let critique = review_to_critique(&review, "S-20", "C-200".to_string());

        assert_eq!(critique.severity, CritiqueSeverity::Medium);
        assert_eq!(critique.author, Some("bob".to_string()));
        assert_eq!(critique.argument, "Consider using a match statement here.");
    }

    #[test]
    fn test_review_to_critique_approved_default_severity() {
        let review = make_review(ReviewState::Approved, "carol", "Looks good to me!");

        let critique = review_to_critique(&review, "S-30", "C-300".to_string());

        // Approved does not match ChangesRequested or Commented branches,
        // so severity stays at the Critique::new default (Medium).
        assert_eq!(critique.severity, CritiqueSeverity::Medium);
        assert_eq!(critique.author, Some("carol".to_string()));
        assert_eq!(critique.argument, "Looks good to me!");
        assert_eq!(critique.github_review_id, Some(5001));
    }

    #[test]
    fn test_review_to_critique_dismissed_default_severity() {
        let review = make_review(ReviewState::Dismissed, "dave", "Dismissed review.");

        let critique = review_to_critique(&review, "S-40", "C-400".to_string());

        // Dismissed also falls through to default severity
        assert_eq!(critique.severity, CritiqueSeverity::Medium);
        assert_eq!(critique.title, "GitHub review from dave");
    }

    #[test]
    fn test_review_to_critique_empty_body() {
        let review = make_review(ReviewState::ChangesRequested, "eve", "");

        let critique = review_to_critique(&review, "S-50", "C-500".to_string());

        assert_eq!(critique.argument, "");
        assert_eq!(critique.severity, CritiqueSeverity::High);
    }
}
