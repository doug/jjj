//! Milestone-specific storage methods.
//!
//! Generic load/save/list/next-id come from the `Persist` trait + the
//! generic methods on `MetadataStore`. This file holds the milestone-
//! specific `delete_milestone` cleanup logic.

use super::MetadataStore;
use crate::error::Result;
use crate::models::Milestone;

impl MetadataStore {
    /// Load a milestone by ID.
    pub fn load_milestone(&self, milestone_id: &str) -> Result<Milestone> {
        self.load::<Milestone>(milestone_id)
    }

    /// Save a milestone.
    pub fn save_milestone(&self, milestone: &Milestone) -> Result<()> {
        self.save(milestone)
    }

    /// List all milestones.
    pub fn list_milestones(&self) -> Result<Vec<Milestone>> {
        self.list::<Milestone>()
    }

    /// Generate the next milestone ID (UUID7).
    pub fn next_milestone_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Delete a milestone by ID, orphaning any associated problems.
    pub fn delete_milestone(&self, milestone_id: &str) -> Result<()> {
        let milestone = self.load_milestone(milestone_id)?;

        // Orphan any associated problems (unset milestone_id)
        for problem_id in &milestone.problem_ids {
            if let Ok(mut problem) = self.load_problem(problem_id) {
                if problem.milestone_id.as_deref() == Some(milestone_id) {
                    problem.milestone_id = None;
                    if let Err(e) = self.save_problem(&problem) {
                        eprintln!("Warning: failed to orphan problem {}: {}", problem_id, e);
                    }
                }
            }
        }

        self.delete_file_and_cache::<Milestone>(milestone_id)
    }
}
