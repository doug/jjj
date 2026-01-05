use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project-wide configuration stored in config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    #[serde(default)]
    pub name: Option<String>,

    /// Column definitions for the Kanban board
    #[serde(default = "default_columns")]
    pub columns: Vec<String>,

    /// Valid tags for tasks
    #[serde(default)]
    pub tags: Vec<String>,

    /// Default reviewers
    #[serde(default)]
    pub default_reviewers: Vec<String>,

    /// Custom settings
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

fn default_columns() -> Vec<String> {
    vec![
        "TODO".to_string(),
        "In Progress".to_string(),
        "Review".to_string(),
        "Done".to_string(),
    ]
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: None,
            columns: default_columns(),
            tags: Vec::new(),
            default_reviewers: Vec::new(),
            settings: HashMap::new(),
        }
    }
}

impl ProjectConfig {
    /// Check if a column name is valid
    pub fn is_valid_column(&self, column: &str) -> bool {
        self.columns.iter().any(|c| c == column)
    }

    /// Add a new column
    pub fn add_column(&mut self, column: String) {
        if !self.columns.contains(&column) {
            self.columns.push(column);
        }
    }

    /// Remove a column
    pub fn remove_column(&mut self, column: &str) -> bool {
        if let Some(pos) = self.columns.iter().position(|c| c == column) {
            self.columns.remove(pos);
            true
        } else {
            false
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
}
