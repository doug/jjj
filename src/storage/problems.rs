//! Problem-specific storage methods.
//!
//! Generic load/save/list/next-id come from the `Persist` trait + the
//! generic methods on `MetadataStore`. This file holds the type-specific
//! helpers (`list_subproblems`, `list_root_problems`, `parent_chain`) and
//! the problem-specific `delete_problem` cleanup logic.

use super::{MetadataStore, CRITIQUES_DIR, SOLUTIONS_DIR};
use crate::error::{JjjError, Result};
use crate::models::Problem;
use std::fs;

impl MetadataStore {
    /// Load a problem by ID.
    ///
    /// Use [`MetadataStore::list_subproblems`] to get the children of a problem;
    /// children are derived from `parent_id` references, never stored.
    pub fn load_problem(&self, problem_id: &str) -> Result<Problem> {
        self.load::<Problem>(problem_id)
    }

    /// Save a problem.
    pub fn save_problem(&self, problem: &Problem) -> Result<()> {
        self.save(problem)
    }

    /// List all problems.
    pub fn list_problems(&self) -> Result<Vec<Problem>> {
        self.list::<Problem>()
    }

    /// Generate the next problem ID (UUID7).
    pub fn next_problem_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Delete a problem and clean up references.
    ///
    /// This will:
    /// - Orphan child problems (remove their parent_id)
    /// - Delete associated solutions and their critiques
    /// - Remove the problem from its milestone
    pub fn delete_problem(&self, problem_id: &str) -> Result<()> {
        let problem = self.load_problem(problem_id)?;

        // Orphan child problems
        match self.list_subproblems(problem_id) {
            Ok(children) => {
                for child in children {
                    match self.load_problem(&child.id) {
                        Ok(mut c) => {
                            c.set_parent(None);
                            if let Err(e) = self.save_problem(&c) {
                                eprintln!(
                                    "Warning: failed to orphan child problem {}: {}",
                                    child.id, e
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: failed to load child problem {}: {}", child.id, e)
                        }
                    }
                }
            }
            Err(e) => eprintln!(
                "Warning: failed to list child problems of {}: {}",
                problem_id, e
            ),
        }

        // Delete associated solutions and their critiques
        match self.list_solutions_for_problem(problem_id) {
            Ok(solutions) => {
                for solution in solutions {
                    match self.list_critiques_for_solution(&solution.id) {
                        Ok(critiques) => {
                            for critique in critiques {
                                let path = self
                                    .meta_path()
                                    .join(CRITIQUES_DIR)
                                    .join(format!("{}.md", critique.id));
                                if let Err(e) = fs::remove_file(&path) {
                                    eprintln!(
                                        "Warning: failed to delete critique {}: {}",
                                        critique.id, e
                                    );
                                }
                            }
                        }
                        Err(e) => eprintln!(
                            "Warning: failed to list critiques for solution {}: {}",
                            solution.id, e
                        ),
                    }
                    let path = self
                        .meta_path()
                        .join(SOLUTIONS_DIR)
                        .join(format!("{}.md", solution.id));
                    if let Err(e) = fs::remove_file(&path) {
                        eprintln!("Warning: failed to delete solution {}: {}", solution.id, e);
                    }
                }
            }
            Err(e) => eprintln!(
                "Warning: failed to list solutions for problem {}: {}",
                problem_id, e
            ),
        }

        // Remove from milestone
        if let Some(ref milestone_id) = problem.milestone_id {
            match self.load_milestone(milestone_id) {
                Ok(mut milestone) => {
                    milestone.remove_problem(problem_id);
                    if let Err(e) = self.save_milestone(&milestone) {
                        eprintln!(
                            "Warning: failed to update milestone {}: {}",
                            milestone_id, e
                        );
                    }
                }
                Err(e) => eprintln!("Warning: failed to load milestone {}: {}", milestone_id, e),
            }
        }

        self.delete_file_and_cache::<Problem>(problem_id)
    }

    /// Get subproblems of a problem.
    ///
    /// Uses the SQLite cache when present (single indexed query); falls back
    /// to walking the filesystem and filtering when the cache is missing.
    pub fn list_subproblems(&self, problem_id: &str) -> Result<Vec<Problem>> {
        if let Some(ref db) = *self.cache() {
            let mut stmt = db
                .conn()
                .prepare("SELECT id FROM problems WHERE parent_id = ?1 ORDER BY created_at")?;
            let ids: Vec<String> = stmt
                .query_map(rusqlite::params![problem_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut children = Vec::with_capacity(ids.len());
            for id in ids {
                match self.load_problem(&id) {
                    Ok(p) => children.push(p),
                    Err(JjjError::ProblemNotFound(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            return Ok(children);
        }
        // Fallback: filesystem walk.
        let problems = self.list_problems()?;
        Ok(problems
            .into_iter()
            .filter(|p| p.parent_id.as_deref() == Some(problem_id))
            .collect())
    }

    /// Get root problems (problems without parents).
    ///
    /// Uses the SQLite cache when present; falls back to filesystem walk.
    pub fn list_root_problems(&self) -> Result<Vec<Problem>> {
        if let Some(ref db) = *self.cache() {
            let mut stmt = db
                .conn()
                .prepare("SELECT id FROM problems WHERE parent_id IS NULL ORDER BY created_at")?;
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut roots = Vec::with_capacity(ids.len());
            for id in ids {
                match self.load_problem(&id) {
                    Ok(p) => roots.push(p),
                    Err(JjjError::ProblemNotFound(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            return Ok(roots);
        }
        let problems = self.list_problems()?;
        Ok(problems
            .into_iter()
            .filter(|p| p.parent_id.is_none())
            .collect())
    }

    /// Get the parent chain for a problem (ancestors up to root).
    pub fn parent_chain(&self, problem_id: &str) -> Result<Vec<Problem>> {
        let mut chain = Vec::new();
        let mut current_id = Some(problem_id.to_string());
        let mut visited = std::collections::HashSet::new();
        visited.insert(problem_id.to_string());

        while let Some(id) = current_id {
            if let Ok(problem) = self.load_problem(&id) {
                current_id = problem.parent_id.clone();
                if let Some(ref next_id) = current_id {
                    if !visited.insert(next_id.clone()) {
                        eprintln!(
                            "Warning: cycle detected in problem parent chain at {}",
                            next_id
                        );
                        break;
                    }
                }
                if current_id.is_some() {
                    chain.push(problem);
                }
            } else {
                break;
            }
        }

        Ok(chain)
    }
}
