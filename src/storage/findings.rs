//! Finding-specific storage methods.
//!
//! Generic load/save/list come from the `Persist` trait plus the generic
//! methods on `MetadataStore`. This file holds only the type-specific queries.

use super::MetadataStore;
use crate::error::Result;
use crate::models::{Finding, FindingStatus};

impl MetadataStore {
    /// Load a finding by ID.
    pub fn load_finding(&self, finding_id: &str) -> Result<Finding> {
        self.load::<Finding>(finding_id)
    }

    /// Save a finding.
    pub fn save_finding(&self, finding: &Finding) -> Result<()> {
        self.save(finding)
    }

    /// List all findings.
    pub fn list_findings(&self) -> Result<Vec<Finding>> {
        self.list::<Finding>()
    }

    /// Generate the next finding ID (UUID7).
    pub fn next_finding_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Delete a finding.
    pub fn delete_finding(&self, finding_id: &str) -> Result<()> {
        self.delete_file_and_cache::<Finding>(finding_id)
    }

    /// Findings recorded against a problem, oldest first.
    ///
    /// Chronological rather than newest-first because evidence accumulates: the
    /// order it was established in is how you read a line of investigation.
    pub fn list_findings_for_problem(&self, problem_id: &str) -> Result<Vec<Finding>> {
        self.query_ids_or_fallback(
            "SELECT id FROM findings WHERE problem_id = ?1 ORDER BY created_at",
            rusqlite::params![problem_id],
            || {
                let mut findings: Vec<Finding> = self
                    .list_findings()?
                    .into_iter()
                    .filter(|f| f.problem_id == problem_id)
                    .collect();
                findings.sort_by_key(|f| f.created_at);
                Ok(findings)
            },
        )
    }

    /// Findings on a problem that have not been superseded.
    ///
    /// The post-load status filter guards the same race the critique variant
    /// does: a cache row may say `current` while the markdown has moved on.
    pub fn list_current_findings_for_problem(&self, problem_id: &str) -> Result<Vec<Finding>> {
        Ok(self
            .list_findings_for_problem(problem_id)?
            .into_iter()
            .filter(|f| f.status == FindingStatus::Current)
            .collect())
    }

    /// Findings that name `entity_id` in their `refs`.
    ///
    /// Used to show the evidence behind a solution or critique. `refs` is a JSON
    /// array column, so this is a substring match on the encoded form rather
    /// than a join — findings are few relative to problems, and a table scan
    /// beats maintaining a junction table nothing else needs.
    pub fn list_findings_referencing(&self, entity_id: &str) -> Result<Vec<Finding>> {
        Ok(self
            .list_findings()?
            .into_iter()
            .filter(|f| f.refs.iter().any(|r| r == entity_id))
            .collect())
    }
}
