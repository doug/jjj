use super::{
    add_frontmatter_context, build_body, parse_body_sections, parse_frontmatter, to_markdown,
    MetadataStore, MILESTONES_DIR,
};
use crate::error::{JjjError, Result};
use crate::models::{Milestone, MilestoneFrontmatter};
use std::fs;

impl MetadataStore {
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
        let (frontmatter, body): (MilestoneFrontmatter, String) = parse_frontmatter(&content)
            .map_err(|e| add_frontmatter_context(e, "milestone", milestone_id))?;

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
            success_criteria: sections
                .get("Success Criteria")
                .cloned()
                .unwrap_or_default(),
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
        let mut failures = Vec::new();
        for entry in fs::read_dir(milestones_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_milestone(stem) {
                        Ok(milestone) => milestones.push(milestone),
                        Err(e) => failures.push(format!("{}: {}", stem, e)),
                    }
                }
            }
        }

        if !failures.is_empty() {
            eprintln!("Warning: Failed to load {} milestone(s):", failures.len());
            for failure in &failures {
                eprintln!("  {}", failure);
            }
        }

        Ok(milestones)
    }

    /// Generate next milestone ID using UUID7.
    pub fn next_milestone_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }

    // =========================================================================
}
