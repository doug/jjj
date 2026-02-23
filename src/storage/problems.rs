use super::{
    add_frontmatter_context, build_body, parse_body_sections, parse_frontmatter, to_markdown,
    MetadataStore, CRITIQUES_DIR, PROBLEMS_DIR, SOLUTIONS_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::{Problem, ProblemFrontmatter};
use std::fs;

impl MetadataStore {
    /// Load a problem by ID
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
        let (frontmatter, body): (ProblemFrontmatter, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, "problem", problem_id))?;

        // Parse body sections
        let sections = parse_body_sections(&body);

        let problem = Problem {
            id: frontmatter.id,
            title: frontmatter.title,
            parent_id: frontmatter.parent_id,
            status: frontmatter.status,
            priority: frontmatter.priority,
            solution_ids: frontmatter.solution_ids,
            child_ids: frontmatter.child_ids,
            milestone_id: frontmatter.milestone_id,
            assignee: frontmatter.assignee,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            description: sections.get("Description").cloned().unwrap_or_default(),
            context: sections.get("Context").cloned().unwrap_or_default(),
            dissolved_reason: frontmatter.dissolved_reason,
            github_issue: frontmatter.github_issue,
        };

        Ok(problem)
    }

    /// Save a problem
    pub fn save_problem(&self, problem: &Problem) -> Result<()> {
        self.ensure_meta_checkout()?;

        let problems_dir = self.meta_path.join(PROBLEMS_DIR);
        fs::create_dir_all(&problems_dir)?;

        let frontmatter = ProblemFrontmatter::from(problem);
        let body = build_body(&[
            ("Description", &problem.description),
            ("Context", &problem.context),
        ]);

        let content = to_markdown(&frontmatter, &body)?;
        let problem_path = problems_dir.join(format!("{}.md", problem.id));
        fs::write(problem_path, content)?;

        Ok(())
    }

    /// Delete a problem and clean up references.
    ///
    /// This will:
    /// - Orphan child problems (remove their parent_id)
    /// - Remove the problem from its parent's child_ids
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
        if let Ok(children) = self.get_subproblems(problem_id) {
            for child in children {
                if let Ok(mut c) = self.load_problem(&child.id) {
                    c.set_parent(None);
                    let _ = self.save_problem(&c);
                }
            }
        }

        // Remove from parent's child_ids
        if let Some(ref parent_id) = problem.parent_id {
            if let Ok(mut parent) = self.load_problem(parent_id) {
                parent.remove_child(problem_id);
                let _ = self.save_problem(&parent);
            }
        }

        // Delete associated solutions and their critiques
        if let Ok(solutions) = self.get_solutions_for_problem(problem_id) {
            for solution in solutions {
                if let Ok(critiques) = self.get_critiques_for_solution(&solution.id) {
                    for critique in critiques {
                        let _ = fs::remove_file(
                            self.meta_path
                                .join(CRITIQUES_DIR)
                                .join(format!("{}.md", critique.id)),
                        );
                    }
                }
                let _ = fs::remove_file(
                    self.meta_path
                        .join(SOLUTIONS_DIR)
                        .join(format!("{}.md", solution.id)),
                );
            }
        }

        // Remove from milestone
        if let Some(ref milestone_id) = problem.milestone_id {
            if let Ok(mut milestone) = self.load_milestone(milestone_id) {
                milestone.remove_problem(problem_id);
                let _ = self.save_milestone(&milestone);
            }
        }

        fs::remove_file(problem_path)?;
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

    /// Get subproblems of a problem
    pub fn get_subproblems(&self, problem_id: &str) -> Result<Vec<Problem>> {
        let problems = self.list_problems()?;
        Ok(problems
            .into_iter()
            .filter(|p| p.parent_id.as_deref() == Some(problem_id))
            .collect())
    }

    /// Get root problems (problems without parents)
    pub fn get_root_problems(&self) -> Result<Vec<Problem>> {
        let problems = self.list_problems()?;
        Ok(problems
            .into_iter()
            .filter(|p| p.parent_id.is_none())
            .collect())
    }

    /// Get the parent chain for a problem (ancestors up to root)
    pub fn get_parent_chain(&self, problem_id: &str) -> Result<Vec<Problem>> {
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
