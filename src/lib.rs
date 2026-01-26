// Public library interface for jjj
// This allows testing and potential use as a library

pub mod cli;
pub mod commands;
pub mod error;
pub mod jj;
pub mod models;
pub mod storage;
pub mod tui;
pub mod utils;

// Re-export commonly used types
pub use error::{JjjError, Result};
pub use models::{
    Critique, CritiqueSeverity, CritiqueStatus, Milestone,
    MilestoneStatus, Problem, ProblemStatus, ProjectConfig,
    Solution, SolutionStatus,
};
