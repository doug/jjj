use super::{
    add_frontmatter_context, parse_frontmatter, to_markdown_strip, MetadataStore, CRITIQUES_DIR,
    SOLUTIONS_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::Solution;
use std::fs;

impl MetadataStore {
    /// Load a solution by ID
    pub fn load_solution(&self, solution_id: &str) -> Result<Solution> {
        self.ensure_meta_checkout()?;

        let solution_path = self
            .meta_path
            .join(SOLUTIONS_DIR)
            .join(format!("{}.md", solution_id));

        if !solution_path.exists() {
            return Err(JjjError::SolutionNotFound(solution_id.to_string()));
        }

        let content = fs::read_to_string(solution_path)?;
        let (mut solution, body): (Solution, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, "solution", solution_id))?;
        solution.approach = body;

        Ok(solution)
    }

    /// Save a solution
    pub fn save_solution(&self, solution: &Solution) -> Result<()> {
        self.ensure_meta_checkout()?;

        let solutions_dir = self.meta_path.join(SOLUTIONS_DIR);
        fs::create_dir_all(&solutions_dir)?;

        let body = if solution.approach.is_empty() {
            String::new()
        } else {
            format!("{}\n", solution.approach)
        };

        let content = to_markdown_strip(solution, &body, "approach")?;
        let solution_path = solutions_dir.join(format!("{}.md", solution.id));
        super::atomic_write(&solution_path, content.as_bytes())?;

        // Best-effort cache sync; the markdown is canonical.
        if let Some(ref db) = *self.cache() {
            if let Err(e) = crate::db::sync::sync_solution_to_cache(db, solution) {
                eprintln!(
                    "Warning: cache sync failed for solution {}: {}",
                    solution.id, e
                );
            }
        }

        Ok(())
    }

    /// Delete a solution and clean up references.
    ///
    /// This will:
    /// - Delete associated critiques
    /// - Remove the solution from its parent problem's solution_ids
    pub fn delete_solution(&self, solution_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let solution_path = self
            .meta_path
            .join(SOLUTIONS_DIR)
            .join(format!("{}.md", solution_id));

        if !solution_path.exists() {
            return Err(JjjError::SolutionNotFound(solution_id.to_string()));
        }

        let solution = self.load_solution(solution_id)?;

        // Delete associated critiques
        if let Ok(critiques) = self.list_critiques_for_solution(solution_id) {
            for critique in critiques {
                if let Err(e) = fs::remove_file(
                    self.meta_path
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

        fs::remove_file(solution_path)?;

        // Best-effort cache removal
        if let Some(ref db) = *self.cache() {
            if let Err(e) = crate::db::sync::remove_entity_from_cache(db, "solution", solution_id) {
                eprintln!(
                    "Warning: cache removal failed for solution {}: {}",
                    solution_id, e
                );
            }
        }

        Ok(())
    }

    /// List all solutions
    pub fn list_solutions(&self) -> Result<Vec<Solution>> {
        self.ensure_meta_checkout()?;

        let solutions_dir = self.meta_path.join(SOLUTIONS_DIR);
        if !solutions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut solutions = Vec::new();
        let mut failures = Vec::new();
        for entry in fs::read_dir(solutions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_solution(stem) {
                        Ok(solution) => solutions.push(solution),
                        Err(e) => failures.push(format!("{}: {}", stem, e)),
                    }
                }
            }
        }

        if !failures.is_empty() {
            eprintln!("Warning: Failed to load {} solution(s):", failures.len());
            for failure in &failures {
                eprintln!("  {}", failure);
            }
        }

        Ok(solutions)
    }

    /// Generate next solution ID using UUID7.
    pub fn next_solution_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
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

    // =========================================================================
}
