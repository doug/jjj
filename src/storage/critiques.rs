use super::{
    add_frontmatter_context, build_body, parse_body_sections, parse_frontmatter, to_markdown,
    MetadataStore, CRITIQUES_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::{Critique, CritiqueFrontmatter, CritiqueStatus};
use std::fs;

impl MetadataStore {
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
        let (frontmatter, body): (CritiqueFrontmatter, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, "critique", critique_id))?;

        let sections = parse_body_sections(&body);

        let critique = Critique {
            id: frontmatter.id,
            title: frontmatter.title,
            solution_id: frontmatter.solution_id,
            status: frontmatter.status,
            severity: frontmatter.severity,
            author: frontmatter.author,
            reviewer: frontmatter.reviewer,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            argument: sections.get("Argument").cloned().unwrap_or_default(),
            evidence: sections.get("Evidence").cloned().unwrap_or_default(),
            file_path: frontmatter.file_path,
            line_start: frontmatter.line_start,
            line_end: frontmatter.line_end,
            code_context: Vec::new(),
            replies: Vec::new(),
            github_review_id: frontmatter.github_review_id,
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

    /// Delete a critique and clean up references.
    ///
    /// This will remove the critique from its parent solution's critique_ids.
    pub fn delete_critique(&self, critique_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let critique_path = self
            .meta_path
            .join(CRITIQUES_DIR)
            .join(format!("{}.md", critique_id));

        if !critique_path.exists() {
            return Err(JjjError::CritiqueNotFound(critique_id.to_string()));
        }

        // Remove from parent solution's critique_ids
        let critique = self.load_critique(critique_id)?;
        if let Ok(mut solution) = self.load_solution(&critique.solution_id) {
            solution.remove_critique(critique_id);
            let _ = self.save_solution(&solution);
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
        let mut failures = Vec::new();
        for entry in fs::read_dir(critiques_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_critique(stem) {
                        Ok(critique) => critiques.push(critique),
                        Err(e) => failures.push(format!("{}: {}", stem, e)),
                    }
                }
            }
        }

        if !failures.is_empty() {
            eprintln!("Warning: Failed to load {} critique(s):", failures.len());
            for failure in &failures {
                eprintln!("  {}", failure);
            }
        }

        Ok(critiques)
    }

    /// Generate next critique ID using UUID7.
    pub fn next_critique_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
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
}
