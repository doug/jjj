use super::{
    add_frontmatter_context, build_body, parse_body_sections, parse_frontmatter, to_markdown,
    MetadataStore, CRITIQUES_DIR, SOLUTIONS_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::{Solution, SolutionFrontmatter};
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
        let (frontmatter, body): (SolutionFrontmatter, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, "solution", solution_id))?;

        let sections = parse_body_sections(&body);

        let solution = Solution {
            id: frontmatter.id,
            title: frontmatter.title,
            problem_id: frontmatter.problem_id,
            status: frontmatter.status,
            critique_ids: frontmatter.critique_ids,
            change_ids: frontmatter.change_ids,
            assignee: frontmatter.assignee,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            approach: sections.get("Approach").cloned().unwrap_or_default(),
            tradeoffs: sections.get("Trade-offs").cloned().unwrap_or_default(),
            supersedes: frontmatter.supersedes,
            force_approved: frontmatter.force_approved,
            github_pr: frontmatter.github_pr,
            github_branch: frontmatter.github_branch,
        };

        Ok(solution)
    }

    /// Save a solution
    pub fn save_solution(&self, solution: &Solution) -> Result<()> {
        self.ensure_meta_checkout()?;

        let solutions_dir = self.meta_path.join(SOLUTIONS_DIR);
        fs::create_dir_all(&solutions_dir)?;

        let frontmatter = SolutionFrontmatter::from(solution);
        let body = build_body(&[
            ("Approach", &solution.approach),
            ("Trade-offs", &solution.tradeoffs),
        ]);

        let content = to_markdown(&frontmatter, &body)?;
        let solution_path = solutions_dir.join(format!("{}.md", solution.id));
        super::atomic_write(&solution_path, content.as_bytes())?;

        // Update FTS if DB exists (best-effort)
        let db_path = self.jj_client.repo_root().join(".jj").join("jjj.db");
        if db_path.exists() {
            if let Ok(db) = crate::db::schema::Database::open(&db_path) {
                let fts_body = format!("{}\n{}", solution.approach, solution.tradeoffs);
                let _ = crate::db::sync::update_fts_entry(
                    db.conn(),
                    "solution",
                    &solution.id,
                    &solution.title,
                    &fts_body,
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
                let _ = fs::remove_file(
                    self.meta_path
                        .join(CRITIQUES_DIR)
                        .join(format!("{}.md", critique.id)),
                );
            }
        }

        // Remove from parent problem's solution_ids
        if let Ok(mut problem) = self.load_problem(&solution.problem_id) {
            problem.remove_solution(solution_id);
            let _ = self.save_problem(&problem);
        }

        fs::remove_file(solution_path)?;
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

    /// Get solutions for a problem
    pub fn list_solutions_for_problem(&self, problem_id: &str) -> Result<Vec<Solution>> {
        let solutions = self.list_solutions()?;
        Ok(solutions
            .into_iter()
            .filter(|s| s.problem_id == problem_id)
            .collect())
    }

    // =========================================================================
}
