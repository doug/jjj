use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

    /// Assigned owner
    pub assignee: Option<String>,

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

    /// Linked GitHub PR number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_pr: Option<u64>,

    /// GitHub branch name for this solution's PR
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_branch: Option<String>,
}

/// Status of a solution (conjecture)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SolutionStatus {
    /// Conjecture put forward, not yet tested
    #[default]
    Proposed,

    /// Under review — ready for criticism
    Review,

    /// Criticism has shown this won't work
    Refuted,

    /// Survived criticism, adopted as current best solution
    Accepted,
}

impl std::fmt::Display for SolutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolutionStatus::Proposed => write!(f, "proposed"),
            SolutionStatus::Review => write!(f, "review"),
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
            "review" => Ok(SolutionStatus::Review),
            "refuted" => Ok(SolutionStatus::Refuted),
            "accepted" => Ok(SolutionStatus::Accepted),
            _ => Err(format!(
                "Unknown solution status: '{}'. Valid values: proposed, review, refuted, accepted",
                s
            )),
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
            assignee: None,
            force_accepted: false,
            created_at: now,
            updated_at: now,
            approach: String::new(),
            tradeoffs: String::new(),
            supersedes: None,
            github_pr: None,
            github_branch: None,
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

    /// Check if a status transition is valid.
    pub fn can_transition_to(&self, target: &SolutionStatus) -> bool {
        matches!(
            (&self.status, target),
            (SolutionStatus::Proposed, SolutionStatus::Review)
                | (SolutionStatus::Proposed, SolutionStatus::Refuted)
                | (SolutionStatus::Review, SolutionStatus::Accepted)
                | (SolutionStatus::Review, SolutionStatus::Refuted)
                | (SolutionStatus::Review, SolutionStatus::Proposed)
        )
    }

    /// Check if solution is active (can be worked on)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            SolutionStatus::Proposed | SolutionStatus::Review
        )
    }

    /// Check if solution is finalized (accepted or refuted)
    pub fn is_finalized(&self) -> bool {
        matches!(
            self.status,
            SolutionStatus::Accepted | SolutionStatus::Refuted
        )
    }

    /// Check if solution is in proposed status
    pub fn is_proposed(&self) -> bool {
        self.status == SolutionStatus::Proposed
    }

    /// Check if solution is in review status
    pub fn is_review(&self) -> bool {
        self.status == SolutionStatus::Review
    }

    /// Check if solution has been accepted
    pub fn is_accepted(&self) -> bool {
        self.status == SolutionStatus::Accepted
    }

    /// Check if solution has been refuted
    pub fn is_refuted(&self) -> bool {
        self.status == SolutionStatus::Refuted
    }

    /// Mark solution as under review
    pub fn start_review(&mut self) {
        self.set_status(SolutionStatus::Review);
    }

    /// Accept the solution (survived criticism)
    pub fn accept(&mut self) {
        self.set_status(SolutionStatus::Accepted);
    }

    /// Refute the solution (criticism showed it won't work)
    pub fn refute(&mut self) {
        self.set_status(SolutionStatus::Refuted);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default)]
    pub force_accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_pr: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_branch: Option<String>,
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
            assignee: s.assignee.clone(),
            force_accepted: s.force_accepted,
            supersedes: s.supersedes.clone(),
            github_pr: s.github_pr,
            github_branch: s.github_branch.clone(),
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
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());

        assert_eq!(solution.status, SolutionStatus::Proposed);
        assert!(solution.is_active());

        solution.start_review();
        assert_eq!(solution.status, SolutionStatus::Review);
        assert!(solution.is_active());

        solution.accept();
        assert_eq!(solution.status, SolutionStatus::Accepted);
        assert!(solution.is_finalized());
    }

    #[test]
    fn test_refutation() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());

        solution.start_review();
        solution.refute();

        assert_eq!(solution.status, SolutionStatus::Refuted);
        assert!(solution.is_finalized());
    }

    #[test]
    fn test_attach_change() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());

        solution.attach_change("kpqxywon".to_string());
        solution.attach_change("zmxptlnw".to_string());

        assert_eq!(solution.change_ids.len(), 2);
        assert!(solution.change_ids.contains(&"kpqxywon".to_string()));
    }

    #[test]
    fn test_add_critique() {
        let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());

        solution.add_critique("CQ-1".to_string());
        solution.add_critique("CQ-2".to_string());

        assert_eq!(solution.critique_ids.len(), 2);
        assert!(solution.critique_ids.contains(&"CQ-1".to_string()));
    }

    #[test]
    fn test_status_parsing() {
        assert_eq!(
            "proposed".parse::<SolutionStatus>().unwrap(),
            SolutionStatus::Proposed
        );
        assert_eq!(
            "review".parse::<SolutionStatus>().unwrap(),
            SolutionStatus::Review
        );
        assert_eq!(
            "refuted".parse::<SolutionStatus>().unwrap(),
            SolutionStatus::Refuted
        );
        assert_eq!(
            "accepted".parse::<SolutionStatus>().unwrap(),
            SolutionStatus::Accepted
        );
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
