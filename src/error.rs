use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, JjjError>;

#[derive(Error, Debug)]
pub enum JjjError {
    #[error("Failed to execute jj command: {0}")]
    JjExecution(String),

    #[error("jj executable not found in PATH.\n\nPlease install Jujutsu:\n  macOS: brew install jj\n  From source: cargo install --git https://github.com/martinvonz/jj jj-cli")]
    JjNotFound,

    #[error("Not in a jj repository.\n\nPlease run this command from within a jj repository,\nor initialize one with: jj git init")]
    NotInRepository,

    #[error("jjj not initialized in this repository.\n\nRun 'jjj init' to set up project management.")]
    MetaBranchNotFound,

    #[error("Task {0} not found.\n\nUse 'jjj task list' to see all tasks.")]
    TaskNotFound(String),

    #[error("Review for change {0} not found.\n\nUse 'jjj review list' to see pending reviews\nor 'jjj review request' to create one.")]
    ReviewNotFound(String),

    #[error("Conflict detected in {0}. Resolve with 'jjj resolve'")]
    Conflict(String),

    #[error("Invalid task ID: {0}")]
    InvalidTaskId(String),

    #[error("Invalid change ID: {0}")]
    InvalidChangeId(String),

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Failed to serialize TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {0}")]
    PathError(PathBuf),

    #[error("TUI error: {0}")]
    Tui(String),

    #[error("{0}")]
    Other(String),
}

impl From<String> for JjjError {
    fn from(s: String) -> Self {
        JjjError::Other(s)
    }
}

impl From<&str> for JjjError {
    fn from(s: &str) -> Self {
        JjjError::Other(s.to_string())
    }
}
