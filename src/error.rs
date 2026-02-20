use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, JjjError>;

#[derive(Error, Debug)]
pub enum JjjError {
    #[error("Failed to execute jj command '{args}': {source}")]
    JjIo {
        args: String,
        #[source]
        source: std::io::Error,
    },

    #[error("jj command '{args}' failed:\n{stderr}")]
    JjCommandFailed { args: String, stderr: String },

    #[error("jj executable not found in PATH.\n\nPlease install Jujutsu:\n  macOS: brew install jj\n  From source: cargo install --git https://github.com/martinvonz/jj jj-cli")]
    JjNotFound,

    #[error("Not in a jj repository.\n\nPlease run this command from within a jj repository,\nor initialize one with: jj git init")]
    NotInRepository,

    #[error(
        "jjj not initialized in this repository.\n\nRun 'jjj init' to set up project management."
    )]
    MetaBranchNotFound,

    #[error("Problem {0} not found.\n\nUse 'jjj problem list' to see all problems.")]
    ProblemNotFound(String),

    #[error("Solution {0} not found.\n\nUse 'jjj solution list' to see all solutions.")]
    SolutionNotFound(String),

    #[error("Critique {0} not found.\n\nUse 'jjj critique list' to see all critiques.")]
    CritiqueNotFound(String),

    #[error("Milestone {0} not found.\n\nUse 'jjj milestone list' to see all milestones.")]
    MilestoneNotFound(String),

    #[error("Conflict detected in {0}. Resolve with 'jjj resolve'")]
    Conflict(String),

    #[error("Invalid problem ID: {0}. Expected format: P-<number>")]
    InvalidProblemId(String),

    #[error("Invalid solution ID: {0}. Expected format: S-<number>")]
    InvalidSolutionId(String),

    #[error("Invalid critique ID: {0}. Expected format: CQ-<number>")]
    InvalidCritiqueId(String),

    #[error("Invalid change ID: {0}")]
    InvalidChangeId(String),

    #[error("Cannot mark problem as solved: {0}")]
    CannotSolveProblem(String),

    #[error("Cannot accept solution: {0}")]
    CannotAcceptSolution(String),

    #[error("Failed to parse {entity_type} '{entity_id}': {message}")]
    FrontmatterParse {
        entity_type: String,
        entity_id: String,
        message: String,
    },

    #[error("Failed to parse {field}: {value}")]
    ParseError { field: String, value: String },

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),

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

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Selection cancelled: {0}")]
    Cancelled(String),

    #[error("{0}")]
    AmbiguousMatch(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Ambiguous ID prefix '{prefix}' matches multiple entities: {matches:?}")]
    AmbiguousId {
        prefix: String,
        matches: Vec<String>,
    },

    #[error("{0}")]
    Other(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
