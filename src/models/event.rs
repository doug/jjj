use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event types for decision logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Problem events
    ProblemCreated,
    ProblemSolved,
    ProblemDissolved,
    ProblemReopened,

    // Solution events
    SolutionCreated,
    SolutionSubmitted,
    SolutionApproved,
    SolutionWithdrawn,

    // Critique events
    CritiqueRaised,
    CritiqueAddressed,
    CritiqueDismissed,
    CritiqueValidated,
    CritiqueReplied,

    // Milestone events
    MilestoneCreated,
    MilestoneCompleted,

    // GitHub sync events
    GithubIssueCreated,
    GithubIssueImported,
    GithubIssueClosed,
    GithubPrCreated,
    GithubPrMerged,
    GithubReviewImported,
}

impl EventType {
    /// Return the snake_case string for this variant (zero-allocation).
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ProblemCreated => "problem_created",
            Self::ProblemSolved => "problem_solved",
            Self::ProblemDissolved => "problem_dissolved",
            Self::ProblemReopened => "problem_reopened",
            Self::SolutionCreated => "solution_created",
            Self::SolutionSubmitted => "solution_submitted",
            Self::SolutionApproved => "solution_approved",
            Self::SolutionWithdrawn => "solution_withdrawn",
            Self::CritiqueRaised => "critique_raised",
            Self::CritiqueAddressed => "critique_addressed",
            Self::CritiqueDismissed => "critique_dismissed",
            Self::CritiqueValidated => "critique_validated",
            Self::CritiqueReplied => "critique_replied",
            Self::MilestoneCreated => "milestone_created",
            Self::MilestoneCompleted => "milestone_completed",
            Self::GithubIssueCreated => "github_issue_created",
            Self::GithubIssueImported => "github_issue_imported",
            Self::GithubIssueClosed => "github_issue_closed",
            Self::GithubPrCreated => "github_pr_created",
            Self::GithubPrMerged => "github_pr_merged",
            Self::GithubReviewImported => "github_review_imported",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single event in the decision log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// When the event occurred
    pub when: DateTime<Utc>,

    /// Type of event
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// Primary entity ID (UUID7 of problem, solution, critique, or milestone)
    pub entity: String,

    /// Who triggered the event
    pub by: String,

    /// Human explanation of why (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Related entity IDs for linking
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,

    /// Additional context depending on event type
    #[serde(flatten)]
    pub extra: EventExtra,
}

/// Type-specific extra fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventExtra {
    /// For critique_raised: target solution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// For critique_raised: severity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,

    /// For critique_raised: title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// For solution_created: problem ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,

    /// For solution_created: supersedes ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,

    /// For github_* events: GitHub issue/PR number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_number: Option<u64>,
}

impl Event {
    /// Create a new event with current timestamp
    pub fn new(event_type: EventType, entity: impl Into<String>, by: impl Into<String>) -> Self {
        Self {
            when: Utc::now(),
            event_type,
            entity: entity.into(),
            by: by.into(),
            rationale: None,
            refs: Vec::new(),
            extra: EventExtra::default(),
        }
    }

    /// Add rationale
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add refs
    pub fn with_refs(mut self, refs: Vec<String>) -> Self {
        self.refs = refs;
        self
    }

    /// Override the timestamp
    pub fn with_timestamp(mut self, when: DateTime<Utc>) -> Self {
        self.when = when;
        self
    }

    /// Add extra fields
    pub fn with_extra(mut self, extra: EventExtra) -> Self {
        self.extra = extra;
        self
    }

    /// Serialize to JSON line (no trailing newline)
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Format for commit message suffix
    pub fn to_commit_suffix(&self) -> Result<String, serde_json::Error> {
        Ok(format!("jjj: {}", self.to_json_line()?))
    }
}
