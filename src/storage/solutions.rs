//! Solution-specific storage methods.
//!
//! Generic load/save/list/next-id come from the `Persist` trait + the
//! generic methods on `MetadataStore`. This file holds the type-specific
//! helpers (`list_solutions_for_problem`) and the solution-specific
//! `delete_solution` cleanup logic.

use super::{MetadataStore, CRITIQUES_DIR};
use crate::error::Result;
use crate::models::{Critique, Solution};
use std::fs;

impl MetadataStore {
    /// Load a solution by ID, with the derived `critique_ids` attached.
    pub fn load_solution(&self, solution_id: &str) -> Result<Solution> {
        let mut solution = self.load::<Solution>(solution_id)?;
        solution.critique_ids = self.reverse_ids_for::<Critique, _>(
            "SELECT id FROM critiques WHERE solution_id = ?1",
            solution_id,
            |c| c.solution_id == solution_id,
        )?;
        Ok(solution)
    }

    /// Save a solution.
    pub fn save_solution(&self, solution: &Solution) -> Result<()> {
        self.save(solution)
    }

    /// List all solutions, with the derived `critique_ids` back-reference
    /// attached (Pillar 4 — derived from `Critique::solution_id`).
    pub fn list_solutions(&self) -> Result<Vec<Solution>> {
        let mut solutions = self.list::<Solution>()?;
        let mut by_solution = self
            .reverse_ids_batch::<Critique, _>("SELECT solution_id, id FROM critiques", |c| {
                c.solution_id.clone()
            })?;
        for solution in &mut solutions {
            solution.critique_ids = by_solution.remove(&solution.id).unwrap_or_default();
        }
        Ok(solutions)
    }

    /// Generate the next solution ID (UUID7).
    pub fn next_solution_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Delete a solution and its critiques.
    ///
    /// The parent problem's `solution_ids` is a derived back-reference (Pillar
    /// 4), so removing the solution file is all that's needed — there is no
    /// parent to rewrite.
    pub fn delete_solution(&self, solution_id: &str) -> Result<()> {
        // Delete associated critiques (their forward ref points here).
        if let Ok(critiques) = self.list_critiques_for_solution(solution_id) {
            for critique in critiques {
                if let Err(e) = fs::remove_file(
                    self.meta_path()
                        .join(CRITIQUES_DIR)
                        .join(format!("{}.md", critique.id)),
                ) {
                    eprintln!("Warning: failed to delete critique {}: {}", critique.id, e);
                }
            }
        }

        self.delete_file_and_cache::<Solution>(solution_id)
    }

    /// Get solutions for a problem.
    ///
    /// Uses the SQLite cache when present; falls back to a filesystem walk.
    pub fn list_solutions_for_problem(&self, problem_id: &str) -> Result<Vec<Solution>> {
        self.query_ids_or_fallback(
            "SELECT id FROM solutions WHERE problem_id = ?1 ORDER BY created_at",
            rusqlite::params![problem_id],
            || {
                Ok(self
                    .list_solutions()?
                    .into_iter()
                    .filter(|s| s.problem_id == problem_id)
                    .collect())
            },
        )
    }
}
