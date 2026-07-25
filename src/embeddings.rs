//! In-process embedding client (Pillar 6 — "offline / no server" made true).
//!
//! With the `semantic` cargo feature, embeddings are computed by an in-process
//! BERT-family sentence encoder via candle: tokenize → forward → mean-pool →
//! L2-normalize. The model (`config.json`, `tokenizer.json`,
//! `model.safetensors`) is loaded from a runtime path — never compiled into
//! the binary — so the repo and default build stay lean:
//!
//! ```text
//! ~/.cache/jjj/models/all-MiniLM-L6-v2/     # default location
//! ```
//!
//! Download once with e.g.:
//!
//! ```text
//! huggingface-cli download sentence-transformers/all-MiniLM-L6-v2 \
//!   config.json tokenizer.json model.safetensors \
//!   --local-dir ~/.cache/jjj/models/all-MiniLM-L6-v2
//! ```
//!
//! Without the feature, `EmbeddingClient::from_config` always returns `None`
//! and search runs FTS-only. (The previous Ollama HTTP backend is gone — the
//! external-server dependency was exactly what Pillar 6 set out to remove.)

use crate::local_config::LocalConfig;
use std::sync::atomic::{AtomicBool, Ordering};

/// Default model directory name under `~/.cache/jjj/models/`.
#[cfg_attr(not(feature = "semantic"), allow(dead_code))]
const DEFAULT_MODEL_NAME: &str = "all-MiniLM-L6-v2";

/// Track if we've already warned about embedding unavailability this session
static WARNED_THIS_SESSION: AtomicBool = AtomicBool::new(false);

/// Error type for embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Empty response from model")]
    EmptyResponse,

    #[error("jjj was built without the `semantic` feature")]
    Disabled,
}

/// Resolve the model directory: explicit config/env path, else the default
/// cache location for the configured (or default) model name.
#[cfg_attr(not(feature = "semantic"), allow(dead_code))]
fn model_dir(config: &LocalConfig) -> std::path::PathBuf {
    if let Some(ref p) = config.embeddings.model_path {
        return std::path::PathBuf::from(shellexpand_home(p));
    }
    let name = config
        .embeddings
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_MODEL_NAME.to_string());
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".cache")
        .join("jjj")
        .join("models")
        .join(name)
}

/// Expand a leading `~/` to `$HOME` (config files are written by hand).
#[cfg_attr(not(feature = "semantic"), allow(dead_code))]
fn shellexpand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

fn warn_once(msg: &str) {
    if !WARNED_THIS_SESSION.swap(true, Ordering::SeqCst) {
        eprintln!("Warning: {}", msg);
        eprintln!("Semantic search features will be disabled.");
    }
}

// ============================================================================
// Backend: in-process candle BERT (feature = "semantic")
// ============================================================================

#[cfg(feature = "semantic")]
mod bert {
    use super::*;
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{BertModel, Config, DTYPE};
    use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

    /// In-process sentence-embedding client: BERT forward → mean-pool over the
    /// attention mask → L2-normalize.
    pub struct EmbeddingClient {
        model: BertModel,
        tokenizer: Tokenizer,
        device: Device,
        model_name: String,
        dimensions: usize,
    }

    impl EmbeddingClient {
        /// Load the model from the configured runtime path.
        ///
        /// Returns `None` (optionally with a one-time warning) if the model
        /// files are missing or fail to load — search then runs FTS-only.
        pub fn from_config(config: &LocalConfig, warn_on_error: bool) -> Option<Self> {
            let dir = super::model_dir(config);
            match Self::load(&dir, config) {
                Ok(client) => Some(client),
                Err(e) => {
                    if warn_on_error {
                        super::warn_once(&format!(
                            "embedding model unavailable at {}: {}\n  Download it with:\n  \
                             huggingface-cli download sentence-transformers/{} \
                             config.json tokenizer.json model.safetensors --local-dir {}",
                            dir.display(),
                            e,
                            super::DEFAULT_MODEL_NAME,
                            dir.display(),
                        ));
                    }
                    None
                }
            }
        }

        fn load(dir: &std::path::Path, config: &LocalConfig) -> Result<Self, EmbeddingError> {
            let config_path = dir.join("config.json");
            let tokenizer_path = dir.join("tokenizer.json");
            let weights_path = dir.join("model.safetensors");
            for p in [&config_path, &tokenizer_path, &weights_path] {
                if !p.exists() {
                    return Err(EmbeddingError::Model(format!("missing {}", p.display())));
                }
            }

            let bert_config: Config = serde_json::from_str(&std::fs::read_to_string(&config_path)?)
                .map_err(|e| EmbeddingError::Model(format!("bad config.json: {}", e)))?;
            let dimensions = bert_config.hidden_size;

            // A user-configured dimension that disagrees with the model is a
            // misconfiguration worth failing loudly on — silently embedding at
            // the wrong width would poison the vector table.
            if let Some(want) = config.embeddings.dimensions {
                if want != dimensions {
                    return Err(EmbeddingError::DimensionMismatch {
                        expected: want,
                        actual: dimensions,
                    });
                }
            }

            let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| EmbeddingError::Model(format!("bad tokenizer.json: {}", e)))?;
            // Pad to the longest sequence in each batch; truncate to the
            // model's positional capacity.
            tokenizer.with_padding(Some(PaddingParams::default()));
            tokenizer
                .with_truncation(Some(TruncationParams {
                    max_length: bert_config.max_position_embeddings,
                    ..Default::default()
                }))
                .map_err(|e| EmbeddingError::Model(format!("tokenizer truncation: {}", e)))?;

            let device = Device::Cpu;
            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[&weights_path], DTYPE, &device)
                    .map_err(|e| EmbeddingError::Model(format!("bad safetensors: {}", e)))?
            };
            let model = BertModel::load(vb, &bert_config)
                .map_err(|e| EmbeddingError::Model(format!("model load: {}", e)))?;

            let model_name = config
                .embeddings
                .model
                .clone()
                .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| super::DEFAULT_MODEL_NAME.to_string());

            Ok(Self {
                model,
                tokenizer,
                device,
                model_name,
                dimensions,
            })
        }

        /// Get the model name.
        pub fn model(&self) -> &str {
            &self.model_name
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

        /// Compute embeddings for multiple texts in one forward pass.
        pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            if texts.is_empty() {
                return Ok(Vec::new());
            }

            let err =
                |what: &'static str| move |e| EmbeddingError::Model(format!("{}: {}", what, e));

            let encodings = self
                .tokenizer
                .encode_batch(texts.to_vec(), true)
                .map_err(|e| EmbeddingError::Model(format!("tokenize: {}", e)))?;

            let ids: Vec<Vec<u32>> = encodings.iter().map(|e| e.get_ids().to_vec()).collect();
            let mask: Vec<Vec<u32>> = encodings
                .iter()
                .map(|e| e.get_attention_mask().to_vec())
                .collect();

            let input_ids = Tensor::new(ids, &self.device).map_err(err("input_ids"))?;
            let attention_mask = Tensor::new(mask, &self.device).map_err(err("mask"))?;
            let token_type_ids = input_ids.zeros_like().map_err(err("token_type_ids"))?;

            // (batch, seq, hidden)
            let hidden = self
                .model
                .forward(&input_ids, &token_type_ids, Some(&attention_mask))
                .map_err(err("forward"))?;

            // Mean-pool over real (unmasked) tokens.
            let mask_f = attention_mask
                .to_dtype(DTYPE)
                .map_err(err("mask dtype"))?
                .unsqueeze(2)
                .map_err(err("mask shape"))?; // (batch, seq, 1)
            let summed = hidden
                .broadcast_mul(&mask_f)
                .map_err(err("mask mul"))?
                .sum(1)
                .map_err(err("sum"))?; // (batch, hidden)
            let counts = mask_f.sum(1).map_err(err("mask sum"))?; // (batch, 1)
            let mean = summed.broadcast_div(&counts).map_err(err("mean"))?;

            // L2-normalize each row.
            let norm = mean
                .sqr()
                .map_err(err("sqr"))?
                .sum_keepdim(1)
                .map_err(err("norm sum"))?
                .sqrt()
                .map_err(err("sqrt"))?;
            let normalized = mean.broadcast_div(&norm).map_err(err("normalize"))?;

            let out: Vec<Vec<f32>> = normalized.to_vec2().map_err(err("to_vec"))?;
            if let Some(first) = out.first() {
                if first.len() != self.dimensions {
                    return Err(EmbeddingError::DimensionMismatch {
                        expected: self.dimensions,
                        actual: first.len(),
                    });
                }
            }
            Ok(out)
        }
    }
}

#[cfg(feature = "semantic")]
pub use bert::EmbeddingClient;

// ============================================================================
// Backend: none (default build) — semantic search disabled
// ============================================================================

#[cfg(not(feature = "semantic"))]
pub struct EmbeddingClient {
    _private: (),
}

#[cfg(not(feature = "semantic"))]
impl EmbeddingClient {
    /// Always `None`: this build has no embedding backend. Warns once when the
    /// user explicitly enabled embeddings so the silence is explained.
    pub fn from_config(_config: &LocalConfig, warn_on_error: bool) -> Option<Self> {
        if warn_on_error {
            warn_once(
                "this jjj build has no embedding backend — rebuild with \
                 `cargo install --features semantic` (or `cargo build --features semantic`)",
            );
        }
        None
    }

    pub fn model(&self) -> &str {
        ""
    }

    pub fn dimensions(&self) -> usize {
        0
    }

    pub fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::Disabled)
    }

    pub fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Err(EmbeddingError::Disabled)
    }
}

// ============================================================================
// Shared helpers (backend-independent)
// ============================================================================

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
pub fn prepare_problem_text(title: &str, description: &str) -> String {
    format!("{}\n\n{}", title, description).trim().to_string()
}

/// Prepare text for embedding from a solution.
pub fn prepare_solution_text(title: &str, approach: &str) -> String {
    format!("{}\n\n{}", title, approach).trim().to_string()
}

/// Prepare text for embedding from a critique.
pub fn prepare_critique_text(title: &str, argument: &str) -> String {
    format!("{}\n\n{}", title, argument).trim().to_string()
}

/// Prepare text for embedding from a milestone.
pub fn prepare_milestone_text(title: &str, description: &str) -> String {
    format!("{}\n\n{}", title, description).trim().to_string()
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
        let text = prepare_problem_text("Title", "Description");
        assert_eq!(text, "Title\n\nDescription");
    }

    #[test]
    fn test_prepare_solution_text() {
        let text = prepare_solution_text("Title", "Approach");
        assert_eq!(text, "Title\n\nApproach");
    }

    #[test]
    fn test_prepare_critique_text() {
        let text = prepare_critique_text("Title", "Argument");
        assert_eq!(text, "Title\n\nArgument");
    }

    #[test]
    fn test_prepare_milestone_text() {
        let text = prepare_milestone_text("Title", "Description");
        assert_eq!(text, "Title\n\nDescription");
    }

    #[test]
    fn test_shellexpand_home() {
        std::env::var("HOME").expect("HOME set in test env");
        let expanded = shellexpand_home("~/models/x");
        assert!(!expanded.starts_with('~'));
        assert!(expanded.ends_with("/models/x"));
        assert_eq!(shellexpand_home("/abs/path"), "/abs/path");
    }

    /// Real end-to-end embedding test. Runs only with the `semantic` feature
    /// AND a model present at the default/configured path — skips otherwise,
    /// so CI without the model stays green.
    #[cfg(feature = "semantic")]
    #[test]
    fn test_bert_embed_end_to_end() {
        let config = LocalConfig::default();
        let Some(client) = EmbeddingClient::from_config(&config, false) else {
            eprintln!("skipping: no embedding model at default path");
            return;
        };

        let vecs = client
            .embed_batch(&[
                "the authentication service rejects valid tokens",
                "login fails even with a correct password",
                "the dishwasher needs a new rinse aid dispenser",
            ])
            .expect("embed_batch");
        assert_eq!(vecs.len(), 3);
        assert_eq!(vecs[0].len(), client.dimensions());

        // L2-normalized: unit norm
        for v in &vecs {
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 0.01, "norm was {}", norm);
        }

        // Related texts must beat the unrelated one
        let related = cosine_similarity(&vecs[0], &vecs[1]);
        let unrelated = cosine_similarity(&vecs[0], &vecs[2]);
        assert!(
            related > unrelated,
            "related {} <= unrelated {}",
            related,
            unrelated
        );
    }
}
