use super::App;
use crate::error::Result;
use ratatui::backend::Backend;
use ratatui::Terminal;

use super::super::next_actions::EntityType;
use super::EditorRequest;

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
        let temp_path = std::env::temp_dir().join(format!(
            "jjj-edit-{}.md",
            &entity_id[..8.min(entity_id.len())]
        ));
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
                Ok(format!(
                    "---\ntitle: {}\nstatus: {}\npriority: {}\n---\n\n## Description\n\n{}\n",
                    problem.title,
                    problem.status,
                    problem.priority,
                    if problem.description.is_empty() {
                        ""
                    } else {
                        &problem.description
                    }
                ))
            }
            EntityType::Solution => {
                let solution = self.store.load_solution(entity_id)?;
                Ok(format!(
                    "---\ntitle: {}\nstatus: {}\n---\n\n## Description\n\n{}\n",
                    solution.title,
                    solution.status,
                    if solution.approach.is_empty() {
                        ""
                    } else {
                        &solution.approach
                    }
                ))
            }
            EntityType::Critique => {
                let critique = self.store.load_critique(entity_id)?;
                Ok(format!(
                    "---\ntitle: {}\nstatus: {}\nseverity: {}\n---\n\n## Description\n\n{}\n",
                    critique.title,
                    critique.status,
                    critique.severity,
                    if critique.argument.is_empty() {
                        ""
                    } else {
                        &critique.argument
                    }
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
        // Simple parsing: extract title from frontmatter, description from body
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(crate::error::JjjError::Validation(
                "Invalid format".to_string(),
            ));
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim();

        // Extract title from frontmatter
        let title = frontmatter
            .lines()
            .find(|l| l.starts_with("title:"))
            .map(|l| l.trim_start_matches("title:").trim().to_string())
            .unwrap_or_default();

        // Extract description from body (after ## Description header)
        let description = body
            .strip_prefix("## Description")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        match entity_type {
            EntityType::Problem => {
                self.store
                    .with_metadata(&format!("Edit problem {}", entity_id), || {
                        let mut problem = self.store.load_problem(entity_id)?;
                        problem.title = title.clone();
                        problem.description = description.clone();
                        self.store.save_problem(&problem)
                    })?;
            }
            EntityType::Solution => {
                self.store
                    .with_metadata(&format!("Edit solution {}", entity_id), || {
                        let mut solution = self.store.load_solution(entity_id)?;
                        solution.title = title.clone();
                        solution.approach = description.clone();
                        self.store.save_solution(&solution)
                    })?;
            }
            EntityType::Critique => {
                self.store
                    .with_metadata(&format!("Edit critique {}", entity_id), || {
                        let mut critique = self.store.load_critique(entity_id)?;
                        critique.title = title.clone();
                        critique.argument = description.clone();
                        self.store.save_critique(&critique)
                    })?;
            }
        }

        self.refresh_data()?;
        Ok(())
    }
}
