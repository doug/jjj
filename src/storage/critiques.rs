//! Critique-specific storage methods.
//!
//! Generic load/save/list/next-id come from the `Persist` trait + the
//! generic methods on `MetadataStore`. This file holds the type-specific
//! helpers (`list_critiques_for_solution`, `has_valid_critiques`, etc.)
//! and the critique-specific `delete_critique` cleanup logic.

use super::MetadataStore;
use crate::error::Result;
use crate::models::{Critique, CritiqueStatus};

impl MetadataStore {
    /// Load a critique by ID.
    pub fn load_critique(&self, critique_id: &str) -> Result<Critique> {
        self.load::<Critique>(critique_id)
    }

    /// Save a critique.
    pub fn save_critique(&self, critique: &Critique) -> Result<()> {
        self.save(critique)
    }

    /// List all critiques.
    pub fn list_critiques(&self) -> Result<Vec<Critique>> {
        self.list::<Critique>()
    }

    /// Generate the next critique ID (UUID7).
    pub fn next_critique_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Delete a critique and clean up references.
    ///
    /// Removes the critique from its parent solution's `critique_ids`.
    pub fn delete_critique(&self, critique_id: &str) -> Result<()> {
        let critique = self.load_critique(critique_id)?;

        if let Ok(mut solution) = self.load_solution(&critique.solution_id) {
            solution.remove_critique(critique_id);
            if let Err(e) = self.save_solution(&solution) {
                eprintln!(
                    "Warning: failed to update solution {}: {}",
                    critique.solution_id, e
                );
            }
        }

        self.delete_file_and_cache::<Critique>(critique_id)
    }

    /// Get critiques for a solution.
    ///
    /// Uses the SQLite cache when present; falls back to a filesystem walk.
    pub fn list_critiques_for_solution(&self, solution_id: &str) -> Result<Vec<Critique>> {
        self.query_ids_or_fallback(
            "SELECT id FROM critiques WHERE solution_id = ?1 ORDER BY created_at",
            rusqlite::params![solution_id],
            || {
                Ok(self
                    .list_critiques()?
                    .into_iter()
                    .filter(|c| c.solution_id == solution_id)
                    .collect())
            },
        )
    }

    /// Get open critiques for a solution.
    ///
    /// Loads the per-solution set and filters by status. The post-load
    /// `Open` filter guards against races where the cache row's status is
    /// `open` but the markdown has since been updated to something else.
    pub fn list_open_critiques_for_solution(&self, solution_id: &str) -> Result<Vec<Critique>> {
        let critiques = self.list_critiques_for_solution(solution_id)?;
        Ok(critiques
            .into_iter()
            .filter(|c| c.status == CritiqueStatus::Open)
            .collect())
    }

    /// Check if a solution has any valid critiques (that block approval).
    ///
    /// Uses the SQLite cache when present (single COUNT query); otherwise
    /// walks the filesystem.
    pub fn has_valid_critiques(&self, solution_id: &str) -> Result<bool> {
        if let Some(ref db) = *self.cache() {
            let count: i64 = db.conn().query_row(
                "SELECT COUNT(*) FROM critiques WHERE solution_id = ?1 AND status = 'valid'",
                rusqlite::params![solution_id],
                |row| row.get(0),
            )?;
            return Ok(count > 0);
        }
        let critiques = self.list_critiques_for_solution(solution_id)?;
        Ok(critiques.iter().any(|c| c.status == CritiqueStatus::Valid))
    }
}
