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
    SolutionAccepted,
    SolutionRefuted,

    // Critique events
    CritiqueRaised,
    CritiqueAddressed,
    CritiqueDismissed,
    CritiqueValidated,

    // Milestone events
    MilestoneCreated,
    MilestoneCompleted,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::ProblemCreated => write!(f, "problem_created"),
            EventType::ProblemSolved => write!(f, "problem_solved"),
            EventType::ProblemDissolved => write!(f, "problem_dissolved"),
            EventType::ProblemReopened => write!(f, "problem_reopened"),
            EventType::SolutionCreated => write!(f, "solution_created"),
            EventType::SolutionAccepted => write!(f, "solution_accepted"),
            EventType::SolutionRefuted => write!(f, "solution_refuted"),
            EventType::CritiqueRaised => write!(f, "critique_raised"),
            EventType::CritiqueAddressed => write!(f, "critique_addressed"),
            EventType::CritiqueDismissed => write!(f, "critique_dismissed"),
            EventType::CritiqueValidated => write!(f, "critique_validated"),
            EventType::MilestoneCreated => write!(f, "milestone_created"),
            EventType::MilestoneCompleted => write!(f, "milestone_completed"),
        }
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

    /// Primary entity ID (p1, s1, c1, m1)
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
}

impl Event {
    /// Create a new event with current timestamp
    pub fn new(event_type: EventType, entity: String, by: String) -> Self {
        Self {
            when: Utc::now(),
            event_type,
            entity,
            by,
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
