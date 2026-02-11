//! Embedding client for computing vector embeddings via OpenAI-compatible APIs.
//!
//! Supports Ollama (default), OpenAI, and other compatible providers.

use crate::local_config::LocalConfig;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Default Ollama embedding endpoint
const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";
const DEFAULT_MODEL: &str = "nomic-embed-text";
const DEFAULT_DIMENSIONS: usize = 768;

/// Track if we've already warned about connection failure this session
static WARNED_THIS_SESSION: AtomicBool = AtomicBool::new(false);

/// Request body for the OpenAI-compatible embeddings API
#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
}

/// Response from the embeddings API
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Error type for embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API returned error: {0}")]
    Api(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Empty response from API")]
    EmptyResponse,
}

/// Client for computing embeddings via OpenAI-compatible API.
pub struct EmbeddingClient {
    base_url: String,
    model: String,
    dimensions: usize,
    api_key: Option<String>,
    http_client: reqwest::blocking::Client,
}

impl EmbeddingClient {
    /// Create a new embedding client from config.
    ///
    /// If `warn_on_error` is true and connection fails, logs a warning.
    /// Returns None if the service is unavailable.
    pub fn from_config(config: &LocalConfig, warn_on_error: bool) -> Option<Self> {
        let base_url = config
            .embeddings
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let model = config
            .embeddings
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let dimensions = config.embeddings.dimensions.unwrap_or(DEFAULT_DIMENSIONS);
        let api_key = config.embeddings.api_key.clone();

        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;

        let client = Self {
            base_url,
            model,
            dimensions,
            api_key,
            http_client,
        };

        // Test connection with a simple embed request
        match client.embed("test") {
            Ok(_) => Some(client),
            Err(e) => {
                if warn_on_error && !WARNED_THIS_SESSION.swap(true, Ordering::SeqCst) {
                    eprintln!(
                        "Warning: Embedding service unavailable at {}: {}",
                        client.base_url, e
                    );
                    eprintln!("Semantic search features will be disabled.");
                }
                None
            }
        }
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Compute embedding for a single text.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embeddings = self.embed_batch(&[text])?;
        embeddings
            .into_iter()
            .next()
            .ok_or(EmbeddingError::EmptyResponse)
    }

    /// Compute embeddings for multiple texts in a single API call.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/embeddings", self.base_url);
        let request = EmbeddingRequest {
            model: &self.model,
            input: texts.to_vec(),
        };

        let mut req = self.http_client.post(&url).json(&request);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(EmbeddingError::Api(format!("{}: {}", status, body)));
        }

        let response: EmbeddingResponse = response.json()?;

        // Validate dimensions
        for (i, data) in response.data.iter().enumerate() {
            if data.embedding.len() != self.dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: data.embedding.len(),
                });
            }
            // Only check first one in production for performance
            if i == 0 {
                break;
            }
        }

        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (ai, bi) in a.iter().zip(b.iter()) {
        dot_product += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot_product / denominator
    }
}

/// Prepare text for embedding from a problem.
pub fn prepare_problem_text(title: &str, description: &str, context: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, description, context)
        .trim()
        .to_string()
}

/// Prepare text for embedding from a solution.
pub fn prepare_solution_text(title: &str, approach: &str, tradeoffs: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, approach, tradeoffs)
        .trim()
        .to_string()
}

/// Prepare text for embedding from a critique.
pub fn prepare_critique_text(title: &str, argument: &str, evidence: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, argument, evidence)
        .trim()
        .to_string()
}

/// Prepare text for embedding from a milestone.
pub fn prepare_milestone_text(title: &str, goals: &str, success_criteria: &str) -> String {
    format!("{}\n\n{}\n\n{}", title, goals, success_criteria)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_prepare_problem_text() {
        let text = prepare_problem_text("Title", "Description", "Context");
        assert_eq!(text, "Title\n\nDescription\n\nContext");
    }

    #[test]
    fn test_prepare_solution_text() {
        let text = prepare_solution_text("Title", "Approach", "Tradeoffs");
        assert_eq!(text, "Title\n\nApproach\n\nTradeoffs");
    }

    #[test]
    fn test_prepare_critique_text() {
        let text = prepare_critique_text("Title", "Argument", "Evidence");
        assert_eq!(text, "Title\n\nArgument\n\nEvidence");
    }

    #[test]
    fn test_prepare_milestone_text() {
        let text = prepare_milestone_text("Title", "Goals", "Criteria");
        assert_eq!(text, "Title\n\nGoals\n\nCriteria");
    }
}
