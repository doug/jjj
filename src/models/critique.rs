use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A critique is an explicit criticism of a solution.
/// Critiques are the mechanism for error elimination - the way we make
/// progress by discovering flaws in our conjectures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Critique {
    /// Unique critique identifier (e.g., "CQ-1")
    pub id: String,

    /// Critique title (summary of the criticism)
    pub title: String,

    /// Solution this critique targets (required)
    pub solution_id: String,

    /// Current status
    pub status: CritiqueStatus,

    /// Severity of the critique
    pub severity: CritiqueSeverity,

    /// Author of the critique
    pub author: Option<String>,

    /// Who should address/review this critique
    pub reviewer: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// The argument - why the solution is problematic (markdown body)
    #[serde(default)]
    pub argument: String,

    /// Supporting evidence or examples
    #[serde(default)]
    pub evidence: String,

    /// Optional file path for code-level critiques
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    /// Starting line number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,

    /// Ending line number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,

    /// Code context for display/matching
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code_context: Vec<String>,

    /// Discussion thread
    #[serde(default)]
    pub replies: Vec<Reply>,

    /// Linked GitHub review ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_review_id: Option<u64>,
}

/// A reply in a critique discussion thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    /// Unique reply identifier (e.g., "CQ-1-R1")
    pub id: String,

    /// Reply author
    pub author: String,

    /// Reply body (markdown)
    pub body: String,

    /// When the reply was created
    pub created_at: DateTime<Utc>,
}

/// Status of a critique
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CritiqueStatus {
    /// Critique raised, not yet addressed
    #[default]
    Open,

    /// Solution modified to address this critique
    Addressed,

    /// Critique accepted as valid - solution should be refuted
    Valid,

    /// Critique shown to be incorrect or irrelevant
    Dismissed,
}

impl std::fmt::Display for CritiqueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CritiqueStatus::Open => write!(f, "open"),
            CritiqueStatus::Addressed => write!(f, "addressed"),
            CritiqueStatus::Valid => write!(f, "valid"),
            CritiqueStatus::Dismissed => write!(f, "dismissed"),
        }
    }
}

impl std::str::FromStr for CritiqueStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(CritiqueStatus::Open),
            "addressed" => Ok(CritiqueStatus::Addressed),
            "valid" => Ok(CritiqueStatus::Valid),
            "dismissed" => Ok(CritiqueStatus::Dismissed),
            _ => Err(format!(
                "Unknown critique status: '{}'. Valid values: open, addressed, valid, dismissed",
                s
            )),
        }
    }
}

/// Severity level for critiques
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum CritiqueSeverity {
    /// Minor issue, doesn't fundamentally challenge the solution
    Low,

    /// Moderate issue that should be addressed
    #[default]
    Medium,

    /// Significant problem that may invalidate the solution
    High,

    /// Critical flaw that definitely invalidates the solution
    Critical,
}

impl std::fmt::Display for CritiqueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CritiqueSeverity::Low => write!(f, "low"),
            CritiqueSeverity::Medium => write!(f, "medium"),
            CritiqueSeverity::High => write!(f, "high"),
            CritiqueSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for CritiqueSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(CritiqueSeverity::Low),
            "medium" => Ok(CritiqueSeverity::Medium),
            "high" => Ok(CritiqueSeverity::High),
            "critical" => Ok(CritiqueSeverity::Critical),
            _ => Err(format!(
                "Unknown critique severity: '{}'. Valid values: low, medium, high, critical",
                s
            )),
        }
    }
}

impl Critique {
    /// Create a new critique of a solution
    pub fn new(id: String, title: String, solution_id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            solution_id,
            status: CritiqueStatus::Open,
            severity: CritiqueSeverity::Medium,
            author: None,
            reviewer: None,
            created_at: now,
            updated_at: now,
            argument: String::new(),
            evidence: String::new(),
            file_path: None,
            line_start: None,
            line_end: None,
            code_context: Vec::new(),
            replies: Vec::new(),
            github_review_id: None,
        }
    }

    /// Update status
    pub fn set_status(&mut self, status: CritiqueStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Check if a status transition is valid.
    pub fn can_transition_to(&self, target: &CritiqueStatus) -> bool {
        matches!(
            (&self.status, target),
            (CritiqueStatus::Open, CritiqueStatus::Addressed)
                | (CritiqueStatus::Open, CritiqueStatus::Valid)
                | (CritiqueStatus::Open, CritiqueStatus::Dismissed)
                | (CritiqueStatus::Addressed, CritiqueStatus::Open)
                | (CritiqueStatus::Addressed, CritiqueStatus::Valid)
                | (CritiqueStatus::Addressed, CritiqueStatus::Dismissed)
        )
    }

    /// Set severity
    pub fn set_severity(&mut self, severity: CritiqueSeverity) {
        self.severity = severity;
        self.updated_at = Utc::now();
    }

    /// Mark as addressed (solution was modified)
    pub fn address(&mut self) {
        self.set_status(CritiqueStatus::Addressed);
    }

    /// Validate the critique (it's correct, solution should be refuted)
    pub fn validate(&mut self) {
        self.set_status(CritiqueStatus::Valid);
    }

    /// Dismiss the critique (it's incorrect or irrelevant)
    pub fn dismiss(&mut self) {
        self.set_status(CritiqueStatus::Dismissed);
    }

    /// Check if critique is still active (needs attention)
    pub fn is_active(&self) -> bool {
        matches!(self.status, CritiqueStatus::Open)
    }

    /// Check if critique is resolved (one way or another)
    pub fn is_resolved(&self) -> bool {
        matches!(
            self.status,
            CritiqueStatus::Addressed | CritiqueStatus::Valid | CritiqueStatus::Dismissed
        )
    }

    /// Check if critique invalidates the solution
    pub fn invalidates_solution(&self) -> bool {
        self.status == CritiqueStatus::Valid
    }

    /// Add a reply to the discussion
    pub fn add_reply(&mut self, author: String, body: String) {
        let reply_num = self.replies.len() + 1;
        let reply = Reply {
            id: format!("{}-r{}", self.id, reply_num),
            author,
            body,
            created_at: Utc::now(),
        };
        self.replies.push(reply);
        self.updated_at = Utc::now();
    }

    /// Set code location for this critique
    pub fn set_location(
        &mut self,
        file_path: String,
        line_start: usize,
        line_end: Option<usize>,
        code_context: Vec<String>,
    ) {
        self.file_path = Some(file_path);
        self.line_start = Some(line_start);
        self.line_end = line_end.or(Some(line_start));
        self.code_context = code_context;
        self.updated_at = Utc::now();
    }

    /// Check if this critique has a code location
    pub fn has_location(&self) -> bool {
        self.file_path.is_some() && self.line_start.is_some()
    }
}

/// YAML frontmatter for Critique markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueFrontmatter {
    pub id: String,
    pub title: String,
    pub solution_id: String,
    pub status: CritiqueStatus,
    pub severity: CritiqueSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_review_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replies: Vec<Reply>,
}

impl From<&Critique> for CritiqueFrontmatter {
    fn from(c: &Critique) -> Self {
        Self {
            id: c.id.clone(),
            title: c.title.clone(),
            solution_id: c.solution_id.clone(),
            status: c.status.clone(),
            severity: c.severity.clone(),
            author: c.author.clone(),
            reviewer: c.reviewer.clone(),
            file_path: c.file_path.clone(),
            line_start: c.line_start,
            line_end: c.line_end,
            created_at: c.created_at,
            updated_at: c.updated_at,
            github_review_id: c.github_review_id,
            replies: c.replies.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_critique() {
        let critique = Critique::new(
            "c1".to_string(),
            "JWT vulnerable to XSS".to_string(),
            "s1".to_string(),
        );
        assert_eq!(critique.id, "c1");
        assert_eq!(critique.title, "JWT vulnerable to XSS");
        assert_eq!(critique.solution_id, "s1");
        assert_eq!(critique.status, CritiqueStatus::Open);
        assert_eq!(critique.severity, CritiqueSeverity::Medium);
        assert!(critique.is_active());
    }

    #[test]
    fn test_status_transitions() {
        let mut critique = Critique::new("c1".to_string(), "Test".to_string(), "s1".to_string());

        assert_eq!(critique.status, CritiqueStatus::Open);
        assert!(critique.is_active());

        critique.address();
        assert_eq!(critique.status, CritiqueStatus::Addressed);
        assert!(critique.is_resolved());
        assert!(!critique.invalidates_solution());
    }

    #[test]
    fn test_valid_critique() {
        let mut critique = Critique::new(
            "c1".to_string(),
            "Critical flaw".to_string(),
            "s1".to_string(),
        );

        critique.validate();
        assert_eq!(critique.status, CritiqueStatus::Valid);
        assert!(critique.is_resolved());
        assert!(critique.invalidates_solution());
    }

    #[test]
    fn test_dismissed_critique() {
        let mut critique = Critique::new(
            "c1".to_string(),
            "Not actually a problem".to_string(),
            "s1".to_string(),
        );

        critique.dismiss();
        assert_eq!(critique.status, CritiqueStatus::Dismissed);
        assert!(critique.is_resolved());
        assert!(!critique.invalidates_solution());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(CritiqueSeverity::Critical > CritiqueSeverity::High);
        assert!(CritiqueSeverity::High > CritiqueSeverity::Medium);
        assert!(CritiqueSeverity::Medium > CritiqueSeverity::Low);
    }

    #[test]
    fn test_status_parsing() {
        assert_eq!(
            "open".parse::<CritiqueStatus>().unwrap(),
            CritiqueStatus::Open
        );
        assert_eq!(
            "addressed".parse::<CritiqueStatus>().unwrap(),
            CritiqueStatus::Addressed
        );
        assert_eq!(
            "valid".parse::<CritiqueStatus>().unwrap(),
            CritiqueStatus::Valid
        );
        assert_eq!(
            "dismissed".parse::<CritiqueStatus>().unwrap(),
            CritiqueStatus::Dismissed
        );
    }

    #[test]
    fn test_severity_parsing() {
        assert_eq!(
            "low".parse::<CritiqueSeverity>().unwrap(),
            CritiqueSeverity::Low
        );
        assert_eq!(
            "medium".parse::<CritiqueSeverity>().unwrap(),
            CritiqueSeverity::Medium
        );
        assert_eq!(
            "high".parse::<CritiqueSeverity>().unwrap(),
            CritiqueSeverity::High
        );
        assert_eq!(
            "critical".parse::<CritiqueSeverity>().unwrap(),
            CritiqueSeverity::Critical
        );
    }

    #[test]
    fn test_add_reply() {
        let mut critique = Critique::new(
            "c1".to_string(),
            "Test critique".to_string(),
            "s1".to_string(),
        );

        critique.add_reply("alice".to_string(), "I disagree".to_string());

        assert_eq!(critique.replies.len(), 1);
        assert_eq!(critique.replies[0].author, "alice");
        assert_eq!(critique.replies[0].body, "I disagree");
        assert!(critique.replies[0].id.starts_with("c1-r"));
    }

    #[test]
    fn test_critique_with_location() {
        let mut critique = Critique::new(
            "c1".to_string(),
            "SQL injection".to_string(),
            "s1".to_string(),
        );

        critique.set_location(
            "src/db.rs".to_string(),
            42,
            Some(45),
            vec!["let query = format!(...)".to_string()],
        );

        assert_eq!(critique.file_path, Some("src/db.rs".to_string()));
        assert_eq!(critique.line_start, Some(42));
        assert_eq!(critique.line_end, Some(45));
        assert!(critique.has_location());
    }

    #[test]
    fn test_critique_without_location() {
        let critique = Critique::new(
            "c1".to_string(),
            "Conceptual critique".to_string(),
            "s1".to_string(),
        );

        assert!(!critique.has_location());
    }

    #[test]
    fn test_critique_with_reviewer() {
        let mut critique = Critique::new(
            "c1".to_string(),
            "Awaiting review from @bob".to_string(),
            "s1".to_string(),
        );
        critique.reviewer = Some("bob".to_string());
        assert_eq!(critique.reviewer, Some("bob".to_string()));
    }
}
