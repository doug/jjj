use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A problem represents something that needs to be addressed.
/// Problems are fundamental - all knowledge begins with problems.
/// Problems can be decomposed into sub-problems, forming a DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    /// Unique problem identifier (e.g., "P-1")
    pub id: String,

    /// Problem title (what is the problem?)
    pub title: String,

    /// Parent problem ID (for sub-problems)
    pub parent_id: Option<String>,

    /// Current status
    pub status: ProblemStatus,

    /// Priority level
    pub priority: Priority,

    /// Solution IDs attempting to address this problem
    #[serde(default)]
    pub solution_ids: Vec<String>,

    /// Child problem IDs (sub-problems - computed from parent_id references)
    #[serde(default)]
    pub child_ids: Vec<String>,

    /// Target milestone (optional)
    pub milestone_id: Option<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Assigned owner
    pub assignee: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Description of the problem (markdown body)
    #[serde(default)]
    pub description: String,

    /// Context - why this is a problem, what makes it hard
    #[serde(default)]
    pub context: String,
}

/// Priority level for a problem (P0 = most critical, P3 = lowest)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "P3/low"),
            Priority::Medium => write!(f, "P2/medium"),
            Priority::High => write!(f, "P1/high"),
            Priority::Critical => write!(f, "P0/critical"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "p0" | "critical" => Ok(Priority::Critical),
            "p1" | "high" => Ok(Priority::High),
            "p2" | "medium" => Ok(Priority::Medium),
            "p3" | "low" => Ok(Priority::Low),
            _ => Err(format!(
                "Invalid priority: {}. Use P0/critical, P1/high, P2/medium, or P3/low",
                s
            )),
        }
    }
}

/// Status of a problem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProblemStatus {
    /// Problem identified, not yet being addressed
    Open,

    /// Actively working on solutions
    InProgress,

    /// A solution has been accepted
    Solved,

    /// Problem was based on false premises or became irrelevant
    Dissolved,
}

impl Default for ProblemStatus {
    fn default() -> Self {
        ProblemStatus::Open
    }
}

impl std::fmt::Display for ProblemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemStatus::Open => write!(f, "open"),
            ProblemStatus::InProgress => write!(f, "in_progress"),
            ProblemStatus::Solved => write!(f, "solved"),
            ProblemStatus::Dissolved => write!(f, "dissolved"),
        }
    }
}

impl std::str::FromStr for ProblemStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(ProblemStatus::Open),
            "in_progress" | "inprogress" => Ok(ProblemStatus::InProgress),
            "solved" => Ok(ProblemStatus::Solved),
            "dissolved" => Ok(ProblemStatus::Dissolved),
            _ => Err(format!("Unknown problem status: {}", s)),
        }
    }
}

impl Problem {
    /// Create a new problem
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            parent_id: None,
            status: ProblemStatus::Open,
            priority: Priority::default(),
            solution_ids: Vec::new(),
            child_ids: Vec::new(),
            milestone_id: None,
            tags: HashSet::new(),
            assignee: None,
            created_at: now,
            updated_at: now,
            description: String::new(),
            context: String::new(),
        }
    }

    /// Set parent problem (making this a sub-problem)
    pub fn set_parent(&mut self, parent_id: Option<String>) {
        self.parent_id = parent_id;
        self.updated_at = Utc::now();
    }

    /// Add a solution to this problem
    pub fn add_solution(&mut self, solution_id: String) {
        if !self.solution_ids.contains(&solution_id) {
            self.solution_ids.push(solution_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a solution from this problem
    pub fn remove_solution(&mut self, solution_id: &str) -> bool {
        if let Some(pos) = self.solution_ids.iter().position(|id| id == solution_id) {
            self.solution_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Add a child problem (sub-problem)
    pub fn add_child(&mut self, child_id: String) {
        if !self.child_ids.contains(&child_id) {
            self.child_ids.push(child_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a child problem
    pub fn remove_child(&mut self, child_id: &str) -> bool {
        if let Some(pos) = self.child_ids.iter().position(|id| id == child_id) {
            self.child_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Set milestone
    pub fn set_milestone(&mut self, milestone_id: Option<String>) {
        self.milestone_id = milestone_id;
        self.updated_at = Utc::now();
    }

    /// Update status
    pub fn set_status(&mut self, status: ProblemStatus) {
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

    /// Check if problem is open (can be worked on)
    pub fn is_open(&self) -> bool {
        matches!(self.status, ProblemStatus::Open | ProblemStatus::InProgress)
    }

    /// Check if problem is resolved (solved or dissolved)
    pub fn is_resolved(&self) -> bool {
        matches!(self.status, ProblemStatus::Solved | ProblemStatus::Dissolved)
    }

    /// Check if problem is in progress
    pub fn is_in_progress(&self) -> bool {
        self.status == ProblemStatus::InProgress
    }

    /// Check if this is a sub-problem (has a parent)
    pub fn is_subproblem(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Check if this is a root problem (no parent)
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// YAML frontmatter for Problem markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemFrontmatter {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub status: ProblemStatus,
    pub priority: Priority,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub solution_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub tags: HashSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Problem> for ProblemFrontmatter {
    fn from(p: &Problem) -> Self {
        Self {
            id: p.id.clone(),
            title: p.title.clone(),
            parent_id: p.parent_id.clone(),
            status: p.status.clone(),
            priority: p.priority.clone(),
            solution_ids: p.solution_ids.clone(),
            child_ids: p.child_ids.clone(),
            milestone_id: p.milestone_id.clone(),
            tags: p.tags.clone(),
            assignee: p.assignee.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_problem() {
        let problem = Problem::new("P-1".to_string(), "User auth is unreliable".to_string());
        assert_eq!(problem.id, "P-1");
        assert_eq!(problem.title, "User auth is unreliable");
        assert_eq!(problem.status, ProblemStatus::Open);
        assert!(problem.is_root());
        assert!(problem.is_open());
    }

    #[test]
    fn test_subproblem() {
        let mut problem = Problem::new("P-2".to_string(), "Token expiry handling".to_string());
        problem.set_parent(Some("P-1".to_string()));

        assert!(problem.is_subproblem());
        assert!(!problem.is_root());
        assert_eq!(problem.parent_id, Some("P-1".to_string()));
    }

    #[test]
    fn test_status_transitions() {
        let mut problem = Problem::new("P-1".to_string(), "Test".to_string());
        assert_eq!(problem.status, ProblemStatus::Open);
        assert!(problem.is_open());

        problem.set_status(ProblemStatus::InProgress);
        assert_eq!(problem.status, ProblemStatus::InProgress);
        assert!(problem.is_open());

        problem.set_status(ProblemStatus::Solved);
        assert_eq!(problem.status, ProblemStatus::Solved);
        assert!(problem.is_resolved());
    }

    #[test]
    fn test_add_solution() {
        let mut problem = Problem::new("P-1".to_string(), "Test".to_string());
        problem.add_solution("S-1".to_string());
        problem.add_solution("S-2".to_string());

        assert_eq!(problem.solution_ids.len(), 2);
        assert!(problem.solution_ids.contains(&"S-1".to_string()));
    }

    #[test]
    fn test_add_child() {
        let mut problem = Problem::new("P-1".to_string(), "Parent".to_string());
        problem.add_child("P-2".to_string());
        problem.add_child("P-3".to_string());

        assert_eq!(problem.child_ids.len(), 2);
        assert!(problem.child_ids.contains(&"P-2".to_string()));
    }

    #[test]
    fn test_status_parsing() {
        assert_eq!("open".parse::<ProblemStatus>().unwrap(), ProblemStatus::Open);
        assert_eq!("in_progress".parse::<ProblemStatus>().unwrap(), ProblemStatus::InProgress);
        assert_eq!("solved".parse::<ProblemStatus>().unwrap(), ProblemStatus::Solved);
        assert_eq!("dissolved".parse::<ProblemStatus>().unwrap(), ProblemStatus::Dissolved);
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!("P0".parse::<Priority>().unwrap(), Priority::Critical);
        assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Critical);
        assert_eq!("P1".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("P2".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("medium".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("P3".parse::<Priority>().unwrap(), Priority::Low);
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", Priority::Critical), "P0/critical");
        assert_eq!(format!("{}", Priority::High), "P1/high");
        assert_eq!(format!("{}", Priority::Medium), "P2/medium");
        assert_eq!(format!("{}", Priority::Low), "P3/low");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn test_problem_priority_default() {
        let p = Problem::new("P-1".to_string(), "Test".to_string());
        assert_eq!(p.priority, Priority::Medium);
    }
}
