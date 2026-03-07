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

/// Information about an inline PR review thread (file-level comment)
#[derive(Debug, Clone)]
pub struct ReviewThread {
    /// databaseId of the first comment in the thread (used for dedup via github_review_id)
    pub comment_id: u64,
    pub author: String,
    pub body: String,
    /// Relative file path in the repository
    pub path: String,
    /// Line number in the file (1-indexed)
    pub line: Option<usize>,
    /// Whether the thread was resolved on GitHub
    pub is_resolved: bool,
    /// Whether the thread is on an outdated diff hunk
    pub is_outdated: bool,
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
    fn list_unlinked_issues(
        &self,
        existing: &[(String, u64)],
        label: Option<&str>,
    ) -> Result<Vec<(u64, String)>>;

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

    /// Merge a pull request.
    fn merge_pr(&self, number: u64) -> Result<()>;

    /// Close an issue.
    fn close_issue(&self, number: u64) -> Result<()>;

    /// Reopen an issue.
    fn reopen_issue(&self, number: u64) -> Result<()>;

    // -- Sync (pull status) --

    /// Pull reviews for a PR.
    fn pull_reviews(&self, pr_number: u64) -> Result<Vec<ReviewInfo>>;

    /// Pull inline review threads (file-level comments) for a PR.
    /// Default implementation returns empty — providers override to support line-level import.
    fn pull_review_threads(&self, pr_number: u64) -> Result<Vec<ReviewThread>> {
        let _ = pr_number;
        Ok(vec![])
    }

    /// Get the status of a PR.
    fn pr_status(&self, pr_number: u64) -> Result<PrStatus>;

    /// Get the status of an issue.
    fn issue_status(&self, number: u64) -> Result<IssueStatus>;
}

/// Build a critique title from a GitHub author and review body.
///
/// If the body has a meaningful first line (> 5 chars), uses `"@author: first line"`.
/// Otherwise falls back to `"GitHub review from @author"`.
fn review_title(author: &str, body: &str) -> String {
    let first_line = body.lines().next().unwrap_or("").trim();
    if first_line.len() > 5 {
        let truncated: String = first_line.chars().take(80).collect();
        format!("@{}: {}", author, truncated)
    } else {
        format!("GitHub review from @{}", author)
    }
}

/// Create a critique from a top-level PR review.
pub fn review_to_critique(review: &ReviewInfo, solution_id: &str, critique_id: String) -> Critique {
    let mut critique = Critique::new(
        critique_id,
        review_title(&review.author, &review.body),
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

/// Create a critique from an inline PR review thread comment.
///
/// The thread's `comment_id` (first comment's databaseId) is stored as
/// `github_review_id` for deduplication.  If the thread has a file path
/// and line number, those are set on the critique so it shows up as an
/// inline annotation in editors.
pub fn thread_to_critique(
    thread: &ReviewThread,
    solution_id: &str,
    critique_id: String,
) -> Critique {
    let mut critique = Critique::new(
        critique_id,
        review_title(&thread.author, &thread.body),
        solution_id.to_string(),
    );
    critique.author = Some(thread.author.clone());
    critique.argument = thread.body.clone();
    critique.github_review_id = Some(thread.comment_id);

    if !thread.path.is_empty() {
        critique.file_path = Some(thread.path.clone());
        critique.line_start = thread.line;
        critique.line_end = thread.line;
    }

    if thread.is_outdated {
        critique.evidence = "Note: this comment is on an outdated diff hunk.".to_string();
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
        assert_eq!(
            critique.title,
            "@alice: The error handling needs improvement."
        );
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
        assert_eq!(critique.title, "@carol: Looks good to me!");
    }

    #[test]
    fn test_review_to_critique_dismissed_default_severity() {
        let review = make_review(ReviewState::Dismissed, "dave", "Dismissed review.");

        let critique = review_to_critique(&review, "S-40", "C-400".to_string());

        // Dismissed also falls through to default severity
        assert_eq!(critique.severity, CritiqueSeverity::Medium);
        assert_eq!(critique.title, "@dave: Dismissed review.");
    }

    #[test]
    fn test_review_to_critique_empty_body() {
        let review = make_review(ReviewState::ChangesRequested, "eve", "");

        let critique = review_to_critique(&review, "S-50", "C-500".to_string());

        assert_eq!(critique.argument, "");
        assert_eq!(critique.severity, CritiqueSeverity::High);
        assert_eq!(critique.title, "GitHub review from @eve");
    }

    #[test]
    fn test_review_title_short_body_falls_back() {
        // Body with <= 5 chars uses the fallback title
        assert_eq!(review_title("alice", "ok"), "GitHub review from @alice");
        assert_eq!(review_title("bob", ""), "GitHub review from @bob");
        assert_eq!(review_title("carol", "nope"), "GitHub review from @carol");
    }

    #[test]
    fn test_review_title_uses_first_line() {
        let body = "This line is used as title\nSecond line is ignored";
        assert_eq!(
            review_title("alice", body),
            "@alice: This line is used as title"
        );
    }

    #[test]
    fn test_review_title_truncates_at_80_chars() {
        let long = "a".repeat(100);
        let title = review_title("bob", &long);
        // Should be "@bob: " (6) + 80 'a' chars = 86 chars
        assert_eq!(title, format!("@bob: {}", "a".repeat(80)));
    }

    #[test]
    fn test_thread_to_critique_with_location() {
        let thread = ReviewThread {
            comment_id: 99001,
            author: "alice".to_string(),
            body: "Missing error check here".to_string(),
            path: "src/auth.rs".to_string(),
            line: Some(42),
            is_resolved: false,
            is_outdated: false,
        };

        let critique = thread_to_critique(&thread, "S-10", "C-200".to_string());

        assert_eq!(critique.title, "@alice: Missing error check here");
        assert_eq!(critique.author, Some("alice".to_string()));
        assert_eq!(critique.argument, "Missing error check here");
        assert_eq!(critique.file_path, Some("src/auth.rs".to_string()));
        assert_eq!(critique.line_start, Some(42));
        assert_eq!(critique.github_review_id, Some(99001));
        assert!(critique.evidence.is_empty());
    }

    #[test]
    fn test_thread_to_critique_outdated_adds_evidence() {
        let thread = ReviewThread {
            comment_id: 99002,
            author: "bob".to_string(),
            body: "Old comment on stale diff".to_string(),
            path: "src/lib.rs".to_string(),
            line: Some(10),
            is_resolved: false,
            is_outdated: true,
        };

        let critique = thread_to_critique(&thread, "S-20", "C-300".to_string());

        assert!(critique.evidence.contains("outdated"));
    }

    #[test]
    fn test_thread_to_critique_no_location_when_path_empty() {
        let thread = ReviewThread {
            comment_id: 99003,
            author: "carol".to_string(),
            body: "General comment without file context".to_string(),
            path: "".to_string(),
            line: None,
            is_resolved: false,
            is_outdated: false,
        };

        let critique = thread_to_critique(&thread, "S-30", "C-400".to_string());

        assert!(critique.file_path.is_none());
        assert!(critique.line_start.is_none());
    }
}
