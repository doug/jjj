use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use std::collections::HashSet;

/// A milestone represents a release, sprint, or delivery target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Unique milestone identifier (e.g., "M-1")
    pub id: String,

    /// Milestone title (e.g., "v1.0 Release")
    pub title: String,

    /// Description of what this milestone delivers
    pub description: Option<String>,

    /// Target completion date
    pub target_date: Option<DateTime<Utc>>,

    /// Current status
    pub status: MilestoneStatus,

    /// Features included in this milestone
    #[serde(default)]
    pub feature_ids: Vec<String>,

    /// Bugs targeted for this milestone
    #[serde(default)]
    pub bug_ids: Vec<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tag_ids: HashSet<String>,

    /// Version number (optional, e.g., "1.0.0")
    pub version: Option<String>,

    /// Assigned owner/lead
    pub assignee: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Status of a milestone
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MilestoneStatus {
    /// Planning phase
    Planning,

    /// Active development
    Active,

    /// Released/completed
    Released,

    /// Cancelled/abandoned
    Cancelled,
}

impl Milestone {
    /// Create a new milestone
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description: None,
            target_date: None,
            status: MilestoneStatus::Planning,
            feature_ids: Vec::new(),
            bug_ids: Vec::new(),
            tag_ids: HashSet::new(),
            version: None,
            assignee: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a feature to this milestone
    pub fn add_feature(&mut self, feature_id: String) {
        if !self.feature_ids.contains(&feature_id) {
            self.feature_ids.push(feature_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a feature from this milestone
    pub fn remove_feature(&mut self, feature_id: &str) -> bool {
        if let Some(pos) = self.feature_ids.iter().position(|id| id == feature_id) {
            self.feature_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Add a bug to this milestone
    pub fn add_bug(&mut self, bug_id: String) {
        if !self.bug_ids.contains(&bug_id) {
            self.bug_ids.push(bug_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a bug from this milestone
    pub fn remove_bug(&mut self, bug_id: &str) -> bool {
        if let Some(pos) = self.bug_ids.iter().position(|id| id == bug_id) {
            self.bug_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Set the target date
    pub fn set_target_date(&mut self, date: DateTime<Utc>) {
        self.target_date = Some(date);
        self.updated_at = Utc::now();
    }

    /// Update status
    pub fn set_status(&mut self, status: MilestoneStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }


    /// Check if milestone is overdue
    pub fn is_overdue(&self) -> bool {
        if let Some(target) = self.target_date {
            target < Utc::now() && self.status != MilestoneStatus::Released
        } else {
            false
        }
    }

    /// Get days until target date
    pub fn days_until_target(&self) -> Option<i64> {
        self.target_date.map(|target| {
            (target - Utc::now()).num_days()
        })
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag_id: String) {
        if self.tag_ids.insert(tag_id) {
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag_id: &str) -> bool {
        if self.tag_ids.remove(tag_id) {
            self.updated_at = Utc::now();
            true
        } else {
            false
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
        assert!(milestone.feature_ids.is_empty());
    }

    #[test]
    fn test_add_feature() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        milestone.add_feature("F-1".to_string());
        milestone.add_feature("F-2".to_string());

        assert_eq!(milestone.feature_ids.len(), 2);
        assert!(milestone.feature_ids.contains(&"F-1".to_string()));
    }

    #[test]
    fn test_remove_feature() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        milestone.add_feature("F-1".to_string());
        milestone.add_feature("F-2".to_string());

        let removed = milestone.remove_feature("F-1");
        assert!(removed);
        assert_eq!(milestone.feature_ids.len(), 1);
        assert!(!milestone.feature_ids.contains(&"F-1".to_string()));
    }

    #[test]
    fn test_add_duplicate_feature() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0".to_string());
        milestone.add_feature("F-1".to_string());
        milestone.add_feature("F-1".to_string());

        assert_eq!(milestone.feature_ids.len(), 1);
    }

    #[test]
    fn test_milestone_serialization() {
        let mut milestone = Milestone::new("M-1".to_string(), "v1.0 Release".to_string());
        milestone.version = Some("1.0.0".to_string());
        milestone.add_feature("F-1".to_string());

        let toml = toml::to_string(&milestone).expect("Failed to serialize");
        let deserialized: Milestone = toml::from_str(&toml).expect("Failed to deserialize");

        assert_eq!(deserialized.id, milestone.id);
        assert_eq!(deserialized.version, milestone.version);
        assert_eq!(deserialized.feature_ids.len(), 1);
    }
}
