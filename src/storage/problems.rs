use super::{
    add_frontmatter_context, build_body, parse_body_sections, parse_frontmatter, to_markdown,
    MetadataStore, CRITIQUES_DIR, PROBLEMS_DIR, SOLUTIONS_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::{Problem, ProblemFrontmatter};
use std::fs;

impl MetadataStore {
    /// Load a problem by ID.
    ///
    /// **Note:** `child_ids` is always empty on the returned value because
    /// children are derived from `parent_id` references across all problems.
    /// Use [`MetadataStore::list_subproblems`] to get the children of a problem.
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
            tags: frontmatter.tags,
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
        super::atomic_write(&problem_path, content.as_bytes())?;

        // Update FTS if DB exists (best-effort)
        let db_path = self.jj_client.repo_root().join(".jj").join("jjj.db");
        if db_path.exists() {
            if let Ok(db) = crate::db::schema::Database::open(&db_path) {
                let body = format!("{}\n{}", problem.description, problem.tags.join(" "));
                let _ = crate::db::sync::update_fts_entry(
                    db.conn(),
                    "problem",
                    &problem.id,
                    &problem.title,
                    &body,
                );
            }
        }

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

        // Derive child_ids from parent_id references (child_ids is not stored on disk).
        let mut child_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for p in &problems {
            if let Some(ref pid) = p.parent_id {
                child_map.entry(pid.clone()).or_default().push(p.id.clone());
            }
        }
        for p in &mut problems {
            p.child_ids = child_map.remove(&p.id).unwrap_or_default();
        }

        Ok(problems)
    }

    /// Generate next problem ID using UUID7.
    pub fn next_problem_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    /// Get subproblems of a problem
    pub fn list_subproblems(&self, problem_id: &str) -> Result<Vec<Problem>> {
        let problems = self.list_problems()?;
        Ok(problems
            .into_iter()
            .filter(|p| p.parent_id.as_deref() == Some(problem_id))
            .collect())
    }

    /// Get root problems (problems without parents)
    pub fn list_root_problems(&self) -> Result<Vec<Problem>> {
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
