//! Error types for jjj.
//!
//! [`JjjError`] is the single error type used throughout the crate. Variants are
//! grouped by concern:
//! - **jj integration** — `JjNotFound`, `JjCommandFailed`, `JjIo`
//! - **Entity lookup** — `ProblemNotFound`, `SolutionNotFound`, `CritiqueNotFound`,
//!   `MilestoneNotFound`, `FindingNotFound`, `EntityNotFound`, `AmbiguousId`,
//!   `AmbiguousMatch`
//! - **Parsing / data integrity** — `FrontmatterParse`, `JsonParse`, `YamlParse`,
//!   `TomlParse`, `TomlSerialize`
//! - **GitHub sync** — `GhNotFound`, `GhCommandFailed`, `GhAuthFailed`,
//!   `GhRepoNotDetected`, `SyncConflict`
//! - **General** — `Validation`, `Config`, `Cancelled`, `Io`, `Database`

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

    #[error("Finding {0} not found.\n\nUse 'jjj finding list' to see all findings.")]
    FindingNotFound(String),

    #[error("Conflict detected in {0}. Run 'jj resolve' to fix conflicts in the working copy.")]
    Conflict(String),

    #[error("Invalid change ID: {0}")]
    InvalidChangeId(String),

    #[error("Cannot mark problem as solved: {0}")]
    CannotSolveProblem(String),

    #[error("Cannot approve solution: {0}")]
    CannotApproveSolution(String),

    #[error("Failed to parse {entity_type} '{entity_id}': {message}\n\nRun 'jjj db validate' to check for corrupted entities.")]
    FrontmatterParse {
        entity_type: String,
        entity_id: String,
        message: String,
    },

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_norway::Error),

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

    /// A command that diagnosed a fault, already printed its own report, and
    /// must now fail the process. The `Display` is bare because the string is
    /// the whole summary — a prefix like "Validation failed" would mis-describe
    /// a diagnostic that ran correctly and found something wrong.
    #[error("{0}")]
    Unhealthy(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Ambiguous ID prefix '{prefix}' matches multiple entities: {matches:?}")]
    AmbiguousId {
        prefix: String,
        matches: Vec<String>,
    },

    #[error("gh CLI not found in PATH.\n\nPlease install GitHub CLI:\n  macOS: brew install gh\n  See: https://cli.github.com/")]
    GhNotFound,

    #[error("Failed to execute gh command '{args}': {source}")]
    GhIo {
        args: String,
        #[source]
        source: std::io::Error,
    },

    #[error("gh command '{args}' failed:\n{stderr}")]
    GhCommandFailed { args: String, stderr: String },

    #[error("GitHub authentication failed.\n\nRun 'gh auth login' to authenticate.")]
    GhAuthFailed,

    #[error("Could not detect GitHub repository.\n\nEnsure this repo has a GitHub remote, or set github.repo in config.")]
    GhRepoNotDetected,

    #[error("Sync conflict for entity {entity_id}: local state '{local_state}' vs remote state '{remote_state}'.\n{suggestion}")]
    SyncConflict {
        entity_id: String,
        local_state: String,
        remote_state: String,
        suggestion: String,
    },

    #[error("Validation failed: {0}")]
    Validation(String),

    /// A guard refused the write because the world moved under the caller.
    ///
    /// Distinct from `Validation` by one property an agent can act on: nothing
    /// was written, and the same command may well succeed after a re-read.
    /// `Validation` means the command itself was wrong, and retrying it changes
    /// nothing. That difference is the whole reason this variant exists —
    /// `--if-lamport` is useless in a loop if a stale read and a malformed
    /// argument are the same exit code and the caller has to tell them apart by
    /// matching on prose. (This session rewrote that prose, which would have
    /// silently broken anyone doing exactly that.)
    #[error("{0}")]
    Precondition(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl JjjError {
    /// The process exit code for this error.
    ///
    /// Two values, because two is what a caller can act on: `13` for a
    /// precondition that may pass on a retry, `1` for everything else. beads
    /// reserves 13 for the same meaning (`if_revision.go:19`), and there is no
    /// reason for two agent-facing tools to disagree on the number.
    ///
    /// Deliberately not a code per variant: every code is a promise to keep,
    /// and a caller that switches on fifteen of them is coupled to jjj's
    /// internal error taxonomy. One retryable class is what conditional writes
    /// actually need.
    pub fn exit_code(&self) -> i32 {
        match self {
            JjjError::Precondition(_) => 13,
            _ => 1,
        }
    }
}
