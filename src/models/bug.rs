use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A bug represents a defect or issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bug {
    /// Unique bug identifier (e.g., "B-1")
    pub id: String,

    /// Bug title (issue description)
    pub title: String,

    /// Detailed description including reproduction steps
    pub description: Option<String>,

    /// Severity level
    pub severity: Severity,

    /// Current status
    pub status: BugStatus,

    /// Related feature (optional)
    pub feature_id: Option<String>,

    /// Target milestone for fix (optional)
    pub milestone_id: Option<String>,

    /// Person assigned to fix the bug
    pub assignee: Option<String>,

    /// Person who reported the bug
    pub reporter: Option<String>,

    /// Attached change IDs for the fix
    #[serde(default)]
    pub change_ids: Vec<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tag_ids: HashSet<String>,

    /// Version where bug was found
    pub affected_version: Option<String>,

    /// Version where bug was fixed
    pub fixed_version: Option<String>,

    /// Steps to reproduce
    pub repro_steps: Option<String>,

    /// Expected behavior
    pub expected_behavior: Option<String>,

    /// Actual behavior
    pub actual_behavior: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Severity level for bugs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Minor issue, cosmetic
    Low,

    /// Feature impaired, workaround exists
    Medium,

    /// Major functionality broken
    High,

    /// System down, data loss, security issue
    Critical,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Medium
    }
}

/// Status of a bug
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BugStatus {
    /// Newly reported, not triaged
    New,

    /// Confirmed as valid bug
    Confirmed,

    /// Work in progress
    InProgress,

    /// Fix implemented
    Fixed,

    /// Fix verified and closed
    Closed,

    /// Will not fix
    WontFix,

    /// Duplicate of another bug
    Duplicate,
}

impl Bug {
    /// Create a new bug
    pub fn new(id: String, title: String, severity: Severity) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description: None,
            severity,
            status: BugStatus::New,
            feature_id: None,
            milestone_id: None,
            assignee: None,
            reporter: None,
            change_ids: Vec::new(),
            tag_ids: HashSet::new(),
            affected_version: None,
            fixed_version: None,
            repro_steps: None,
            expected_behavior: None,
            actual_behavior: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the related feature
    pub fn set_feature(&mut self, feature_id: Option<String>) {
        self.feature_id = feature_id;
        self.updated_at = Utc::now();
    }

    /// Set the target milestone
    pub fn set_milestone(&mut self, milestone_id: Option<String>) {
        self.milestone_id = milestone_id;
        self.updated_at = Utc::now();
    }

    /// Update status
    pub fn set_status(&mut self, status: BugStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Update severity
    pub fn set_severity(&mut self, severity: Severity) {
        self.severity = severity;
        self.updated_at = Utc::now();
    }

    /// Attach a change ID for the fix
    pub fn attach_change(&mut self, change_id: String) {
        if !self.change_ids.contains(&change_id) {
            self.change_ids.push(change_id);
            self.updated_at = Utc::now();
        }
    }

    /// Detach a change ID
    pub fn detach_change(&mut self, change_id: &str) -> bool {
        if let Some(pos) = self.change_ids.iter().position(|id| id == change_id) {
            self.change_ids.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
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

    /// Check if bug is open
    pub fn is_open(&self) -> bool {
        matches!(
            self.status,
            BugStatus::New | BugStatus::Confirmed | BugStatus::InProgress
        )
    }

    /// Check if bug is resolved
    pub fn is_resolved(&self) -> bool {
        matches!(
            self.status,
            BugStatus::Fixed | BugStatus::Closed | BugStatus::WontFix | BugStatus::Duplicate
        )
    }

    /// Mark as fixed in a version
    pub fn mark_fixed(&mut self, version: String) {
        self.fixed_version = Some(version);
        self.set_status(BugStatus::Fixed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bug() {
        let bug = Bug::new(
            "B-1".to_string(),
            "Login fails".to_string(),
            Severity::High,
        );
        assert_eq!(bug.id, "B-1");
        assert_eq!(bug.title, "Login fails");
        assert_eq!(bug.severity, Severity::High);
        assert_eq!(bug.status, BugStatus::New);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn test_bug_status_transitions() {
        let mut bug = Bug::new("B-1".to_string(), "Test bug".to_string(), Severity::Medium);

        assert_eq!(bug.status, BugStatus::New);

        bug.set_status(BugStatus::Confirmed);
        assert_eq!(bug.status, BugStatus::Confirmed);

        bug.set_status(BugStatus::InProgress);
        assert_eq!(bug.status, BugStatus::InProgress);

        bug.set_status(BugStatus::Fixed);
        assert_eq!(bug.status, BugStatus::Fixed);

        bug.set_status(BugStatus::Closed);
        assert_eq!(bug.status, BugStatus::Closed);
    }

    #[test]
    fn test_bug_is_open() {
        let mut bug = Bug::new("B-1".to_string(), "Test".to_string(), Severity::Low);

        assert!(bug.is_open());
        assert!(!bug.is_resolved());

        bug.set_status(BugStatus::Confirmed);
        assert!(bug.is_open());

        bug.set_status(BugStatus::Fixed);
        assert!(!bug.is_open());
        assert!(bug.is_resolved());
    }

    #[test]
    fn test_link_to_feature() {
        let mut bug = Bug::new("B-1".to_string(), "Test".to_string(), Severity::Medium);
        assert!(bug.feature_id.is_none());

        bug.set_feature(Some("F-1".to_string()));
        assert_eq!(bug.feature_id, Some("F-1".to_string()));
    }

    #[test]
    fn test_link_to_milestone() {
        let mut bug = Bug::new("B-1".to_string(), "Test".to_string(), Severity::High);
        assert!(bug.milestone_id.is_none());

        bug.set_milestone(Some("M-1".to_string()));
        assert_eq!(bug.milestone_id, Some("M-1".to_string()));
    }

    #[test]
    fn test_attach_change() {
        let mut bug = Bug::new("B-1".to_string(), "Test".to_string(), Severity::Critical);
        bug.attach_change("kpqxywon".to_string());
        bug.attach_change("zmxptlnw".to_string());

        assert_eq!(bug.change_ids.len(), 2);
        assert!(bug.change_ids.contains(&"kpqxywon".to_string()));
    }

    #[test]
    fn test_mark_fixed() {
        let mut bug = Bug::new("B-1".to_string(), "Test".to_string(), Severity::Medium);

        bug.mark_fixed("1.0.0".to_string());

        assert_eq!(bug.fixed_version, Some("1.0.0".to_string()));
        assert_eq!(bug.status, BugStatus::Fixed);
    }

    #[test]
    fn test_bug_serialization() {
        let mut bug = Bug::new("B-1".to_string(), "Login fails".to_string(), Severity::Critical);
        bug.set_feature(Some("F-1".to_string()));
        bug.attach_change("abc123".to_string());
        bug.add_tag("tag-1".to_string());

        let json = serde_json::to_string_pretty(&bug).expect("Failed to serialize");
        let deserialized: Bug = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.id, bug.id);
        assert_eq!(deserialized.severity, Severity::Critical);
        assert_eq!(deserialized.feature_id, Some("F-1".to_string()));
        assert_eq!(deserialized.change_ids.len(), 1);
        assert_eq!(deserialized.tag_ids.len(), 1);
    }
}
