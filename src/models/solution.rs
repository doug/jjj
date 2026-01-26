use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A solution is a conjecture - a tentative attempt to solve a problem.
/// Solutions must face explicit criticism to survive or be refuted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    /// Unique solution identifier (e.g., "S-1")
    pub id: String,

    /// Solution title (what is the proposed approach?)
    pub title: String,

    /// Problem this solution attempts to solve (required)
    pub problem_id: String,

    /// Current status
    pub status: SolutionStatus,

    /// Critique IDs - critiques of this solution
    #[serde(default)]
    pub critique_ids: Vec<String>,

    /// Associated jj change IDs implementing this solution
    #[serde(default)]
    pub change_ids: Vec<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Assigned owner
    pub assignee: Option<String>,

    /// Requested reviewers
    #[serde(default)]
    pub requested_reviewers: Vec<String>,

    /// People who have LGTM'd
    #[serde(default)]
    pub reviewed_by: Vec<String>,

    /// Override project default for review requirement
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Approach - how this solution addresses the problem (markdown body)
    #[serde(default)]
    pub approach: String,

    /// Trade-offs - known limitations and trade-offs
    #[serde(default)]
    pub tradeoffs: String,
}

/// Status of a solution (conjecture)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SolutionStatus {
    /// Conjecture put forward, not yet tested
    Proposed,

    /// Being implemented/tested
    Testing,

    /// Criticism has shown this won't work
    Refuted,

    /// Survived criticism, adopted as current best solution
    Accepted,
}

impl Default for SolutionStatus {
    fn default() -> Self {
        SolutionStatus::Proposed
    }
}

impl std::fmt::Display for SolutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolutionStatus::Proposed => write!(f, "proposed"),
            SolutionStatus::Testing => write!(f, "testing"),
            SolutionStatus::Refuted => write!(f, "refuted"),
            SolutionStatus::Accepted => write!(f, "accepted"),
        }
    }
}

impl std::str::FromStr for SolutionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(SolutionStatus::Proposed),
            "testing" => Ok(SolutionStatus::Testing),
            "refuted" => Ok(SolutionStatus::Refuted),
            "accepted" => Ok(SolutionStatus::Accepted),
            _ => Err(format!("Unknown solution status: {}", s)),
        }
    }
}

impl Solution {
    /// Create a new solution for a problem
    pub fn new(id: String, title: String, problem_id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            problem_id,
            status: SolutionStatus::Proposed,
            critique_ids: Vec::new(),
            change_ids: Vec::new(),
            tags: HashSet::new(),
            assignee: None,
            requested_reviewers: Vec::new(),
            reviewed_by: Vec::new(),
            requires_review: None,
            created_at: now,
            updated_at: now,
            approach: String::new(),
            tradeoffs: String::new(),
        }
    }

    /// Add a critique to this solution
    pub fn add_critique(&mut self, critique_id: String) {
        if !self.critique_ids.contains(&critique_id) {
            self.critique_ids.push(critique_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a critique from this solution
    pub fn remove_critique(&mut self, critique_id: &str) -> bool {
        if let Some(pos) = self.critique_ids.iter().position(|id| id == critique_id) {
            self.critique_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Attach a jj change to this solution
    pub fn attach_change(&mut self, change_id: String) {
        if !self.change_ids.contains(&change_id) {
            self.change_ids.push(change_id);
            self.updated_at = Utc::now();
        }
    }

    /// Detach a jj change from this solution
    pub fn detach_change(&mut self, change_id: &str) -> bool {
        if let Some(pos) = self.change_ids.iter().position(|id| id == change_id) {
            self.change_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update status
    pub fn set_status(&mut self, status: SolutionStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if self.tags.insert(tag) {
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if self.tags.remove(tag) {
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Check if solution is active (can be worked on)
    pub fn is_active(&self) -> bool {
        matches!(self.status, SolutionStatus::Proposed | SolutionStatus::Testing)
    }

    /// Check if solution is finalized (accepted or refuted)
    pub fn is_finalized(&self) -> bool {
        matches!(self.status, SolutionStatus::Accepted | SolutionStatus::Refuted)
    }

    /// Check if solution is in proposed status
    pub fn is_proposed(&self) -> bool {
        self.status == SolutionStatus::Proposed
    }

    /// Check if solution is in testing status
    pub fn is_testing(&self) -> bool {
        self.status == SolutionStatus::Testing
    }

    /// Check if solution has been accepted
    pub fn is_accepted(&self) -> bool {
        self.status == SolutionStatus::Accepted
    }

    /// Check if solution has been refuted
    pub fn is_refuted(&self) -> bool {
        self.status == SolutionStatus::Refuted
    }

    /// Mark solution as being tested
    pub fn start_testing(&mut self) {
        self.set_status(SolutionStatus::Testing);
    }

    /// Accept the solution (survived criticism)
    pub fn accept(&mut self) {
        self.set_status(SolutionStatus::Accepted);
    }

    /// Refute the solution (criticism showed it won't work)
    pub fn refute(&mut self) {
        self.set_status(SolutionStatus::Refuted);
    }

    /// Request review from someone
    pub fn request_review(&mut self, reviewer: String) {
        if !self.requested_reviewers.contains(&reviewer) {
            self.requested_reviewers.push(reviewer);
            self.updated_at = Utc::now();
        }
    }

    /// Record an LGTM from someone
    pub fn add_lgtm(&mut self, reviewer: String) {
        if !self.reviewed_by.contains(&reviewer) {
            self.reviewed_by.push(reviewer);
            self.updated_at = Utc::now();
        }
    }

    /// Check if any requested reviewer has LGTM'd
    pub fn has_lgtm_from_requested_reviewer(&self) -> bool {
        self.reviewed_by.iter().any(|r| self.requested_reviewers.contains(r))
    }

    /// Set the requires_review override
    pub fn set_requires_review(&mut self, requires: Option<bool>) {
        self.requires_review = requires;
        self.updated_at = Utc::now();
    }
}

/// YAML frontmatter for Solution markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionFrontmatter {
    pub id: String,
    pub title: String,
    pub problem_id: String,
    pub status: SolutionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub critique_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub change_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub tags: HashSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_reviewers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Solution> for SolutionFrontmatter {
    fn from(s: &Solution) -> Self {
        Self {
            id: s.id.clone(),
            title: s.title.clone(),
            problem_id: s.problem_id.clone(),
            status: s.status.clone(),
            critique_ids: s.critique_ids.clone(),
            change_ids: s.change_ids.clone(),
            tags: s.tags.clone(),
            assignee: s.assignee.clone(),
            requested_reviewers: s.requested_reviewers.clone(),
            reviewed_by: s.reviewed_by.clone(),
            requires_review: s.requires_review,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_solution() {
        let solution = Solution::new(
            "S-1".to_string(),
            "Use JWT tokens".to_string(),
            "P-1".to_string(),
        );
        assert_eq!(solution.id, "S-1");
        assert_eq!(solution.title, "Use JWT tokens");
        assert_eq!(solution.problem_id, "P-1");
        assert_eq!(solution.status, SolutionStatus::Proposed);
        assert!(solution.is_active());
    }

    #[test]
    fn test_status_transitions() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        assert_eq!(solution.status, SolutionStatus::Proposed);
        assert!(solution.is_active());

        solution.start_testing();
        assert_eq!(solution.status, SolutionStatus::Testing);
        assert!(solution.is_active());

        solution.accept();
        assert_eq!(solution.status, SolutionStatus::Accepted);
        assert!(solution.is_finalized());
    }

    #[test]
    fn test_refutation() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        solution.start_testing();
        solution.refute();

        assert_eq!(solution.status, SolutionStatus::Refuted);
        assert!(solution.is_finalized());
    }

    #[test]
    fn test_attach_change() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        solution.attach_change("kpqxywon".to_string());
        solution.attach_change("zmxptlnw".to_string());

        assert_eq!(solution.change_ids.len(), 2);
        assert!(solution.change_ids.contains(&"kpqxywon".to_string()));
    }

    #[test]
    fn test_add_critique() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        solution.add_critique("CQ-1".to_string());
        solution.add_critique("CQ-2".to_string());

        assert_eq!(solution.critique_ids.len(), 2);
        assert!(solution.critique_ids.contains(&"CQ-1".to_string()));
    }

    #[test]
    fn test_status_parsing() {
        assert_eq!("proposed".parse::<SolutionStatus>().unwrap(), SolutionStatus::Proposed);
        assert_eq!("testing".parse::<SolutionStatus>().unwrap(), SolutionStatus::Testing);
        assert_eq!("refuted".parse::<SolutionStatus>().unwrap(), SolutionStatus::Refuted);
        assert_eq!("accepted".parse::<SolutionStatus>().unwrap(), SolutionStatus::Accepted);
    }

    #[test]
    fn test_request_reviewers() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        solution.request_review("alice".to_string());
        solution.request_review("bob".to_string());

        assert_eq!(solution.requested_reviewers.len(), 2);
        assert!(solution.requested_reviewers.contains(&"alice".to_string()));
    }

    #[test]
    fn test_lgtm() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        solution.request_review("alice".to_string());
        solution.add_lgtm("alice".to_string());

        assert!(solution.reviewed_by.contains(&"alice".to_string()));
        assert!(solution.has_lgtm_from_requested_reviewer());
    }

    #[test]
    fn test_lgtm_from_non_requested() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        solution.request_review("alice".to_string());
        solution.add_lgtm("bob".to_string()); // Bob wasn't requested

        assert!(solution.reviewed_by.contains(&"bob".to_string()));
        assert!(!solution.has_lgtm_from_requested_reviewer());
    }

    #[test]
    fn test_requires_review_override() {
        let mut solution = Solution::new(
            "S-1".to_string(),
            "Test".to_string(),
            "P-1".to_string(),
        );

        assert!(solution.requires_review.is_none());

        solution.set_requires_review(Some(true));
        assert_eq!(solution.requires_review, Some(true));

        solution.set_requires_review(Some(false));
        assert_eq!(solution.requires_review, Some(false));
    }
}
