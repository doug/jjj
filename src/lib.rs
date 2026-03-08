//! jjj (Jujutsu Juggler) — distributed project management using Popperian epistemology.
//!
//! Entities follow a three-tier model:
//! - **Problems**: things to solve, forming a DAG via `parent_id`
//! - **Solutions**: conjectures attached to jj Change IDs (not commit hashes)
//! - **Critiques**: error-elimination feedback that can block solution approval
//!
//! All metadata lives in an orphaned `jjj` bookmark and syncs via `jj git push/fetch`.
//! There is no central server and no database required for the core workflow.
//! SQLite is used as a local cache for search and embeddings only.

pub mod automation;
pub mod cli;
pub mod commands;
pub mod context;
pub mod db;
pub mod display;
pub mod embeddings;
pub mod error;
pub mod id;
pub mod jj;
pub mod local_config;
pub mod models;
pub mod picker;
pub mod resolve;
pub mod storage;
pub mod sync;
pub mod tui;
pub mod utils;

// Re-export commonly used types
pub use error::{JjjError, Result};
pub use models::{
    Critique, CritiqueSeverity, CritiqueStatus, Milestone, MilestoneStatus, Problem, ProblemStatus,
    ProjectConfig, Solution, SolutionStatus,
};
