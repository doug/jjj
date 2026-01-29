use crate::error::{JjjError, Result};
use crate::jj::JjClient;
use crate::models::{
    Critique, CritiqueFrontmatter, CritiqueStatus, Milestone, MilestoneFrontmatter,
    Problem, ProblemFrontmatter, ProblemStatus,
    ProjectConfig, Solution, SolutionFrontmatter, SolutionStatus,
};
use std::fs;
use std::path::PathBuf;

const META_BOOKMARK: &str = "jjj/meta";
const CONFIG_FILE: &str = "config.toml";
const PROBLEMS_DIR: &str = "problems";
const SOLUTIONS_DIR: &str = "solutions";
const CRITIQUES_DIR: &str = "critiques";
const MILESTONES_DIR: &str = "milestones";

/// Storage layer for jjj metadata
pub struct MetadataStore {
    /// Path to the metadata directory (checked out from jjj/meta)
    meta_path: PathBuf,

    /// JJ client for interacting with the repository
    pub jj_client: JjClient,

    /// JJ client for the metadata workspace
    meta_client: JjClient,
}

// =============================================================================
// Markdown Frontmatter Parsing
// =============================================================================

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter<T: serde::de::DeserializeOwned>(content: &str) -> Result<(T, String)> {
    let content = content.trim();

    // Check for frontmatter delimiter
    if !content.starts_with("---") {
        return Err(JjjError::FrontmatterParse(
            "File must start with YAML frontmatter (---)".to_string(),
        ));
    }

    // Find the closing delimiter
    let rest = &content[3..];
    let end_pos = rest
        .find("\n---")
        .ok_or_else(|| JjjError::FrontmatterParse("Missing closing frontmatter delimiter".to_string()))?;

    let yaml_str = &rest[..end_pos].trim();
    let body = rest[end_pos + 4..].trim().to_string();

    let frontmatter: T = serde_yaml::from_str(yaml_str)?;

    Ok((frontmatter, body))
}

/// Serialize entity to markdown with YAML frontmatter
fn to_markdown<T: serde::Serialize>(frontmatter: &T, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    Ok(format!("---\n{}---\n\n{}", yaml, body))
}

/// Parse markdown body sections (## headers)
fn parse_body_sections(body: &str) -> std::collections::HashMap<String, String> {
    let mut sections = std::collections::HashMap::new();
    let mut current_section = String::new();
    let mut current_content = String::new();

    for line in body.lines() {
        if line.starts_with("## ") {
            if !current_section.is_empty() {
                sections.insert(current_section.clone(), current_content.trim().to_string());
            }
            current_section = line.strip_prefix("## ").unwrap().to_string();
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if !current_section.is_empty() {
        sections.insert(current_section, current_content.trim().to_string());
    }

    sections
}

/// Build markdown body from sections
fn build_body(sections: &[(&str, &str)]) -> String {
    sections
        .iter()
        .filter(|(_, content)| !content.is_empty())
        .map(|(header, content)| format!("## {}\n\n{}", header, content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

impl MetadataStore {
    /// Create a new metadata store
    pub fn new(jj_client: JjClient) -> Result<Self> {
        let repo_root = jj_client.repo_root().to_path_buf();
        let meta_path = repo_root.join(".jj").join("jjj-meta");

        let meta_client = JjClient::with_root(meta_path.clone())?;

        Ok(Self {
            meta_path,
            jj_client,
            meta_client,
        })
    }

    /// Initialize the metadata store (create jjj/meta bookmark)
    pub fn init(&self) -> Result<()> {
        // Check if already initialized
        if self.jj_client.bookmark_exists(META_BOOKMARK)? {
            return Err("jjj is already initialized".into());
        }

        // Create an empty orphan root
        let change_id = self.jj_client.new_empty_change("Initialize jjj metadata")?;

        // Create the bookmark
        self.jj_client.create_bookmark(META_BOOKMARK, &change_id)?;

        // Checkout the meta bookmark to create the directory structure
        self.ensure_meta_checkout()?;

        // Create initial structure
        fs::create_dir_all(self.meta_path.join(PROBLEMS_DIR))?;
        fs::create_dir_all(self.meta_path.join(SOLUTIONS_DIR))?;
        fs::create_dir_all(self.meta_path.join(CRITIQUES_DIR))?;
        fs::create_dir_all(self.meta_path.join(MILESTONES_DIR))?;

        // Create default config
        let default_config = ProjectConfig::default();
        self.save_config(&default_config)?;

        // Commit the initial structure
        self.commit_changes("Initialize jjj structure")?;

        Ok(())
    }

    /// Ensure the metadata directory is checked out
    fn ensure_meta_checkout(&self) -> Result<()> {
        if !self.meta_path.exists() {
            // Create workspace for metadata
            self.jj_client.execute(&[
                "workspace",
                "add",
                self.meta_path.to_str().unwrap(),
                "-r",
                META_BOOKMARK,
            ])?;
        }
        Ok(())
    }

    // =========================================================================
    // Config Operations
    // =========================================================================

    /// Load project configuration
    pub fn load_config(&self) -> Result<ProjectConfig> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        if !config_path.exists() {
            return Ok(ProjectConfig::default());
        }

        let content = fs::read_to_string(config_path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save project configuration
    pub fn save_config(&self, config: &ProjectConfig) -> Result<()> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        let content = toml::to_string_pretty(config)?;
        fs::write(config_path, content)?;

        Ok(())
    }

    // =========================================================================
    // Problem Operations
    // =========================================================================

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
        let (frontmatter, body): (ProblemFrontmatter, String) = parse_frontmatter(&content)?;

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

    /// Delete a problem
    pub fn delete_problem(&self, problem_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let problem_path = self
            .meta_path
            .join(PROBLEMS_DIR)
            .join(format!("{}.md", problem_id));

        if !problem_path.exists() {
            return Err(JjjError::ProblemNotFound(problem_id.to_string()));
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
        for entry in fs::read_dir(problems_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(problem) = self.load_problem(stem) {
                        problems.push(problem);
                    }
                }
            }
        }

        Ok(problems)
    }

    /// Generate next problem ID
    pub fn next_problem_id(&self) -> Result<String> {
        let problems = self.list_problems()?;

        let max_id = problems
            .iter()
            .filter_map(|p| p.id.strip_prefix("p").and_then(|s| s.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);

        Ok(format!("p{}", max_id + 1))
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
        Ok(problems.into_iter().filter(|p| p.parent_id.is_none()).collect())
    }

    /// Get the parent chain for a problem (ancestors up to root)
    pub fn get_parent_chain(&self, problem_id: &str) -> Result<Vec<Problem>> {
        let mut chain = Vec::new();
        let mut current_id = Some(problem_id.to_string());

        while let Some(id) = current_id {
            if let Ok(problem) = self.load_problem(&id) {
                current_id = problem.parent_id.clone();
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
    // Solution Operations
    // =========================================================================

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
        let (frontmatter, body): (SolutionFrontmatter, String) = parse_frontmatter(&content)?;

        let sections = parse_body_sections(&body);

        let solution = Solution {
            id: frontmatter.id,
            title: frontmatter.title,
            problem_id: frontmatter.problem_id,
            status: frontmatter.status,
            critique_ids: frontmatter.critique_ids,
            change_ids: frontmatter.change_ids,
            assignee: frontmatter.assignee,
            reviewers: frontmatter.reviewers,
            sign_offs: frontmatter.sign_offs,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            approach: sections.get("Approach").cloned().unwrap_or_default(),
            tradeoffs: sections.get("Trade-offs").cloned().unwrap_or_default(),
            supersedes: frontmatter.supersedes,
            force_accepted: frontmatter.force_accepted,
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
        fs::write(solution_path, content)?;

        Ok(())
    }

    /// Delete a solution
    pub fn delete_solution(&self, solution_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let solution_path = self
            .meta_path
            .join(SOLUTIONS_DIR)
            .join(format!("{}.md", solution_id));

        if !solution_path.exists() {
            return Err(JjjError::SolutionNotFound(solution_id.to_string()));
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
        for entry in fs::read_dir(solutions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(solution) = self.load_solution(stem) {
                        solutions.push(solution);
                    }
                }
            }
        }

        Ok(solutions)
    }

    /// Generate next solution ID
    pub fn next_solution_id(&self) -> Result<String> {
        let solutions = self.list_solutions()?;

        let max_id = solutions
            .iter()
            .filter_map(|s| s.id.strip_prefix("s").and_then(|s| s.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);

        Ok(format!("s{}", max_id + 1))
    }

    /// Get solutions for a problem
    pub fn get_solutions_for_problem(&self, problem_id: &str) -> Result<Vec<Solution>> {
        let solutions = self.list_solutions()?;
        Ok(solutions
            .into_iter()
            .filter(|s| s.problem_id == problem_id)
            .collect())
    }

    // =========================================================================
    // Critique Operations
    // =========================================================================

    /// Load a critique by ID
    pub fn load_critique(&self, critique_id: &str) -> Result<Critique> {
        self.ensure_meta_checkout()?;

        let critique_path = self
            .meta_path
            .join(CRITIQUES_DIR)
            .join(format!("{}.md", critique_id));

        if !critique_path.exists() {
            return Err(JjjError::CritiqueNotFound(critique_id.to_string()));
        }

        let content = fs::read_to_string(critique_path)?;
        let (frontmatter, body): (CritiqueFrontmatter, String) = parse_frontmatter(&content)?;

        let sections = parse_body_sections(&body);

        let critique = Critique {
            id: frontmatter.id,
            title: frontmatter.title,
            solution_id: frontmatter.solution_id,
            status: frontmatter.status,
            severity: frontmatter.severity,
            author: frontmatter.author,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            argument: sections.get("Argument").cloned().unwrap_or_default(),
            evidence: sections.get("Evidence").cloned().unwrap_or_default(),
            file_path: frontmatter.file_path,
            line_start: frontmatter.line_start,
            line_end: frontmatter.line_end,
            code_context: Vec::new(),
            replies: Vec::new(),
        };

        Ok(critique)
    }

    /// Save a critique
    pub fn save_critique(&self, critique: &Critique) -> Result<()> {
        self.ensure_meta_checkout()?;

        let critiques_dir = self.meta_path.join(CRITIQUES_DIR);
        fs::create_dir_all(&critiques_dir)?;

        let frontmatter = CritiqueFrontmatter::from(critique);
        let body = build_body(&[
            ("Argument", &critique.argument),
            ("Evidence", &critique.evidence),
        ]);

        let content = to_markdown(&frontmatter, &body)?;
        let critique_path = critiques_dir.join(format!("{}.md", critique.id));
        fs::write(critique_path, content)?;

        Ok(())
    }

    /// Delete a critique
    pub fn delete_critique(&self, critique_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let critique_path = self
            .meta_path
            .join(CRITIQUES_DIR)
            .join(format!("{}.md", critique_id));

        if !critique_path.exists() {
            return Err(JjjError::CritiqueNotFound(critique_id.to_string()));
        }

        fs::remove_file(critique_path)?;
        Ok(())
    }

    /// List all critiques
    pub fn list_critiques(&self) -> Result<Vec<Critique>> {
        self.ensure_meta_checkout()?;

        let critiques_dir = self.meta_path.join(CRITIQUES_DIR);
        if !critiques_dir.exists() {
            return Ok(Vec::new());
        }

        let mut critiques = Vec::new();
        for entry in fs::read_dir(critiques_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(critique) = self.load_critique(stem) {
                        critiques.push(critique);
                    }
                }
            }
        }

        Ok(critiques)
    }

    /// Generate next critique ID
    pub fn next_critique_id(&self) -> Result<String> {
        let critiques = self.list_critiques()?;

        let max_id = critiques
            .iter()
            .filter_map(|c| c.id.strip_prefix("c").and_then(|s| s.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);

        Ok(format!("c{}", max_id + 1))
    }

    /// Get critiques for a solution
    pub fn get_critiques_for_solution(&self, solution_id: &str) -> Result<Vec<Critique>> {
        let critiques = self.list_critiques()?;
        Ok(critiques
            .into_iter()
            .filter(|c| c.solution_id == solution_id)
            .collect())
    }

    /// Get open critiques for a solution
    pub fn get_open_critiques_for_solution(&self, solution_id: &str) -> Result<Vec<Critique>> {
        let critiques = self.get_critiques_for_solution(solution_id)?;
        Ok(critiques
            .into_iter()
            .filter(|c| c.status == CritiqueStatus::Open)
            .collect())
    }

    /// Check if a solution has any valid critiques (that would refute it)
    pub fn has_valid_critiques(&self, solution_id: &str) -> Result<bool> {
        let critiques = self.get_critiques_for_solution(solution_id)?;
        Ok(critiques.iter().any(|c| c.status == CritiqueStatus::Valid))
    }

    // =========================================================================
    // Milestone Operations
    // =========================================================================

    /// Load a milestone by ID
    pub fn load_milestone(&self, milestone_id: &str) -> Result<Milestone> {
        self.ensure_meta_checkout()?;

        let milestone_path = self
            .meta_path
            .join(MILESTONES_DIR)
            .join(format!("{}.md", milestone_id));

        if !milestone_path.exists() {
            return Err(JjjError::MilestoneNotFound(milestone_id.to_string()));
        }

        let content = fs::read_to_string(milestone_path)?;
        let (frontmatter, body): (MilestoneFrontmatter, String) = parse_frontmatter(&content)?;

        let sections = parse_body_sections(&body);

        let milestone = Milestone {
            id: frontmatter.id,
            title: frontmatter.title,
            target_date: frontmatter.target_date,
            status: frontmatter.status,
            problem_ids: frontmatter.problem_ids,
            assignee: frontmatter.assignee,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            goals: sections.get("Goals").cloned().unwrap_or_default(),
            success_criteria: sections.get("Success Criteria").cloned().unwrap_or_default(),
        };

        Ok(milestone)
    }

    /// Save a milestone
    pub fn save_milestone(&self, milestone: &Milestone) -> Result<()> {
        self.ensure_meta_checkout()?;

        let milestones_dir = self.meta_path.join(MILESTONES_DIR);
        fs::create_dir_all(&milestones_dir)?;

        let frontmatter = MilestoneFrontmatter::from(milestone);
        let body = build_body(&[
            ("Goals", &milestone.goals),
            ("Success Criteria", &milestone.success_criteria),
        ]);

        let content = to_markdown(&frontmatter, &body)?;
        let milestone_path = milestones_dir.join(format!("{}.md", milestone.id));
        fs::write(milestone_path, content)?;

        Ok(())
    }

    /// List all milestones
    pub fn list_milestones(&self) -> Result<Vec<Milestone>> {
        self.ensure_meta_checkout()?;

        let milestones_dir = self.meta_path.join(MILESTONES_DIR);
        if !milestones_dir.exists() {
            return Ok(Vec::new());
        }

        let mut milestones = Vec::new();
        for entry in fs::read_dir(milestones_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(milestone) = self.load_milestone(stem) {
                        milestones.push(milestone);
                    }
                }
            }
        }

        Ok(milestones)
    }

    /// Generate next milestone ID
    pub fn next_milestone_id(&self) -> Result<String> {
        let milestones = self.list_milestones()?;

        let max_id = milestones
            .iter()
            .filter_map(|m| m.id.strip_prefix("m").and_then(|s| s.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);

        Ok(format!("m{}", max_id + 1))
    }

    // =========================================================================
    // High-Level Operations
    // =========================================================================

    /// Check if a problem can be marked as solved
    /// Requires: at least one accepted solution OR all subproblems solved
    pub fn can_solve_problem(&self, problem_id: &str) -> Result<(bool, String)> {
        let problem = self.load_problem(problem_id)?;

        // Check if already solved
        if problem.status == ProblemStatus::Solved {
            return Ok((false, "Problem is already solved".to_string()));
        }

        // Check for accepted solutions
        let solutions = self.get_solutions_for_problem(problem_id)?;
        let has_accepted = solutions.iter().any(|s| s.status == SolutionStatus::Accepted);

        if has_accepted {
            return Ok((true, String::new()));
        }

        // Check if all subproblems are solved
        let subproblems = self.get_subproblems(problem_id)?;
        if !subproblems.is_empty() {
            let all_solved = subproblems.iter().all(|p| p.status == ProblemStatus::Solved);
            if all_solved {
                return Ok((true, "All subproblems are solved".to_string()));
            }
            return Ok((
                false,
                "Not all subproblems are solved and no accepted solution exists".to_string(),
            ));
        }

        Ok((false, "No accepted solution exists".to_string()))
    }

    /// Check if a solution can be accepted
    /// Requires: no valid critiques
    pub fn can_accept_solution(&self, solution_id: &str) -> Result<(bool, String)> {
        let solution = self.load_solution(solution_id)?;

        // Check if already finalized
        if solution.is_finalized() {
            return Ok((
                false,
                format!("Solution is already {:?}", solution.status),
            ));
        }

        // Check for valid critiques
        if self.has_valid_critiques(solution_id)? {
            return Ok((false, "Solution has valid critiques that refute it".to_string()));
        }

        // Check for open critiques (warning but not blocking)
        let open_critiques = self.get_open_critiques_for_solution(solution_id)?;
        if !open_critiques.is_empty() {
            return Ok((
                true,
                format!(
                    "Warning: {} open critique(s) remain unaddressed",
                    open_critiques.len()
                ),
            ));
        }

        Ok((true, String::new()))
    }

    // =========================================================================
    // Commit Operations
    // =========================================================================

    /// Commit changes to the metadata
    fn commit_changes(&self, message: &str) -> Result<()> {
        // Create a new change in the metadata workspace
        self.meta_client.new_empty_change(message)?;

        // Update the bookmark to point to the new change
        let meta_change = self.meta_client.current_change_id()?;

        // Update bookmark in the main repo (bookmarks are shared)
        self.jj_client
            .execute(&["bookmark", "set", META_BOOKMARK, "-r", &meta_change])?;

        Ok(())
    }

    /// Perform an operation on the metadata and commit
    pub fn with_metadata<F, R>(&self, message: &str, operation: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        let result = operation()?;
        self.commit_changes(message)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Priority;
    use chrono::Utc;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
id: p1
title: Test Problem
status: open
priority: medium
created_at: 2024-01-15T10:30:00Z
updated_at: 2024-01-15T10:30:00Z
---

## Description

This is a test problem.

## Context

Some context here.
"#;

        let (frontmatter, body): (ProblemFrontmatter, String) = parse_frontmatter(content).unwrap();
        assert_eq!(frontmatter.id, "p1");
        assert_eq!(frontmatter.title, "Test Problem");
        assert!(body.contains("## Description"));
    }

    #[test]
    fn test_parse_body_sections() {
        let body = r#"## Description

This is the description.

## Context

This is the context.
"#;

        let sections = parse_body_sections(body);
        assert_eq!(sections.get("Description").unwrap(), "This is the description.");
        assert_eq!(sections.get("Context").unwrap(), "This is the context.");
    }

    #[test]
    fn test_to_markdown() {
        let frontmatter = ProblemFrontmatter {
            id: "p1".to_string(),
            title: "Test".to_string(),
            parent_id: None,
            status: ProblemStatus::Open,
            priority: Priority::default(),
            solution_ids: vec![],
            child_ids: vec![],
            milestone_id: None,
            assignee: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            dissolved_reason: None,
        };

        let body = "## Description\n\nTest description";
        let result = to_markdown(&frontmatter, body).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("id: p1"));
        assert!(result.contains("## Description"));
    }

    #[test]
    fn test_build_body() {
        let sections = vec![
            ("Description", "Test description"),
            ("Context", "Test context"),
            ("Empty", ""),
        ];

        let body = build_body(&sections);
        assert!(body.contains("## Description"));
        assert!(body.contains("Test description"));
        assert!(body.contains("## Context"));
        assert!(!body.contains("## Empty")); // Empty sections are skipped
    }
}
