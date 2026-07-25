use crate::error::Result;
use crate::tui::app::{App, InputAction, InputMode};

impl App {
    pub(in crate::tui::app) fn start_delete(&mut self) -> Result<()> {
        use crate::tui::tree::TreeNode;

        if !self.ui.selected_ids.is_empty() {
            // Batch delete: collect all selected entities
            let mut entities = Vec::new();
            for item in &self.cache.tree_items {
                if !self.ui.selected_ids.contains(item.node.id()) {
                    continue;
                }
                match &item.node {
                    TreeNode::Critique { id, .. } => {
                        entities.push(("critique".to_string(), id.clone()));
                    }
                    TreeNode::Solution { id, .. } => {
                        entities.push(("solution".to_string(), id.clone()));
                    }
                    TreeNode::Problem { id, .. } => {
                        entities.push(("problem".to_string(), id.clone()));
                    }
                    TreeNode::Milestone { id, .. } => {
                        entities.push(("milestone".to_string(), id.clone()));
                    }
                    _ => {}
                }
            }

            if entities.is_empty() {
                return Ok(());
            }

            self.ui.input_mode = InputMode::Input {
                prompt: format!("Delete {} items? y to confirm: ", entities.len()),
                buffer: String::new(),
                action: InputAction::BatchConfirmDelete { entities },
                cursor_pos: 0,
            };
            return Ok(());
        }

        // Single delete (existing logic)
        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            let (entity_type, entity_id, title) = match &item.node {
                TreeNode::Critique { id, title, .. } => {
                    ("critique".to_string(), id.clone(), title.clone())
                }
                TreeNode::Solution { id, title, .. } => {
                    let has_critiques = self.data.critiques.iter().any(|c| c.solution_id == *id);
                    if has_critiques {
                        self.show_flash("Delete critiques first");
                        return Ok(());
                    }
                    ("solution".to_string(), id.clone(), title.clone())
                }
                TreeNode::Problem { id, title, .. } => {
                    let has_solutions = self.data.solutions.iter().any(|s| s.problem_id == *id);
                    if has_solutions {
                        self.show_flash("Delete solutions first");
                        return Ok(());
                    }
                    ("problem".to_string(), id.clone(), title.clone())
                }
                TreeNode::Milestone { id, title, .. } => {
                    let has_problems = self
                        .data
                        .problems
                        .iter()
                        .any(|p| p.milestone_id.as_deref() == Some(id));
                    if has_problems {
                        self.show_flash("Remove problems first");
                        return Ok(());
                    }
                    ("milestone".to_string(), id.clone(), title.clone())
                }
                _ => return Ok(()),
            };

            self.ui.input_mode = InputMode::Input {
                prompt: format!("Delete '{}'? y to confirm: ", title),
                buffer: String::new(),
                action: InputAction::ConfirmDelete {
                    entity_type,
                    entity_id,
                },
                cursor_pos: 0,
            };
        }
        Ok(())
    }

    pub(in crate::tui::app) fn batch_delete(
        &mut self,
        entities: &[(String, String)],
    ) -> Result<()> {
        let mut deleted = 0usize;
        let mut errors = Vec::new();

        self.store
            .with_metadata(&format!("Batch delete {} items", entities.len()), || {
                for (entity_type, entity_id) in entities {
                    let result = match entity_type.as_str() {
                        "critique" => self.store.delete_critique(entity_id),
                        "solution" => self.store.delete_solution(entity_id),
                        "problem" => self.store.delete_problem(entity_id),
                        "milestone" => self.store.delete_milestone(entity_id),
                        _ => continue,
                    };
                    match result {
                        Ok(_) => deleted += 1,
                        Err(e) => {
                            errors.push(format!("{}: {}", &entity_id[..6.min(entity_id.len())], e))
                        }
                    }
                }
                Ok(())
            })?;

        let msg = if errors.is_empty() {
            format!("Deleted {} items", deleted)
        } else {
            format!("Deleted {}, {} failed", deleted, errors.len())
        };
        self.show_flash(&msg);
        self.ui.selected_ids.clear();
        self.refresh_data()?;
        Ok(())
    }

    pub(in crate::tui::app) fn delete_entity(
        &mut self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<()> {
        let id = entity_id.to_string();
        let result = match entity_type {
            "critique" => self
                .store
                .with_metadata(&format!("Delete critique {}", entity_id), || {
                    self.store.delete_critique(entity_id)
                }),
            "solution" => self
                .store
                .with_metadata(&format!("Delete solution {}", entity_id), || {
                    self.store.delete_solution(entity_id)
                }),
            "problem" => self
                .store
                .with_metadata(&format!("Delete problem {}", entity_id), || {
                    self.store.delete_problem(entity_id)
                }),
            "milestone" => self
                .store
                .with_metadata(&format!("Delete milestone {}", entity_id), || {
                    self.store.delete_milestone(entity_id)
                }),
            _ => return Ok(()),
        };
        match result {
            Ok(_) => {
                self.show_flash(&format!("Deleted {}", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    pub(in crate::tui::app) fn start_move_to_milestone(&mut self) -> Result<()> {
        use crate::tui::tree::TreeNode;

        if !self.ui.selected_ids.is_empty() {
            // Collect selected problem IDs
            let problem_ids: Vec<String> = self
                .cache
                .tree_items
                .iter()
                .filter(|item| self.ui.selected_ids.contains(item.node.id()))
                .filter_map(|item| match &item.node {
                    TreeNode::Problem { id, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();

            if problem_ids.is_empty() {
                self.show_flash("No problems selected");
                return Ok(());
            }

            self.ui.input_mode = InputMode::Input {
                prompt: format!(
                    "Move {} problems to milestone [→ backlog]: ",
                    problem_ids.len()
                ),
                buffer: String::new(),
                action: InputAction::MoveProblemsToMilestone { problem_ids },
                cursor_pos: 0,
            };
            return Ok(());
        }

        // Single move (existing logic)
        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            if let TreeNode::Problem { id, .. } = &item.node {
                self.ui.input_mode = InputMode::Input {
                    prompt: "Milestone [→ backlog]: ".to_string(),
                    buffer: String::new(),
                    action: InputAction::MoveProblemToMilestone {
                        problem_id: id.clone(),
                    },
                    cursor_pos: 0,
                };
            }
        }
        Ok(())
    }

    pub(in crate::tui::app) fn batch_move_to_milestone(
        &mut self,
        problem_ids: &[String],
        input: &str,
    ) -> Result<()> {
        let input = input.trim();

        let target_milestone = if input.is_empty() {
            None
        } else {
            let input_lower = input.to_lowercase();
            self.data
                .milestones
                .iter()
                .find(|m| m.title.to_lowercase().contains(&input_lower))
        };

        if !input.is_empty() && target_milestone.is_none() {
            self.show_flash("No matching milestone found");
            return Ok(());
        }

        let target_id = target_milestone.map(|m| m.id.clone());
        let dest = target_milestone
            .map(|m| m.title.clone())
            .unwrap_or_else(|| "backlog".to_string());

        self.store.with_metadata(
            &format!("Batch move {} problems to {}", problem_ids.len(), dest),
            || {
                for problem_id in problem_ids {
                    let old_milestone_id = self
                        .store
                        .load_problem(problem_id)
                        .ok()
                        .and_then(|p| p.milestone_id.clone());

                    let mut problem = self.store.load_problem(problem_id)?;
                    problem.milestone_id = target_id.clone();
                    self.store.save_problem(&problem)?;

                    if let Some(ref old_id) = old_milestone_id {
                        if let Ok(mut old_milestone) = self.store.load_milestone(old_id) {
                            old_milestone.remove_problem(problem_id);
                            self.store.save_milestone(&old_milestone)?;
                        }
                    }

                    if let Some(ref new_id) = target_id {
                        let mut new_milestone = self.store.load_milestone(new_id)?;
                        new_milestone.add_problem(problem_id);
                        self.store.save_milestone(&new_milestone)?;
                    }
                }
                Ok(())
            },
        )?;

        self.show_flash(&format!("Moved {} to {}", problem_ids.len(), dest));
        self.ui.selected_ids.clear();
        self.refresh_data()?;
        Ok(())
    }

    pub(in crate::tui::app) fn move_problem_to_milestone(
        &mut self,
        problem_id: &str,
        input: &str,
    ) -> Result<()> {
        let input = input.trim();

        // Find target milestone by fuzzy title match
        let target_milestone = if input.is_empty() {
            None
        } else {
            let input_lower = input.to_lowercase();
            self.data
                .milestones
                .iter()
                .find(|m| m.title.to_lowercase().contains(&input_lower))
        };

        if !input.is_empty() && target_milestone.is_none() {
            self.show_flash("No matching milestone found");
            return Ok(());
        }

        let target_id = target_milestone.map(|m| m.id.clone());

        // Load problem to find old milestone
        let problem = self.store.load_problem(problem_id)?;
        let old_milestone_id = problem.milestone_id.clone();

        self.store
            .with_metadata(&format!("Move problem {} to milestone", problem_id), || {
                // Update problem's milestone_id
                let mut problem = self.store.load_problem(problem_id)?;
                problem.milestone_id = target_id.clone();
                self.store.save_problem(&problem)?;

                // Remove from old milestone
                if let Some(ref old_id) = old_milestone_id {
                    if let Ok(mut old_milestone) = self.store.load_milestone(old_id) {
                        old_milestone.remove_problem(problem_id);
                        self.store.save_milestone(&old_milestone)?;
                    }
                }

                // Add to new milestone
                if let Some(ref new_id) = target_id {
                    let mut new_milestone = self.store.load_milestone(new_id)?;
                    new_milestone.add_problem(problem_id);
                    self.store.save_milestone(&new_milestone)?;
                }

                Ok(())
            })?;

        let dest = target_milestone
            .map(|m| m.title.as_str())
            .unwrap_or("backlog");
        self.show_flash(&format!("Moved to {}", dest));
        self.refresh_data()?;
        Ok(())
    }
}
