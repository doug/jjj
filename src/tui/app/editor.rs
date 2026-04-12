use super::App;
use crate::display::short_id;
use crate::error::Result;
use ratatui::backend::Backend;
use ratatui::Terminal;

use super::super::next_actions::EntityType;
use super::EditorRequest;

/// Parsed frontmatter + body from an editor document.
#[derive(Debug)]
pub(crate) struct ParsedEditorContent {
    pub title: String,
    pub tags: Vec<String>,
    pub description: String,
    pub fields: std::collections::HashMap<String, String>,
}

/// Parse a `---\nfrontmatter\n---\nbody` document into structured fields.
pub(crate) fn parse_editor_content(
    content: &str,
) -> std::result::Result<ParsedEditorContent, String> {
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err("Invalid format: missing --- delimiters".to_string());
    }

    let frontmatter = parts[1].trim();
    let body = parts[2].trim().to_string();

    // Fields where inline comments should be stripped (enum/date fields only).
    // Freeform fields like title and tags are left as-is to preserve # in text.
    const COMMENT_FIELDS: &[&str] = &[
        "status",
        "priority",
        "confidence",
        "severity",
        "target_date",
    ];

    // Parse all frontmatter fields into a map
    let mut fields = std::collections::HashMap::new();
    for line in frontmatter.lines() {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let raw_value = line[colon_pos + 1..].trim();
            let value = if COMMENT_FIELDS.contains(&key.as_str()) {
                // Strip YAML-style inline comment (# ...) for enum fields
                raw_value
                    .find(" #")
                    .map(|i| raw_value[..i].trim())
                    .unwrap_or(raw_value)
                    .to_string()
            } else {
                raw_value.to_string()
            };
            if !value.is_empty() {
                fields.insert(key, value);
            }
        }
    }

    let title = fields
        .get("title")
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();
    if title.is_empty() {
        return Err("title is required".to_string());
    }

    let tags: Vec<String> = fields
        .get("tags")
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Body after frontmatter is the description (no ## header needed)
    let description = body.clone();

    Ok(ParsedEditorContent {
        title,
        tags,
        description,
        fields,
    })
}

impl App {
    /// Initiate editing the selected entity in an external editor.
    ///
    /// Serializes the entity to a temp file, then sets `editor_request` to signal
    /// the main loop to suspend the TUI, run `$VISUAL` / `$EDITOR` / `vi`, and
    /// resume. On resume, the edited content is diffed against the original and
    /// saved if changed.
    pub(super) fn open_in_editor(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

        // Get selected entity
        let (entity_type, entity_id) =
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, .. } => (EntityType::Problem, id.clone()),
                    TreeNode::Solution { id, .. } => (EntityType::Solution, id.clone()),
                    TreeNode::Critique { id, .. } => (EntityType::Critique, id.clone()),
                    TreeNode::Milestone { id, .. } => (EntityType::Milestone, id.clone()),
                    _ => {
                        self.show_flash("Cannot edit this item type");
                        return Ok(());
                    }
                }
            } else {
                self.show_flash("No item selected");
                return Ok(());
            };

        // Serialize entity to temp file
        let temp_path = std::env::temp_dir().join(format!("jjj-edit-{}.md", short_id(&entity_id)));
        let original_content = match self.serialize_entity_for_edit(&entity_type, &entity_id) {
            Ok(content) => content,
            Err(e) => {
                self.show_flash(&format!("Load error: {}", e));
                return Ok(());
            }
        };

        if let Err(e) = std::fs::write(&temp_path, &original_content) {
            self.show_flash(&format!("Write error: {}", e));
            return Ok(());
        }

        // Get editor
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());

        // Signal that we need to suspend
        self.editor_request = Some(EditorRequest {
            entity_type,
            entity_id,
            temp_path,
            original_content,
            editor,
        });

        Ok(())
    }

    /// Render an entity as a markdown document with YAML frontmatter for editing.
    ///
    /// The format is intentionally minimal: just the fields users are likely to
    /// want to change (title, status, priority/severity, and the main text body).
    /// `apply_edited_content()` parses this format back after the editor exits.
    fn serialize_entity_for_edit(
        &self,
        entity_type: &EntityType,
        entity_id: &str,
    ) -> Result<String> {
        match entity_type {
            EntityType::Problem => {
                let problem = self.store.load_problem(entity_id)?;
                let tags_line = if problem.tags.is_empty() {
                    "tags: \n".to_string()
                } else {
                    format!("tags: {}\n", problem.tags.join(", "))
                };
                Ok(format!(
                    "---\ntitle: {}\nstatus: {} # open, in_progress, solved, dissolved\npriority: {} # critical, high, medium, low\nconfidence: {} # unknown, red, amber, green\n{}---\n\n{}\n",
                    problem.title,
                    problem.status,
                    problem.priority,
                    problem.confidence,
                    tags_line,
                    problem.description,
                ))
            }
            EntityType::Solution => {
                let solution = self.store.load_solution(entity_id)?;
                let tags_line = if solution.tags.is_empty() {
                    "tags: \n".to_string()
                } else {
                    format!("tags: {}\n", solution.tags.join(", "))
                };
                Ok(format!(
                    "---\ntitle: {}\nstatus: {} # proposed, submitted, approved, withdrawn\n{}---\n\n{}\n",
                    solution.title,
                    solution.status,
                    tags_line,
                    solution.approach,
                ))
            }
            EntityType::Critique => {
                let critique = self.store.load_critique(entity_id)?;
                Ok(format!(
                    "---\ntitle: {}\nstatus: {} # open, addressed, valid, dismissed\nseverity: {} # critical, high, medium, low\n---\n\n{}\n",
                    critique.title,
                    critique.status,
                    critique.severity,
                    critique.argument,
                ))
            }
            EntityType::Milestone => {
                let milestone = self.store.load_milestone(entity_id)?;
                let target_date_line = match &milestone.target_date {
                    Some(d) => format!("target_date: {} # YYYY-MM-DD\n", d.format("%Y-%m-%d")),
                    None => "target_date:  # YYYY-MM-DD\n".to_string(),
                };
                Ok(format!(
                    "---\ntitle: {}\nstatus: {} # planning, active, completed, cancelled\n{}---\n\n{}\n",
                    milestone.title,
                    milestone.status,
                    target_date_line,
                    milestone.description,
                ))
            }
        }
    }

    pub(super) fn run_editor<B: Backend + std::io::Write>(
        &mut self,
        terminal: &mut Terminal<B>,
        request: EditorRequest,
    ) -> Result<()> {
        use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
        use crossterm::execute;
        use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
        use std::process::Command;

        // Leave alternate screen
        crossterm::terminal::disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;

        // Run editor
        let status = Command::new(&request.editor)
            .arg(&request.temp_path)
            .status();

        // Re-enter alternate screen
        crossterm::terminal::enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        terminal.clear()?;

        // Process result
        match status {
            Ok(exit_status) if exit_status.success() => {
                match std::fs::read_to_string(&request.temp_path) {
                    Err(e) => self.show_flash(&format!("Read error: {}", e)),
                    Ok(new_content) if new_content == request.original_content => {
                        self.show_flash("No changes");
                    }
                    Ok(new_content) => {
                        match self.apply_edited_content(
                            &request.entity_type,
                            &request.entity_id,
                            &new_content,
                        ) {
                            Ok(()) => self.show_flash(&format!("Updated {}", request.entity_id)),
                            Err(e) => self.show_flash(&format!("Save error: {}", e)),
                        }
                    }
                }
            }
            Ok(_) => {
                self.show_flash("Edit cancelled");
            }
            Err(e) => {
                self.show_flash(&format!("Editor error: {}", e));
            }
        }

        // Clean up temp file
        let _ = std::fs::remove_file(&request.temp_path);

        Ok(())
    }

    fn apply_edited_content(
        &mut self,
        entity_type: &EntityType,
        entity_id: &str,
        content: &str,
    ) -> Result<()> {
        let parsed = parse_editor_content(content).map_err(crate::error::JjjError::Validation)?;

        match entity_type {
            EntityType::Problem => {
                let priority = parsed
                    .fields
                    .get("priority")
                    .and_then(|s| s.parse::<crate::models::Priority>().ok());
                let confidence = parsed
                    .fields
                    .get("confidence")
                    .and_then(|s| s.parse::<crate::models::Confidence>().ok());
                let status = parsed
                    .fields
                    .get("status")
                    .and_then(|s| s.parse::<crate::models::ProblemStatus>().ok());
                self.store
                    .with_metadata(&format!("Edit problem {}", entity_id), || {
                        let mut problem = self.store.load_problem(entity_id)?;
                        problem.title = parsed.title.clone();
                        problem.description = parsed.description.clone();
                        problem.tags = parsed.tags.clone();
                        if let Some(p) = priority {
                            problem.priority = p;
                        }
                        if let Some(c) = confidence {
                            problem.confidence = c;
                        }
                        if let Some(s) = &status {
                            if *s != problem.status {
                                problem
                                    .try_set_status(s.clone())
                                    .map_err(crate::error::JjjError::Validation)?;
                            }
                        }
                        self.store.save_problem(&problem)
                    })?;
            }
            EntityType::Solution => {
                let status = parsed
                    .fields
                    .get("status")
                    .and_then(|s| s.parse::<crate::models::SolutionStatus>().ok());
                self.store
                    .with_metadata(&format!("Edit solution {}", entity_id), || {
                        let mut solution = self.store.load_solution(entity_id)?;
                        solution.title = parsed.title.clone();
                        solution.approach = parsed.description.clone();
                        solution.tags = parsed.tags.clone();
                        if let Some(s) = &status {
                            if *s != solution.status {
                                solution
                                    .try_set_status(s.clone())
                                    .map_err(crate::error::JjjError::Validation)?;
                            }
                        }
                        self.store.save_solution(&solution)
                    })?;
            }
            EntityType::Critique => {
                let severity = parsed
                    .fields
                    .get("severity")
                    .and_then(|s| s.parse::<crate::models::CritiqueSeverity>().ok());
                let status = parsed
                    .fields
                    .get("status")
                    .and_then(|s| s.parse::<crate::models::CritiqueStatus>().ok());
                self.store
                    .with_metadata(&format!("Edit critique {}", entity_id), || {
                        let mut critique = self.store.load_critique(entity_id)?;
                        critique.title = parsed.title.clone();
                        critique.argument = parsed.description.clone();
                        if let Some(sev) = severity {
                            critique.severity = sev;
                        }
                        if let Some(s) = &status {
                            if *s != critique.status {
                                critique
                                    .try_set_status(s.clone())
                                    .map_err(crate::error::JjjError::Validation)?;
                            }
                        }
                        self.store.save_critique(&critique)
                    })?;
            }
            EntityType::Milestone => {
                let status = parsed
                    .fields
                    .get("status")
                    .and_then(|s| s.parse::<crate::models::MilestoneStatus>().ok());
                let target_date = parsed.fields.get("target_date").and_then(|s| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .and_then(|dt| dt.and_local_timezone(chrono::Utc).single())
                });

                self.store
                    .with_metadata(&format!("Edit milestone {}", entity_id), || {
                        let mut milestone = self.store.load_milestone(entity_id)?;
                        milestone.title = parsed.title.clone();
                        milestone.description = parsed.description.clone();
                        if let Some(s) = status {
                            milestone.set_status(s);
                        }
                        if let Some(d) = target_date {
                            milestone.target_date = Some(d);
                        } else {
                            milestone.target_date = None;
                        }
                        self.store.save_milestone(&milestone)
                    })?;
            }
        }

        self.refresh_data()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_problem_with_priority() {
        let content = "---\ntitle: Fix auth bug\nstatus: Open\npriority: high\ntags: auth, security\n---\n\nThe login form breaks.\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.title, "Fix auth bug");
        assert_eq!(parsed.fields.get("status").unwrap(), "Open");
        assert_eq!(parsed.fields.get("priority").unwrap(), "high");
        assert_eq!(parsed.tags, vec!["auth", "security"]);
        assert_eq!(parsed.description, "The login form breaks.");

        // Verify priority parses
        let priority = parsed
            .fields
            .get("priority")
            .unwrap()
            .parse::<crate::models::Priority>()
            .unwrap();
        assert_eq!(priority, crate::models::Priority::High);
    }

    #[test]
    fn test_parse_priority_change() {
        let content =
            "---\ntitle: Fix auth bug\nstatus: Open\npriority: critical\ntags: \n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        let priority = parsed
            .fields
            .get("priority")
            .unwrap()
            .parse::<crate::models::Priority>()
            .unwrap();
        assert_eq!(priority, crate::models::Priority::Critical);
    }

    #[test]
    fn test_parse_solution_with_status() {
        let content =
            "---\ntitle: Use JWT tokens\nstatus: Submitted\ntags: \n---\n\nSwitch to JWT.\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.title, "Use JWT tokens");
        assert_eq!(parsed.description, "Switch to JWT.");
        let status = parsed
            .fields
            .get("status")
            .unwrap()
            .parse::<crate::models::SolutionStatus>()
            .unwrap();
        assert_eq!(status, crate::models::SolutionStatus::Submitted);
    }

    #[test]
    fn test_parse_critique_with_severity() {
        let content =
            "---\ntitle: Missing input validation\nstatus: Open\nseverity: critical\n---\n\nNo sanitization.\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.description, "No sanitization.");
        let severity = parsed
            .fields
            .get("severity")
            .unwrap()
            .parse::<crate::models::CritiqueSeverity>()
            .unwrap();
        assert_eq!(severity, crate::models::CritiqueSeverity::Critical);
    }

    #[test]
    fn test_parse_milestone_with_date() {
        let content = "---\ntitle: v1.0 Release\nstatus: Active\ntarget_date: 2026-06-01\n---\n\nShip it and make sure all tests pass.\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.title, "v1.0 Release");
        assert_eq!(parsed.fields.get("target_date").unwrap(), "2026-06-01");
        assert_eq!(parsed.description, "Ship it and make sure all tests pass.");
        let status = parsed
            .fields
            .get("status")
            .unwrap()
            .parse::<crate::models::MilestoneStatus>()
            .unwrap();
        assert_eq!(status, crate::models::MilestoneStatus::Active);
    }

    #[test]
    fn test_parse_empty_tags() {
        let content = "---\ntitle: Test\nstatus: Open\npriority: medium\ntags: \n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        assert!(parsed.tags.is_empty());
    }

    #[test]
    fn test_parse_invalid_format() {
        let content = "no frontmatter here";
        assert!(parse_editor_content(content).is_err());
    }

    #[test]
    fn test_parse_invalid_priority_ignored() {
        let content = "---\ntitle: Test\nstatus: Open\npriority: bogus\ntags: \n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.fields.get("priority").unwrap(), "bogus");
        assert!(parsed
            .fields
            .get("priority")
            .unwrap()
            .parse::<crate::models::Priority>()
            .is_err());
    }

    #[test]
    fn test_parse_strips_inline_comments() {
        let content = "---\ntitle: Fix auth bug\nstatus: Open # open, in_progress, solved, dissolved\npriority: high # critical, high, medium, low\ntags: auth, security\n---\n\nThe login form breaks.\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.fields.get("status").unwrap(), "Open");
        assert_eq!(parsed.fields.get("priority").unwrap(), "high");
        assert_eq!(parsed.tags, vec!["auth", "security"]);
        assert_eq!(parsed.description, "The login form breaks.");

        let priority = parsed
            .fields
            .get("priority")
            .unwrap()
            .parse::<crate::models::Priority>()
            .unwrap();
        assert_eq!(priority, crate::models::Priority::High);

        let status = parsed
            .fields
            .get("status")
            .unwrap()
            .parse::<crate::models::ProblemStatus>()
            .unwrap();
        assert_eq!(status, crate::models::ProblemStatus::Open);
    }

    #[test]
    fn test_parse_strips_comments_on_all_entity_types() {
        let content = "---\ntitle: Use JWT\nstatus: Proposed # proposed, submitted, approved, withdrawn\ntags: \n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.fields.get("status").unwrap(), "Proposed");

        let content = "---\ntitle: Bad input\nstatus: Open # open, addressed, valid, dismissed\nseverity: high # critical, high, medium, low\n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.fields.get("status").unwrap(), "Open");
        assert_eq!(parsed.fields.get("severity").unwrap(), "high");

        // Milestone with comments
        let content = "---\ntitle: v2.0\nstatus: Planning # planning, active, completed, cancelled\ntarget_date: 2026-12-01 # YYYY-MM-DD\n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.fields.get("status").unwrap(), "Planning");
        assert_eq!(parsed.fields.get("target_date").unwrap(), "2026-12-01");
    }

    #[test]
    fn test_parse_preserves_hash_in_title() {
        let content = "---\ntitle: Fix issue #42\nstatus: Open\npriority: medium\n---\n\n\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(parsed.title, "Fix issue #42");
    }

    #[test]
    fn test_parse_body_is_description() {
        // Body after frontmatter is the description — no ## header needed
        let content = "---\ntitle: Test\nstatus: Open\n---\n\nThis is **markdown** with\nmultiple lines.\n\n## A heading\n\nMore text.\n";
        let parsed = parse_editor_content(content).unwrap();
        assert_eq!(
            parsed.description,
            "This is **markdown** with\nmultiple lines.\n\n## A heading\n\nMore text."
        );
    }
}
