//! Solution-specific storage methods.
//!
//! Generic load/save/list/next-id come from the `Persist` trait + the
//! generic methods on `MetadataStore`. This file holds the type-specific
//! helpers (`list_solutions_for_problem`) and the solution-specific
//! `delete_solution` cleanup logic.

use super::{MetadataStore, CRITIQUES_DIR};
use crate::error::{JjjError, Result};
use crate::models::Solution;
use std::fs;

impl MetadataStore {
    /// Load a solution by ID.
    pub fn load_solution(&self, solution_id: &str) -> Result<Solution> {
        self.load::<Solution>(solution_id)
    }

    /// Save a solution.
    pub fn save_solution(&self, solution: &Solution) -> Result<()> {
        self.save(solution)
    }

    /// List all solutions.
    pub fn list_solutions(&self) -> Result<Vec<Solution>> {
        self.list::<Solution>()
    }

    /// Generate the next solution ID (UUID7).
    pub fn next_solution_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Delete a solution and clean up references.
    ///
    /// This will:
    /// - Delete associated critiques
    /// - Remove the solution from its parent problem's solution_ids
    pub fn delete_solution(&self, solution_id: &str) -> Result<()> {
        let solution = self.load_solution(solution_id)?;

        // Delete associated critiques
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

        // Remove from parent problem's solution_ids
        if let Ok(mut problem) = self.load_problem(&solution.problem_id) {
            problem.remove_solution(solution_id);
            if let Err(e) = self.save_problem(&problem) {
                eprintln!(
                    "Warning: failed to update problem {}: {}",
                    solution.problem_id, e
                );
            }
        }

        self.delete_file_and_cache::<Solution>(solution_id)
    }

    /// Get solutions for a problem.
    ///
    /// Uses the SQLite cache when present (indexed query); falls back to
    /// filesystem walk when the cache is missing.
    pub fn list_solutions_for_problem(&self, problem_id: &str) -> Result<Vec<Solution>> {
        if let Some(ref db) = *self.cache() {
            let mut stmt = db
                .conn()
                .prepare("SELECT id FROM solutions WHERE problem_id = ?1 ORDER BY created_at")?;
            let ids: Vec<String> = stmt
                .query_map(rusqlite::params![problem_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut sols = Vec::with_capacity(ids.len());
            for id in ids {
                match self.load_solution(&id) {
                    Ok(s) => sols.push(s),
                    Err(JjjError::SolutionNotFound(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            return Ok(sols);
        }
        let solutions = self.list_solutions()?;
        Ok(solutions
            .into_iter()
            .filter(|s| s.problem_id == problem_id)
            .collect())
    }
}
