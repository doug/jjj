use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tag definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    /// Unique tag identifier (e.g., "tag-1")
    pub id: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Color for UI (hex code or name)
    pub color: Option<String>,
}

/// Project-wide configuration stored in config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    #[serde(default)]
    pub name: Option<String>,

    /// Valid tags for entities
    #[serde(default)]
    pub tags: Vec<Tag>,

    /// Default reviewers
    #[serde(default)]
    pub default_reviewers: Vec<String>,

    /// Custom settings
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: None,
            tags: Vec::new(),
            default_reviewers: Vec::new(),
            settings: HashMap::new(),
        }
    }
}

impl ProjectConfig {
    /// Add a tag
    pub fn add_tag(&mut self, name: String, description: Option<String>, color: Option<String>) -> Tag {
        // Check if tag with name already exists
        if let Some(existing) = self.tags.iter().find(|t| t.name == name) {
            return existing.clone();
        }

        let id = self.next_tag_id();
        let tag = Tag {
            id,
            name,
            description,
            color,
        };
        self.tags.push(tag.clone());
        tag
    }

    /// Generate next tag ID
    fn next_tag_id(&self) -> String {
        let max_id = self
            .tags
            .iter()
            .filter_map(|t| t.id.strip_prefix("tag-").and_then(|s| s.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);
        format!("tag-{}", max_id + 1)
    }

    /// Get tag by ID
    pub fn get_tag(&self, id: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.id == id)
    }

    /// Get tag by name
    pub fn get_tag_by_name(&self, name: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.name == name)
    }

    /// Remove a tag by ID
    pub fn remove_tag(&mut self, id: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|t| t.id == id) {
            self.tags.remove(pos);
            true
        } else {
            false
        }
    }

    /// Edit a tag
    pub fn edit_tag(&mut self, id: &str, name: Option<String>, description: Option<String>, color: Option<String>) -> bool {
        if let Some(tag) = self.tags.iter_mut().find(|t| t.id == id) {
            if let Some(n) = name {
                tag.name = n;
            }
            if description.is_some() {
                tag.description = description;
            }
            if color.is_some() {
                tag.color = color;
            }
            true
        } else {
            false
        }
    }
}
