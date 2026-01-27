use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A milestone represents a cycle when we expect to have reasonable solutions
/// for a set of problems. It's a temporal target for problem resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Unique milestone identifier (e.g., "M-1")
    pub id: String,

    /// Milestone title (e.g., "v1.0 Release")
    pub title: String,

    /// Target completion date
    pub target_date: Option<DateTime<Utc>>,

    /// Current status
    pub status: MilestoneStatus,

    /// Problems we aim to solve this cycle
    #[serde(default)]
    pub problem_ids: Vec<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Assigned owner/lead
    pub assignee: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Goals - what we hope to achieve (markdown body)
    #[serde(default)]
    pub goals: String,

    /// Success criteria - how we'll know it's complete
    #[serde(default)]
    pub success_criteria: String,
}

/// Status of a milestone
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    /// Planning phase
    Planning,

    /// Active development
    Active,

    /// Completed
    Completed,

    /// Cancelled/abandoned
    Cancelled,
}

impl Default for MilestoneStatus {
    fn default() -> Self {
        MilestoneStatus::Planning
    }
}

impl std::fmt::Display for MilestoneStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MilestoneStatus::Planning => write!(f, "planning"),
            MilestoneStatus::Active => write!(f, "active"),
            MilestoneStatus::Completed => write!(f, "completed"),
            MilestoneStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for MilestoneStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "planning" => Ok(MilestoneStatus::Planning),
            "active" => Ok(MilestoneStatus::Active),
            "completed" => Ok(MilestoneStatus::Completed),
            "cancelled" => Ok(MilestoneStatus::Cancelled),
            _ => Err(format!("Unknown milestone status: {}", s)),
        }
    }
}

impl Milestone {
    /// Create a new milestone
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            target_date: None,
            status: MilestoneStatus::Planning,
            problem_ids: Vec::new(),
            tags: HashSet::new(),
            assignee: None,
            created_at: now,
            updated_at: now,
            goals: String::new(),
            success_criteria: String::new(),
        }
    }

    /// Add a problem to this milestone
    pub fn add_problem(&mut self, problem_id: String) {
        if !self.problem_ids.contains(&problem_id) {
            self.problem_ids.push(problem_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a problem from this milestone
    pub fn remove_problem(&mut self, problem_id: &str) -> bool {
        if let Some(pos) = self.problem_ids.iter().position(|id| id == problem_id) {
            self.problem_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Set the target date
    pub fn set_target_date(&mut self, date: Option<DateTime<Utc>>) {
        self.target_date = date;
        self.updated_at = Utc::now();
    }

    /// Update status
    pub fn set_status(&mut self, status: MilestoneStatus) {
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

    /// Check if milestone is overdue
    pub fn is_overdue(&self) -> bool {
        if let Some(target) = self.target_date {
            target < Utc::now() && self.status != MilestoneStatus::Completed
        } else {
            false
        }
    }

    /// Get days until target date (negative if overdue)
    pub fn days_until_target(&self) -> Option<i64> {
        self.target_date.map(|target| (target - Utc::now()).num_days())
    }

    /// Check if milestone is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, MilestoneStatus::Planning | MilestoneStatus::Active)
    }
}

/// YAML frontmatter for Milestone markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneFrontmatter {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<DateTime<Utc>>,
    pub status: MilestoneStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub problem_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub tags: HashSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Milestone> for MilestoneFrontmatter {
    fn from(m: &Milestone) -> Self {
        Self {
            id: m.id.clone(),
            title: m.title.clone(),
            target_date: m.target_date,
            status: m.status.clone(),
            problem_ids: m.problem_ids.clone(),
            tags: m.tags.clone(),
            assignee: m.assignee.clone(),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_milestone() {
        let milestone = Milestone::new("M-1".to_string(), "v1.0 Release".to_string());
        assert_eq!(milestone.id, "M-1");
        assert_eq!(milestone.title, "v1.0 Release");
        assert_eq!(milestone.status, MilestoneStatus::Planning);
        assert!(milestone.problem_ids.is_empty());
    }

    #[test]
    fn test_add_problem() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        milestone.add_problem("P-1".to_string());
        milestone.add_problem("P-2".to_string());

        assert_eq!(milestone.problem_ids.len(), 2);
        assert!(milestone.problem_ids.contains(&"P-1".to_string()));
    }

    #[test]
    fn test_remove_problem() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        milestone.add_problem("P-1".to_string());
        milestone.add_problem("P-2".to_string());

        let removed = milestone.remove_problem("P-1");
        assert!(removed);
        assert_eq!(milestone.problem_ids.len(), 1);
        assert!(!milestone.problem_ids.contains(&"P-1".to_string()));
    }

    #[test]
    fn test_add_duplicate_problem() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        milestone.add_problem("P-1".to_string());
        milestone.add_problem("P-1".to_string());

        assert_eq!(milestone.problem_ids.len(), 1);
    }

    #[test]
    fn test_status_transitions() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        assert_eq!(milestone.status, MilestoneStatus::Planning);
        assert!(milestone.is_active());

        milestone.set_status(MilestoneStatus::Active);
        assert_eq!(milestone.status, MilestoneStatus::Active);
        assert!(milestone.is_active());

        milestone.set_status(MilestoneStatus::Completed);
        assert_eq!(milestone.status, MilestoneStatus::Completed);
        assert!(!milestone.is_active());
    }

    #[test]
    fn test_status_parsing() {
        assert_eq!("planning".parse::<MilestoneStatus>().unwrap(), MilestoneStatus::Planning);
        assert_eq!("active".parse::<MilestoneStatus>().unwrap(), MilestoneStatus::Active);
        assert_eq!("completed".parse::<MilestoneStatus>().unwrap(), MilestoneStatus::Completed);
        assert_eq!("cancelled".parse::<MilestoneStatus>().unwrap(), MilestoneStatus::Cancelled);
    }
}
