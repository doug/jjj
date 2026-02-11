// Public library interface for jjj
// This allows testing and potential use as a library

pub mod cli;
pub mod commands;
pub mod context;
pub mod db;
pub mod display;
pub mod error;
pub mod id;
pub mod local_config;
pub mod jj;
pub mod models;
pub mod picker;
pub mod resolve;
pub mod storage;
pub mod tui;
pub mod utils;

// Re-export commonly used types
pub use error::{JjjError, Result};
pub use models::{
    Critique, CritiqueSeverity, CritiqueStatus, Milestone, MilestoneStatus, Problem, ProblemStatus,
    ProjectConfig, Solution, SolutionStatus,
};
