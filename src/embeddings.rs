//! Embedding client for computing vector embeddings via the Ollama API.
//!
//! Uses stdlib TcpStream to talk directly to a local Ollama endpoint
//! (default: localhost:11434). No external HTTP client dependency needed.

use crate::local_config::LocalConfig;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};

/// Default Ollama host and port
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 11434;
const DEFAULT_PATH: &str = "/v1/embeddings";
const DEFAULT_MODEL: &str = "qwen3-embedding:8b";
const DEFAULT_DIMENSIONS: usize = 4096;

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
    Http(#[from] std::io::Error),

    #[error("API returned error status {status}: {body}")]
    Api { status: u16, body: String },

    #[error("Failed to parse embedding response: {0}")]
    Parse(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Empty response from API")]
    EmptyResponse,
}

/// Client for computing embeddings via a local Ollama instance.
pub struct EmbeddingClient {
    host: String,
    port: u16,
    path: String,
    model: String,
    dimensions: usize,
}

impl EmbeddingClient {
    /// Create a new embedding client from config.
    ///
    /// Tests the connection immediately. Returns None if the service is unavailable.
    pub fn from_config(config: &LocalConfig, warn_on_error: bool) -> Option<Self> {
        // Parse host/port from base_url if provided, otherwise use defaults
        let (host, port, path) = if let Some(ref base_url) = config.embeddings.base_url {
            parse_base_url(base_url)
        } else {
            (
                DEFAULT_HOST.to_string(),
                DEFAULT_PORT,
                DEFAULT_PATH.to_string(),
            )
        };

        let model = config
            .embeddings
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let dimensions = config.embeddings.dimensions.unwrap_or(DEFAULT_DIMENSIONS);

        let client = Self {
            host: host.clone(),
            port,
            path,
            model,
            dimensions,
        };

        // Test connection
        match client.embed("test") {
            Ok(_) => Some(client),
            Err(e) => {
                if warn_on_error && !WARNED_THIS_SESSION.swap(true, Ordering::SeqCst) {
                    eprintln!(
                        "Warning: Embedding service unavailable at {}:{}: {}",
                        host, port, e
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

        let request = EmbeddingRequest {
            model: &self.model,
            input: texts.to_vec(),
        };
        let body =
            serde_json::to_string(&request).map_err(|e| EmbeddingError::Parse(e.to_string()))?;

        let response_body = self.http_post(&body)?;

        let response: EmbeddingResponse = serde_json::from_str(&response_body)
            .map_err(|e| EmbeddingError::Parse(format!("{}: {}", e, response_body)))?;

        // Validate dimensions of first result
        if let Some(data) = response.data.first() {
            if data.embedding.len() != self.dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: data.embedding.len(),
                });
            }
        }

        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }

    /// Send a raw HTTP POST to the Ollama endpoint and return the response body.
    fn http_post(&self, body: &str) -> Result<String, EmbeddingError> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr)?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;

        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.path, self.host, self.port, body.len(), body
        );
        stream.write_all(request.as_bytes())?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader.read_line(&mut status_line)?;

        let status_code = parse_status_code(&status_line);

        // Skip headers
        loop {
            let mut header = String::new();
            reader.read_line(&mut header)?;
            if header == "\r\n" || header.is_empty() {
                break;
            }
        }

        // Read body
        let mut response_body = String::new();
        use std::io::Read;
        reader.read_to_string(&mut response_body)?;

        if status_code != 200 {
            return Err(EmbeddingError::Api {
                status: status_code,
                body: response_body,
            });
        }

        Ok(response_body)
    }
}

/// Parse host, port, and path from a base URL string like "http://localhost:11434/v1".
fn parse_base_url(base_url: &str) -> (String, u16, String) {
    // Strip scheme
    let without_scheme = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url);

    // Split host:port from path
    let (host_port, path) = if let Some(idx) = without_scheme.find('/') {
        let path = format!(
            "{}/embeddings",
            &without_scheme[idx..].trim_end_matches('/')
        );
        (&without_scheme[..idx], path)
    } else {
        (without_scheme, DEFAULT_PATH.to_string())
    };

    // Split host and port
    if let Some(colon) = host_port.rfind(':') {
        let host = host_port[..colon].to_string();
        let port = host_port[colon + 1..].parse().unwrap_or(DEFAULT_PORT);
        (host, port, path)
    } else {
        (host_port.to_string(), DEFAULT_PORT, path)
    }
}

/// Extract the HTTP status code from a status line like "HTTP/1.1 200 OK".
fn parse_status_code(status_line: &str) -> u16 {
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
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

    #[test]
    fn test_parse_base_url_default() {
        let (host, port, path) = parse_base_url("http://localhost:11434/v1");
        assert_eq!(host, "localhost");
        assert_eq!(port, 11434);
        assert_eq!(path, "/v1/embeddings");
    }

    #[test]
    fn test_parse_base_url_custom_port() {
        let (host, port, _) = parse_base_url("http://localhost:9999/v1");
        assert_eq!(host, "localhost");
        assert_eq!(port, 9999);
    }

    #[test]
    fn test_parse_status_code() {
        assert_eq!(parse_status_code("HTTP/1.1 200 OK\r\n"), 200);
        assert_eq!(parse_status_code("HTTP/1.1 404 Not Found\r\n"), 404);
        assert_eq!(
            parse_status_code("HTTP/1.1 500 Internal Server Error\r\n"),
            500
        );
    }
}
