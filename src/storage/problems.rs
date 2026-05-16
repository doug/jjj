use super::{
    add_frontmatter_context, parse_frontmatter, to_markdown_strip, MetadataStore, CRITIQUES_DIR,
    PROBLEMS_DIR, SOLUTIONS_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::Problem;
use std::fs;

impl MetadataStore {
    /// Load a problem by ID.
    ///
    /// Use [`MetadataStore::list_subproblems`] to get the children of a problem;
    /// children are derived from `parent_id` references, never stored.
    pub fn load_problem(&self, problem_id: &str) -> Result<Problem> {
        self.ensure_meta_checkout()?;

        let problem_path = self
            .meta_path
            .join(PROBLEMS_DIR)
            .join(format!("{}.md", problem_id));

        if !problem_path.exists() {
            return Err(JjjError::ProblemNotFound(problem_id.to_string()));
        }

        let content = fs::read_to_string(problem_path)?;
        let (mut problem, body): (Problem, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, "problem", problem_id))?;
        problem.description = body;

        Ok(problem)
    }

    /// Save a problem
    pub fn save_problem(&self, problem: &Problem) -> Result<()> {
        self.ensure_meta_checkout()?;

        let problems_dir = self.meta_path.join(PROBLEMS_DIR);
        fs::create_dir_all(&problems_dir)?;

        let body = if problem.description.is_empty() {
            String::new()
        } else {
            format!("{}\n", problem.description)
        };

        let content = to_markdown_strip(problem, &body, "description")?;
        let problem_path = problems_dir.join(format!("{}.md", problem.id));
        super::atomic_write(&problem_path, content.as_bytes())?;

        // Best-effort cache sync; the markdown is canonical.
        if let Some(ref db) = *self.cache() {
            if let Err(e) = crate::db::sync::sync_problem_to_cache(db, problem) {
                eprintln!(
                    "Warning: cache sync failed for problem {}: {}",
                    problem.id, e
                );
            }
        }

        Ok(())
    }

    /// Delete a problem and clean up references.
    ///
    /// This will:
    /// - Orphan child problems (remove their parent_id)
    /// - Delete associated solutions and their critiques
    /// - Remove the problem from its milestone
    pub fn delete_problem(&self, problem_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let problem_path = self
            .meta_path
            .join(PROBLEMS_DIR)
            .join(format!("{}.md", problem_id));

        if !problem_path.exists() {
            return Err(JjjError::ProblemNotFound(problem_id.to_string()));
        }

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
                                    .meta_path
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
                        .meta_path
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

        fs::remove_file(problem_path)?;

        // Best-effort cache removal
        if let Some(ref db) = *self.cache() {
            if let Err(e) = crate::db::sync::remove_entity_from_cache(db, "problem", problem_id) {
                eprintln!(
                    "Warning: cache removal failed for problem {}: {}",
                    problem_id, e
                );
            }
        }

        Ok(())
    }

    /// List all problems
    pub fn list_problems(&self) -> Result<Vec<Problem>> {
        self.ensure_meta_checkout()?;

        let problems_dir = self.meta_path.join(PROBLEMS_DIR);
        if !problems_dir.exists() {
            return Ok(Vec::new());
        }

        let mut problems = Vec::new();
        let mut failures = Vec::new();
        for entry in fs::read_dir(problems_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_problem(stem) {
                        Ok(problem) => problems.push(problem),
                        Err(e) => failures.push(format!("{}: {}", stem, e)),
                    }
                }
            }
        }

        if !failures.is_empty() {
            eprintln!("Warning: Failed to load {} problem(s):", failures.len());
            for failure in &failures {
                eprintln!("  {}", failure);
            }
        }

        Ok(problems)
    }

    /// Generate next problem ID using UUID7.
    pub fn next_problem_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
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

    /// Get the parent chain for a problem (ancestors up to root)
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

    // =========================================================================
}
