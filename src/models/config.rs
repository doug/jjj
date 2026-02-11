use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project-wide configuration stored in config.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    #[serde(default)]
    pub name: Option<String>,

    /// Default reviewers
    #[serde(default)]
    pub default_reviewers: Vec<String>,

    /// Custom settings
    #[serde(default)]
    pub settings: HashMap<String, String>,
}
