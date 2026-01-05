use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A task in the Kanban board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier (e.g., "T-1024")
    pub id: String,

    /// Task title
    pub title: String,

    /// Parent feature (required - every task belongs to a feature)
    pub feature_id: String,

    /// Current column (e.g., "TODO", "In Progress", "Done")
    pub column: String,

    /// Tags for categorization (e.g., "backend", "frontend")
    #[serde(default)]
    pub tags: HashSet<String>,

    /// Assigned user (e.g., "@james")
    pub assignee: Option<String>,

    /// Associated change IDs (changes attached to this task)
    #[serde(default)]
    pub change_ids: Vec<String>,

    /// Number of comments
    #[serde(default)]
    pub comment_count: usize,

    /// Task description
    pub description: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Task metadata version (for conflict detection)
    #[serde(default)]
    pub version: u64,
}

impl Task {
    /// Create a new task with the given ID and title
    pub fn new(id: String, title: String, feature_id: String, column: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            feature_id,
            column,
            tags: HashSet::new(),
            assignee: None,
            change_ids: Vec::new(),
            comment_count: 0,
            description: None,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Add a tag to the task
    pub fn add_tag(&mut self, tag: String) {
        self.tags.insert(tag);
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Remove a tag from the task
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let removed = self.tags.remove(tag);
        if removed {
            self.updated_at = Utc::now();
            self.version += 1;
        }
        removed
    }

    /// Attach a change ID to this task
    pub fn attach_change(&mut self, change_id: String) {
        if !self.change_ids.contains(&change_id) {
            self.change_ids.push(change_id);
            self.updated_at = Utc::now();
            self.version += 1;
        }
    }

    /// Detach a change ID from this task
    pub fn detach_change(&mut self, change_id: &str) -> bool {
        if let Some(pos) = self.change_ids.iter().position(|id| id == change_id) {
            self.change_ids.remove(pos);
            self.updated_at = Utc::now();
            self.version += 1;
            true
        } else {
            false
        }
    }

    /// Move task to a different column
    pub fn move_to_column(&mut self, column: String) {
        self.column = column;
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

/// Task filter criteria
#[derive(Debug, Default)]
pub struct TaskFilter {
    pub column: Option<String>,
    pub tag: Option<String>,
    pub assignee: Option<String>,
}

impl TaskFilter {
    pub fn matches(&self, task: &Task) -> bool {
        if let Some(ref column) = self.column {
            if &task.column != column {
                return false;
            }
        }

        if let Some(ref tag) = self.tag {
            if !task.tags.contains(tag) {
                return false;
            }
        }

        if let Some(ref assignee) = self.assignee {
            match &task.assignee {
                Some(task_assignee) => {
                    if task_assignee != assignee {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }
}
