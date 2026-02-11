//! Local configuration for jjj stored in .jj/jjj.toml
//!
//! This config is never synced - it's local to each machine.
//! Used for embedding service configuration and other local settings.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for the embedding service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    /// Explicitly enable embeddings (if true, warn on connection failure)
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Base URL for the embedding API (default: http://localhost:11434/v1)
    #[serde(default)]
    pub base_url: Option<String>,

    /// Model name (e.g., "qwen3-embedding:8b")
    #[serde(default)]
    pub model: Option<String>,

    /// Embedding dimensions
    #[serde(default)]
    pub dimensions: Option<usize>,

    /// API key for remote services
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Local configuration stored in .jj/jjj.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalConfig {
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
}

impl LocalConfig {
    /// Load config from .jj/jjj.toml, with env var overrides.
    /// Returns default config if file doesn't exist.
    pub fn load(repo_root: &Path) -> Self {
        let config_path = repo_root.join(".jj").join("jjj.toml");

        let mut config = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            LocalConfig::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();
        config
    }

    /// Apply environment variable overrides.
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_ENABLED") {
            self.embeddings.enabled = Some(val == "true" || val == "1");
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_BASE_URL") {
            self.embeddings.base_url = Some(val);
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_MODEL") {
            self.embeddings.model = Some(val);
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_DIMENSIONS") {
            if let Ok(dims) = val.parse() {
                self.embeddings.dimensions = Some(dims);
            }
        }
        if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_API_KEY") {
            self.embeddings.api_key = Some(val);
        }
    }

    /// Check if embeddings are explicitly enabled in config.
    pub fn embeddings_explicitly_enabled(&self) -> bool {
        self.embeddings.enabled == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = LocalConfig::default();
        assert!(config.embeddings.enabled.is_none());
        assert!(config.embeddings.base_url.is_none());
        assert!(config.embeddings.model.is_none());
    }

    #[test]
    fn test_env_overrides() {
        // Set env vars
        env::set_var("JJJ_EMBEDDINGS_ENABLED", "true");
        env::set_var("JJJ_EMBEDDINGS_BASE_URL", "http://test:8080/v1");
        env::set_var("JJJ_EMBEDDINGS_MODEL", "test-model");
        env::set_var("JJJ_EMBEDDINGS_DIMENSIONS", "1024");

        let mut config = LocalConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.embeddings.enabled, Some(true));
        assert_eq!(
            config.embeddings.base_url,
            Some("http://test:8080/v1".to_string())
        );
        assert_eq!(config.embeddings.model, Some("test-model".to_string()));
        assert_eq!(config.embeddings.dimensions, Some(1024));

        // Clean up
        env::remove_var("JJJ_EMBEDDINGS_ENABLED");
        env::remove_var("JJJ_EMBEDDINGS_BASE_URL");
        env::remove_var("JJJ_EMBEDDINGS_MODEL");
        env::remove_var("JJJ_EMBEDDINGS_DIMENSIONS");
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
[embeddings]
enabled = true
base_url = "http://localhost:11434/v1"
model = "qwen3-embedding:8b"
dimensions = 4096
"#;

        let config: LocalConfig = toml::from_str(toml_str).expect("Failed to parse TOML");
        assert_eq!(config.embeddings.enabled, Some(true));
        assert_eq!(
            config.embeddings.base_url,
            Some("http://localhost:11434/v1".to_string())
        );
        assert_eq!(
            config.embeddings.model,
            Some("qwen3-embedding:8b".to_string())
        );
        assert_eq!(config.embeddings.dimensions, Some(4096));
    }
}
