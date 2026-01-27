use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignOff {
    pub reviewer: String,
    pub at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

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

    /// Reviewers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewers: Vec<String>,

    /// Sign-offs from reviewers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sign_offs: Vec<SignOff>,

    /// Force-accepted without full sign-off
    #[serde(default)]
    pub force_accepted: bool,

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

    /// ID of the solution this one supersedes (for lineage tracking)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
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
            reviewers: Vec::new(),
            sign_offs: Vec::new(),
            force_accepted: false,
            created_at: now,
            updated_at: now,
            approach: String::new(),
            tradeoffs: String::new(),
            supersedes: None,
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

    /// Add a reviewer
    pub fn add_reviewer(&mut self, reviewer: String) {
        if !self.reviewers.contains(&reviewer) {
            self.reviewers.push(reviewer);
            self.updated_at = Utc::now();
        }
    }

    /// Add a sign-off from a reviewer
    pub fn add_sign_off(&mut self, reviewer: String, comment: Option<String>) {
        if !self.sign_offs.iter().any(|so| so.reviewer == reviewer) {
            self.sign_offs.push(SignOff {
                reviewer,
                at: Utc::now(),
                comment,
            });
            self.updated_at = Utc::now();
        }
    }

    /// Check if all requested reviewers have signed off
    pub fn all_reviewers_signed_off(&self) -> bool {
        if self.reviewers.is_empty() {
            return true;
        }
        self.reviewers.iter().all(|r| self.sign_offs.iter().any(|so| &so.reviewer == r))
    }

    /// Get reviewers who haven't signed off yet
    pub fn pending_reviewers(&self) -> Vec<&str> {
        self.reviewers.iter()
            .filter(|r| !self.sign_offs.iter().any(|so| &so.reviewer == *r))
            .map(|r| r.as_str())
            .collect()
    }

    /// Check if this solution requires review (derived from having reviewers)
    pub fn requires_review(&self) -> bool {
        !self.reviewers.is_empty()
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
    pub reviewers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sign_offs: Vec<SignOff>,
    #[serde(default)]
    pub force_accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
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
            reviewers: s.reviewers.clone(),
            sign_offs: s.sign_offs.clone(),
            force_accepted: s.force_accepted,
            supersedes: s.supersedes.clone(),
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
    fn test_add_reviewer() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
        solution.add_reviewer("alice".to_string());
        solution.add_reviewer("bob".to_string());
        solution.add_reviewer("alice".to_string()); // duplicate
        assert_eq!(solution.reviewers.len(), 2);
        assert!(solution.reviewers.contains(&"alice".to_string()));
    }

    #[test]
    fn test_add_sign_off() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
        solution.add_sign_off("alice".to_string(), Some("looks good".to_string()));
        assert_eq!(solution.sign_offs.len(), 1);
        assert_eq!(solution.sign_offs[0].reviewer, "alice");
        assert_eq!(solution.sign_offs[0].comment.as_deref(), Some("looks good"));
    }

    #[test]
    fn test_all_reviewers_signed_off() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
        solution.add_reviewer("alice".to_string());
        solution.add_reviewer("bob".to_string());
        assert!(!solution.all_reviewers_signed_off());
        solution.add_sign_off("alice".to_string(), None);
        assert!(!solution.all_reviewers_signed_off());
        solution.add_sign_off("bob".to_string(), None);
        assert!(solution.all_reviewers_signed_off());
    }

    #[test]
    fn test_pending_reviewers() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
        solution.add_reviewer("alice".to_string());
        solution.add_reviewer("bob".to_string());
        solution.add_sign_off("alice".to_string(), None);
        let pending = solution.pending_reviewers();
        assert_eq!(pending, vec!["bob"]);
    }

    #[test]
    fn test_requires_review_derived() {
        let solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
        assert!(!solution.requires_review());
        let mut solution2 = solution.clone();
        solution2.add_reviewer("alice".to_string());
        assert!(solution2.requires_review());
    }

    #[test]
    fn test_solution_supersedes() {
        let s = Solution::new(
            "S-2".to_string(),
            "Better approach".to_string(),
            "P-1".to_string(),
        );
        assert_eq!(s.supersedes, None);

        let mut s2 = s.clone();
        s2.supersedes = Some("S-1".to_string());
        assert_eq!(s2.supersedes.as_deref(), Some("S-1"));
    }

}
