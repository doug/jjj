use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A feature represents a user-facing capability or improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// Unique feature identifier (e.g., "F-1")
    pub id: String,

    /// Feature title (e.g., "User Authentication")
    pub title: String,

    /// Description (ideally in user story format)
    pub description: Option<String>,

    /// Parent milestone (optional)
    pub milestone_id: Option<String>,

    /// Current status
    pub status: FeatureStatus,

    /// Assigned owner/lead
    pub assignee: Option<String>,

    /// Child tasks
    #[serde(default)]
    pub task_ids: Vec<String>,

    /// Related bugs
    #[serde(default)]
    pub bug_ids: Vec<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Priority level
    pub priority: Priority,

    /// Story points or size estimate (optional)
    pub story_points: Option<u32>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Status of a feature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeatureStatus {
    /// In backlog, not started
    Backlog,

    /// Active development
    InProgress,

    /// In review
    Review,

    /// Completed
    Done,

    /// Blocked by dependencies
    Blocked,
}

/// Priority level for features
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
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

impl Feature {
    /// Create a new feature
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description: None,
            milestone_id: None,
            status: FeatureStatus::Backlog,
            assignee: None,
            task_ids: Vec::new(),
            bug_ids: Vec::new(),
            tags: HashSet::new(),
            priority: Priority::default(),
            story_points: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a task to this feature
    pub fn add_task(&mut self, task_id: String) {
        if !self.task_ids.contains(&task_id) {
            self.task_ids.push(task_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a task from this feature
    pub fn remove_task(&mut self, task_id: &str) -> bool {
        if let Some(pos) = self.task_ids.iter().position(|id| id == task_id) {
            self.task_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Add a related bug
    pub fn add_bug(&mut self, bug_id: String) {
        if !self.bug_ids.contains(&bug_id) {
            self.bug_ids.push(bug_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a related bug
    pub fn remove_bug(&mut self, bug_id: &str) -> bool {
        if let Some(pos) = self.bug_ids.iter().position(|id| id == bug_id) {
            self.bug_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
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

    /// Set milestone
    pub fn set_milestone(&mut self, milestone_id: Option<String>) {
        self.milestone_id = milestone_id;
        self.updated_at = Utc::now();
    }

    /// Update status
    pub fn set_status(&mut self, status: FeatureStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Set priority
    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
        self.updated_at = Utc::now();
    }

    /// Calculate progress based on task completion
    /// Returns (completed_tasks, total_tasks)
    pub fn task_progress(&self) -> (usize, usize) {
        // This would need actual task data to calculate
        // For now, just return the count
        (0, self.task_ids.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_feature() {
        let feature = Feature::new("F-1".to_string(), "User Authentication".to_string());
        assert_eq!(feature.id, "F-1");
        assert_eq!(feature.title, "User Authentication");
        assert_eq!(feature.status, FeatureStatus::Backlog);
        assert_eq!(feature.priority, Priority::Medium);
    }

    #[test]
    fn test_add_task() {
        let mut feature = Feature::new("F-1".to_string(), "Auth".to_string());
        feature.add_task("T-1".to_string());
        feature.add_task("T-2".to_string());

        assert_eq!(feature.task_ids.len(), 2);
        assert!(feature.task_ids.contains(&"T-1".to_string()));
    }

    #[test]
    fn test_add_duplicate_task() {
        let mut feature = Feature::new("F-1".to_string(), "Auth".to_string());
        feature.add_task("T-1".to_string());
        feature.add_task("T-1".to_string());

        assert_eq!(feature.task_ids.len(), 1);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn test_status_transitions() {
        let mut feature = Feature::new("F-1".to_string(), "Auth".to_string());
        assert_eq!(feature.status, FeatureStatus::Backlog);

        feature.set_status(FeatureStatus::InProgress);
        assert_eq!(feature.status, FeatureStatus::InProgress);

        feature.set_status(FeatureStatus::Review);
        assert_eq!(feature.status, FeatureStatus::Review);

        feature.set_status(FeatureStatus::Done);
        assert_eq!(feature.status, FeatureStatus::Done);
    }

    #[test]
    fn test_milestone_assignment() {
        let mut feature = Feature::new("F-1".to_string(), "Auth".to_string());
        assert!(feature.milestone_id.is_none());

        feature.set_milestone(Some("M-1".to_string()));
        assert_eq!(feature.milestone_id, Some("M-1".to_string()));

        feature.set_milestone(None);
        assert!(feature.milestone_id.is_none());
    }

    #[test]
    fn test_feature_serialization() {
        let mut feature = Feature::new("F-1".to_string(), "User Auth".to_string());
        feature.set_priority(Priority::High);
        feature.add_task("T-1".to_string());
        feature.add_tag("backend".to_string());

        let json = serde_json::to_string_pretty(&feature).expect("Failed to serialize");
        let deserialized: Feature = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.id, feature.id);
        assert_eq!(deserialized.priority, Priority::High);
        assert_eq!(deserialized.task_ids.len(), 1);
        assert_eq!(deserialized.tags.len(), 1);
    }
}
