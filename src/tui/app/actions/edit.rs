use crate::error::Result;
use crate::tui::app::{App, InputAction, InputMode};
use crate::tui::next_actions::EntityType;

impl App {
    pub(in crate::tui::app) fn update_title(
        &mut self,
        entity_type: &EntityType,
        entity_id: &str,
        new_title: &str,
    ) -> Result<()> {
        match entity_type {
            EntityType::Problem => {
                self.store.with_metadata(
                    &format!("Update problem title: {}", new_title),
                    || {
                        let mut problem = self.store.load_problem(entity_id)?;
                        problem.title = new_title.to_string();
                        self.store.save_problem(&problem)
                    },
                )?;
            }
            EntityType::Solution => {
                self.store.with_metadata(
                    &format!("Update solution title: {}", new_title),
                    || {
                        let mut solution = self.store.load_solution(entity_id)?;
                        solution.title = new_title.to_string();
                        self.store.save_solution(&solution)
                    },
                )?;
            }
            EntityType::Finding => {
                self.store.with_metadata(
                    &format!("Update finding title: {}", new_title),
                    || {
                        let mut finding = self.store.load_finding(entity_id)?;
                        finding.title = new_title.to_string();
                        self.store.save_finding(&finding)
                    },
                )?;
            }
            EntityType::Critique => {
                self.store.with_metadata(
                    &format!("Update critique title: {}", new_title),
                    || {
                        let mut critique = self.store.load_critique(entity_id)?;
                        critique.title = new_title.to_string();
                        self.store.save_critique(&critique)
                    },
                )?;
            }
            EntityType::Milestone => {
                self.store.with_metadata(
                    &format!("Update milestone title: {}", new_title),
                    || {
                        let mut milestone = self.store.load_milestone(entity_id)?;
                        milestone.title = new_title.to_string();
                        self.store.save_milestone(&milestone)
                    },
                )?;
            }
        }
        self.show_flash(&format!("Updated title: {}", new_title));
        self.refresh_data()?;
        Ok(())
    }

    pub(in crate::tui::app) fn start_new_item(&mut self) -> Result<()> {
        use crate::tui::tree::TreeNode;

        let (prompt, action) = if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            match &item.node {
                TreeNode::ProjectRoot { .. } => (
                    "New milestone title: ".to_string(),
                    InputAction::NewMilestone,
                ),
                TreeNode::Milestone { id, .. } => (
                    "New problem title: ".to_string(),
                    InputAction::NewProblem {
                        milestone_id: Some(id.clone()),
                    },
                ),
                TreeNode::Backlog { .. } => (
                    "New problem title: ".to_string(),
                    InputAction::NewProblem { milestone_id: None },
                ),
                TreeNode::Problem { id, .. } => (
                    "New solution title: ".to_string(),
                    InputAction::NewSolution {
                        problem_id: id.clone(),
                    },
                ),
                TreeNode::Solution { id, .. } => (
                    "New critique title: ".to_string(),
                    InputAction::NewCritique {
                        solution_id: id.clone(),
                    },
                ),
                TreeNode::Critique { .. } => return Ok(()),
            }
        } else {
            return Ok(());
        };

        self.ui.input_mode = InputMode::Input {
            prompt,
            buffer: String::new(),
            action,
            cursor_pos: 0,
        };
        Ok(())
    }

    pub(in crate::tui::app) fn start_edit_title(&mut self) -> Result<()> {
        use crate::tui::tree::TreeNode;

        let (prompt, action, current_title) =
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Problem,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Solution { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Solution,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Critique { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Critique,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Milestone { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Milestone,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    _ => return Ok(()),
                }
            } else {
                return Ok(());
            };

        let cursor_pos = current_title.chars().count();
        self.ui.input_mode = InputMode::Input {
            prompt,
            buffer: current_title,
            action,
            cursor_pos,
        };
        Ok(())
    }

    pub(in crate::tui::app) fn start_edit_tags(&mut self) -> Result<()> {
        use crate::tui::tree::TreeNode;

        // Multi-select: batch tag mode with +add/-remove syntax
        if !self.ui.selected_ids.is_empty() {
            let targets: Vec<(EntityType, String)> = self
                .cache
                .tree_items
                .iter()
                .filter(|item| self.ui.selected_ids.contains(item.node.id()))
                .filter_map(|item| match &item.node {
                    TreeNode::Problem { id, .. } => Some((EntityType::Problem, id.clone())),
                    TreeNode::Solution { id, .. } => Some((EntityType::Solution, id.clone())),
                    _ => None,
                })
                .collect();
            if targets.is_empty() {
                return Ok(());
            }
            self.ui.input_mode = InputMode::Input {
                prompt: format!(
                    "Tags for {} items (+add, -remove, or replace): ",
                    targets.len()
                ),
                buffer: String::new(),
                action: InputAction::BatchEditTags { targets },
                cursor_pos: 0,
            };
            return Ok(());
        }

        // Single item: pre-fill current tags
        let (prompt, action, current_tags) =
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, .. } => {
                        let problem = self.store.load_problem(id)?;
                        (
                            "Tags (comma-separated): ".to_string(),
                            InputAction::EditTags {
                                entity_type: EntityType::Problem,
                                entity_id: id.clone(),
                            },
                            problem.tags.join(", "),
                        )
                    }
                    TreeNode::Solution { id, .. } => {
                        let solution = self.store.load_solution(id)?;
                        (
                            "Tags (comma-separated): ".to_string(),
                            InputAction::EditTags {
                                entity_type: EntityType::Solution,
                                entity_id: id.clone(),
                            },
                            solution.tags.join(", "),
                        )
                    }
                    _ => return Ok(()),
                }
            } else {
                return Ok(());
            };

        let cursor_pos = current_tags.chars().count();
        self.ui.input_mode = InputMode::Input {
            prompt,
            buffer: current_tags,
            action,
            cursor_pos,
        };
        Ok(())
    }

    pub(in crate::tui::app) fn update_tags(
        &mut self,
        entity_type: &EntityType,
        entity_id: &str,
        input: &str,
    ) -> Result<()> {
        let mut tags: Vec<String> = input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // Case-insensitive dedup
        let mut seen = std::collections::HashSet::new();
        tags.retain(|t| seen.insert(t.to_lowercase()));
        tags.sort();

        match entity_type {
            EntityType::Problem => {
                self.store
                    .with_metadata(&format!("Update problem tags: {}", entity_id), || {
                        let mut problem = self.store.load_problem(entity_id)?;
                        problem.tags = tags.clone();
                        self.store.save_problem(&problem)
                    })?;
            }
            EntityType::Solution => {
                self.store.with_metadata(
                    &format!("Update solution tags: {}", entity_id),
                    || {
                        let mut solution = self.store.load_solution(entity_id)?;
                        solution.tags = tags.clone();
                        self.store.save_solution(&solution)
                    },
                )?;
            }
            EntityType::Finding => {
                self.store
                    .with_metadata(&format!("Update finding tags: {}", entity_id), || {
                        let mut finding = self.store.load_finding(entity_id)?;
                        finding.tags = tags.clone();
                        self.store.save_finding(&finding)
                    })?;
            }
            EntityType::Critique | EntityType::Milestone => return Ok(()),
        }
        self.show_flash("Tags updated");
        self.refresh_data()?;
        Ok(())
    }

    /// Batch update tags on multiple entities.
    ///
    /// Supports three modes based on input syntax:
    /// - `+tag1, +tag2` — add tags to all targets (keeps existing)
    /// - `-tag1, -tag2` — remove tags from all targets
    /// - `tag1, tag2` (no prefix) — replace all tags on all targets
    /// - Mixed `+tag1, -tag2` — add and remove in one operation
    pub(in crate::tui::app) fn batch_update_tags(
        &mut self,
        targets: &[(EntityType, String)],
        input: &str,
    ) -> Result<()> {
        // Parse input into add/remove/replace sets
        let tokens: Vec<&str> = input
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let has_prefixes = tokens
            .iter()
            .any(|t| t.starts_with('+') || t.starts_with('-'));

        let (to_add, to_remove): (Vec<String>, Vec<String>) = if has_prefixes {
            let mut add = Vec::new();
            let mut remove = Vec::new();
            for token in &tokens {
                if let Some(tag) = token.strip_prefix('+') {
                    let tag = tag.trim();
                    if !tag.is_empty() {
                        add.push(tag.to_string());
                    }
                } else if let Some(tag) = token.strip_prefix('-') {
                    let tag = tag.trim();
                    if !tag.is_empty() {
                        remove.push(tag.to_string());
                    }
                } else {
                    // No prefix in mixed mode — treat as add
                    add.push(token.to_string());
                }
            }
            (add, remove)
        } else {
            // No prefixes — replace mode
            (tokens.iter().map(|s| s.to_string()).collect(), Vec::new())
        };
        let replace_mode = !has_prefixes;

        let count = targets.len();
        self.store
            .with_metadata(&format!("Batch update tags on {} items", count), || {
                for (entity_type, entity_id) in targets {
                    match entity_type {
                        EntityType::Problem => {
                            let mut problem = self.store.load_problem(entity_id)?;
                            if replace_mode {
                                problem.tags = to_add.clone();
                            } else {
                                for tag in &to_add {
                                    if !problem.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                                        problem.tags.push(tag.clone());
                                    }
                                }
                                problem.tags.retain(|t| {
                                    !to_remove.iter().any(|r| t.eq_ignore_ascii_case(r))
                                });
                            }
                            problem.tags.sort();
                            self.store.save_problem(&problem)?;
                        }
                        EntityType::Solution => {
                            let mut solution = self.store.load_solution(entity_id)?;
                            if replace_mode {
                                solution.tags = to_add.clone();
                            } else {
                                for tag in &to_add {
                                    if !solution.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                                        solution.tags.push(tag.clone());
                                    }
                                }
                                solution.tags.retain(|t| {
                                    !to_remove.iter().any(|r| t.eq_ignore_ascii_case(r))
                                });
                            }
                            solution.tags.sort();
                            self.store.save_solution(&solution)?;
                        }
                        _ => {}
                    }
                }
                Ok(())
            })?;

        let msg = if replace_mode {
            format!("Tags set on {} items", count)
        } else {
            let mut parts = Vec::new();
            if !to_add.is_empty() {
                parts.push(format!("+{}", to_add.join(", +")));
            }
            if !to_remove.is_empty() {
                parts.push(format!("-{}", to_remove.join(", -")));
            }
            format!("{} on {} items", parts.join(", "), count)
        };
        self.show_flash(&msg);
        self.ui.selected_ids.clear();
        self.refresh_data()?;
        Ok(())
    }

    pub(in crate::tui::app) fn cycle_confidence(&mut self) -> Result<()> {
        use crate::tui::tree::TreeNode;

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            if let TreeNode::Problem { id, .. } = &item.node {
                let id = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Cycle confidence on {}", id), || {
                        let mut problem = self.store.load_problem(&id)?;
                        problem.confidence = problem.confidence.next();
                        self.store.save_problem(&problem)?;
                        Ok(problem.confidence.clone())
                    }) {
                    Ok(new_conf) => {
                        self.show_flash(&format!("Confidence: {}", new_conf));
                        self.refresh_data()?;
                    }
                    Err(e) => {
                        self.show_flash(&format!("Error: {}", e));
                    }
                }
            }
        }
        Ok(())
    }
}
