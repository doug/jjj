use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A finding is **evidence**: something that was measured, reproduced, or
/// established about a problem.
///
/// jjj models conjectures ([`crate::models::Solution`]) and refutations
/// ([`crate::models::Critique`]) and, until this type existed, nothing else. So
/// investigations arrived disguised as solutions — "Symbol-size breakdown of
/// gallery.wasm", "Measure the 15MB", "Root cause found and documented; not
/// fixed" — and were then either approved as though they were code or withdrawn
/// as "not fixed". Across two swarm trials, seven solutions were investigations
/// wearing a solution's clothes, and that last title is the fleet hand-rolling a
/// workaround for a missing concept.
///
/// Popper distinguishes a conjecture from the observations that motivate it. A
/// finding is not a fourth thing bolted onto the model; it is the missing third.
///
/// # Why there is no approval state
///
/// A measurement is not accepted or rejected — it is cited, or it is
/// contradicted by a better measurement. So the only transition is
/// [`FindingStatus::Superseded`], which records *which* finding replaced it.
/// Adding review states here would import positive justification into a system
/// built on refutation.
///
/// # Serialization
///
/// `evidence` is the markdown body, not part of the YAML frontmatter. See the
/// doc on [`crate::models::Problem`] for full serialization rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique finding identifier (UUID7).
    pub id: String,

    /// What was found, stated as a claim ("decode.parse has a 120,004-op floor").
    pub title: String,

    /// Problem this is evidence about (required).
    ///
    /// A finding with no problem is a note, and notes belong somewhere else.
    pub problem_id: String,

    /// Whether this is still the best measurement, or has been superseded.
    pub status: FindingStatus,

    /// Who measured it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// The finding that replaced this one, set when `status` is `Superseded`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,

    /// Entities this finding bears on — solutions it explains, critiques it
    /// answers, other findings it builds upon.
    ///
    /// Free-form UUIDs rather than a typed FK: a finding routinely relates to
    /// several kinds of thing at once, and a column per kind would be four
    /// columns mostly holding NULL.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,

    /// How the measurement was taken, so someone else can repeat it.
    ///
    /// The single most valuable field on the type: a number nobody can reproduce
    /// is a rumour. Optional because some findings are arguments from code
    /// reading rather than from running something.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Tags for flexible categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,

    /// Markdown body: the evidence itself. Not stored in the YAML frontmatter;
    /// stripped by `to_markdown_strip` on save and assigned from the body on
    /// load.
    #[serde(default)]
    pub evidence: String,
}

/// Whether a finding is still the best available measurement.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FindingStatus {
    /// Stands until contradicted.
    #[default]
    Current,

    /// A later, better measurement replaced this one. Kept rather than deleted:
    /// knowing a number was once believed — and what corrected it — is why the
    /// same investigation does not get run a third time.
    Superseded,
}

impl std::str::FromStr for FindingStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "current" => Ok(FindingStatus::Current),
            "superseded" => Ok(FindingStatus::Superseded),
            _ => Err(format!(
                "Unknown finding status: '{}'. Valid values: current, superseded",
                s
            )),
        }
    }
}

impl Finding {
    /// Create a new finding attached to a problem.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        problem_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            problem_id: problem_id.into(),
            status: FindingStatus::Current,
            author: None,
            superseded_by: None,
            refs: Vec::new(),
            method: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            evidence: String::new(),
        }
    }

    /// Whether this finding still stands.
    pub fn is_current(&self) -> bool {
        self.status == FindingStatus::Current
    }

    /// Mark this finding as replaced by `replacement`.
    ///
    /// Refuses to supersede a finding by itself: the resulting record would
    /// claim its own replacement and `finding show` would loop following it.
    pub fn supersede(&mut self, replacement: impl Into<String>) -> Result<(), String> {
        let replacement = replacement.into();
        if replacement == self.id {
            return Err("a finding cannot supersede itself".to_string());
        }
        self.status = FindingStatus::Superseded;
        self.superseded_by = Some(replacement);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Restore a superseded finding to current, dropping the back-reference.
    pub fn restore(&mut self) {
        self.status = FindingStatus::Current;
        self.superseded_by = None;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_finding_is_current_and_unsuperseded() {
        let f = Finding::new("f1", "decode.parse floors at 120,004 ops", "p1");
        assert!(f.is_current());
        assert_eq!(f.superseded_by, None);
        assert_eq!(f.problem_id, "p1");
    }

    #[test]
    fn superseding_records_the_replacement() {
        let mut f = Finding::new("f1", "The wasm blob is 11.27MB", "p1");
        f.supersede("f2").unwrap();
        assert_eq!(f.status, FindingStatus::Superseded);
        assert_eq!(f.superseded_by, Some("f2".to_string()));
        assert!(!f.is_current());
    }

    #[test]
    fn a_finding_cannot_supersede_itself() {
        // Otherwise `finding show` follows superseded_by into a cycle, and the
        // record asserts something that cannot be true.
        let mut f = Finding::new("f1", "Test", "p1");
        assert!(f.supersede("f1").is_err());
        assert!(
            f.is_current(),
            "the failed transition must not mutate state"
        );
    }

    #[test]
    fn restoring_clears_the_back_reference() {
        let mut f = Finding::new("f1", "Test", "p1");
        f.supersede("f2").unwrap();
        f.restore();
        assert!(f.is_current());
        assert_eq!(
            f.superseded_by, None,
            "a restored finding still pointing at its replacement reads as superseded"
        );
    }

    #[test]
    fn status_parses_from_the_serialized_form() {
        assert_eq!(
            "current".parse::<FindingStatus>().unwrap(),
            FindingStatus::Current
        );
        assert_eq!(
            "superseded".parse::<FindingStatus>().unwrap(),
            FindingStatus::Superseded
        );
        assert!("approved".parse::<FindingStatus>().is_err());
    }
}
