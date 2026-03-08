use super::App;
use crate::error::Result;

impl App {
    pub(super) fn navigate_up(&mut self) {
        if self.ui.tree_index > 0 {
            self.ui.tree_index -= 1;
        }
        self.update_selected_detail();
    }

    pub(super) fn navigate_down(&mut self) {
        if self.ui.tree_index < self.cache.tree_items.len().saturating_sub(1) {
            self.ui.tree_index += 1;
        }
        self.update_selected_detail();
    }

    /// Jump to the next (or previous) tree item that has an action symbol.
    ///
    /// Wraps around: going forward past the last action item cycles to the first;
    /// going backward past the first cycles to the last. Automatically expands
    /// ancestor nodes so the target is visible in the tree before navigating.
    pub(super) fn jump_to_next_action(&mut self, reverse: bool) {
        if self.cache.tree_items.is_empty() {
            return;
        }

        // Find indices of items with action symbols
        let action_indices: Vec<usize> = self
            .cache
            .tree_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.action_symbol.is_some())
            .map(|(i, _)| i)
            .collect();

        if action_indices.is_empty() {
            return;
        }

        // Find next action item
        let current = self.ui.tree_index;
        let next_index = if reverse {
            // Find previous action item (or wrap to last)
            action_indices
                .iter()
                .rev()
                .find(|&&i| i < current)
                .or_else(|| action_indices.last())
                .copied()
        } else {
            // Find next action item (or wrap to first)
            action_indices
                .iter()
                .find(|&&i| i > current)
                .or_else(|| action_indices.first())
                .copied()
        };

        if let Some(idx) = next_index {
            // Expand ancestors to reveal the item
            let target_id = self.cache.tree_items[idx].node.id().to_string();
            self.expand_to_reveal(&target_id);
            self.rebuild_tree();
            // Re-find the item after tree rebuild
            for (i, item) in self.cache.tree_items.iter().enumerate() {
                if item.node.id() == target_id {
                    self.ui.tree_index = i;
                    break;
                }
            }
            self.update_selected_detail();
        }
    }

    /// Ensure a target entity is visible by expanding its ancestor nodes.
    ///
    /// Walks up the entity hierarchy (critique → solution → problem → milestone)
    /// and inserts each ancestor's ID into `expanded_nodes`. Must be followed by
    /// `rebuild_tree()` to take effect.
    fn expand_to_reveal(&mut self, target_id: &str) {
        // For a solution, we need its problem expanded, and that problem's milestone expanded
        if let Some(solution) = self.data.solutions.iter().find(|s| s.id == target_id) {
            self.ui.expanded_nodes.insert(solution.problem_id.clone());

            if let Some(problem) = self
                .data
                .problems
                .iter()
                .find(|p| p.id == solution.problem_id)
            {
                if let Some(milestone_id) = &problem.milestone_id {
                    self.ui.expanded_nodes.insert(milestone_id.clone());
                } else {
                    self.ui.expanded_nodes.insert("backlog".to_string());
                }
            }
        }

        // For a problem, we need its milestone expanded
        if let Some(problem) = self.data.problems.iter().find(|p| p.id == target_id) {
            if let Some(milestone_id) = &problem.milestone_id {
                self.ui.expanded_nodes.insert(milestone_id.clone());
            } else {
                self.ui.expanded_nodes.insert("backlog".to_string());
            }
        }

        // For a critique, we need its solution and problem expanded
        if let Some(critique) = self.data.critiques.iter().find(|c| c.id == target_id) {
            self.ui.expanded_nodes.insert(critique.solution_id.clone());

            if let Some(solution) = self
                .data
                .solutions
                .iter()
                .find(|s| s.id == critique.solution_id)
            {
                self.ui.expanded_nodes.insert(solution.problem_id.clone());

                if let Some(problem) = self
                    .data
                    .problems
                    .iter()
                    .find(|p| p.id == solution.problem_id)
                {
                    if let Some(milestone_id) = &problem.milestone_id {
                        self.ui.expanded_nodes.insert(milestone_id.clone());
                    } else {
                        self.ui.expanded_nodes.insert("backlog".to_string());
                    }
                }
            }
        }
    }

    pub(super) fn collapse_or_parent(&mut self) {
        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Collapse current node
                self.ui.expanded_nodes.remove(&node_id);
                self.rebuild_tree();
            } else if item.depth > 0 {
                // Move to parent
                for i in (0..self.ui.tree_index).rev() {
                    if self.cache.tree_items[i].depth < item.depth {
                        self.ui.tree_index = i;
                        break;
                    }
                }
            }
        }
    }

    pub(super) fn expand_or_child(&mut self) {
        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            if !item.has_children {
                return;
            }

            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Move to first child
                if self.ui.tree_index + 1 < self.cache.tree_items.len() {
                    self.ui.tree_index += 1;
                }
            } else {
                // Expand
                self.ui.expanded_nodes.insert(node_id);
                self.rebuild_tree();
            }
        }
    }

    pub(super) fn scroll_detail_down(&mut self) {
        self.ui.detail_scroll = self.ui.detail_scroll.saturating_add(1);
    }

    pub(super) fn scroll_detail_up(&mut self) {
        self.ui.detail_scroll = self.ui.detail_scroll.saturating_sub(1);
    }

    pub(super) fn page_detail_down(&mut self) {
        self.ui.detail_scroll = self.ui.detail_scroll.saturating_add(10);
    }

    pub(super) fn toggle_related_panel(&mut self) {
        self.ui.show_related = !self.ui.show_related;
    }

    pub(super) fn toggle_filter(&mut self) {
        self.ui.filter_actions_only = !self.ui.filter_actions_only;
        let mode = if self.ui.filter_actions_only {
            "Actions only"
        } else {
            "Full tree"
        };
        self.show_flash(mode);
    }

    pub(super) fn start_search(&mut self) {
        use super::InputAction;
        use super::InputMode;
        let buffer = self.ui.search_filter.clone().unwrap_or_default();
        let cursor_pos = buffer.len();
        self.ui.input_mode = InputMode::Input {
            prompt: "/".to_string(),
            buffer,
            action: InputAction::Search,
            cursor_pos,
        };
    }

    pub(super) fn toggle_help(&mut self) {
        use super::InputMode;
        self.ui.input_mode = match &self.ui.input_mode {
            InputMode::Help => InputMode::Normal,
            _ => InputMode::Help,
        };
    }

    pub(super) fn goto_change(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

        let solution_id = if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            match &item.node {
                TreeNode::Solution { id, .. } => id.clone(),
                _ => return Ok(()),
            }
        } else {
            return Ok(());
        };

        let solution = match self.data.solutions.iter().find(|s| s.id == solution_id) {
            Some(s) => s,
            None => {
                self.show_flash("Solution not found");
                return Ok(());
            }
        };

        if let Some(change_id) = solution.change_ids.last() {
            match self.store.jj_client.edit(change_id) {
                Ok(_) => self.show_flash(&format!("Switched to {}", change_id)),
                Err(e) => self.show_flash(&format!("Error: {}", e)),
            }
        } else {
            self.show_flash("No changes attached");
        }

        Ok(())
    }

    pub fn rebuild_tree(&mut self) {
        self.cache.tree_items = super::super::build_flat_tree(
            &self.data.milestones,
            &self.data.problems,
            &self.data.solutions,
            &self.data.critiques,
            &self.ui.expanded_nodes,
        );
        // Re-apply search filter if active
        if self.ui.search_filter.is_some() {
            self.apply_search_filter_to_tree();
        }
    }

    pub(super) fn apply_search_filter(&mut self) {
        self.rebuild_tree();
        super::super::annotate_tree_with_actions(
            &mut self.cache.tree_items,
            &self.cache.next_actions,
        );
        // Clamp tree_index
        let max_index = self.cache.tree_items.len().saturating_sub(1);
        if self.ui.tree_index > max_index {
            self.ui.tree_index = max_index;
        }
        self.update_selected_detail();
    }

    fn apply_search_filter_to_tree(&mut self) {
        if let Some(ref query) = self.ui.search_filter {
            let query_lower = query.to_lowercase();
            self.cache.tree_items.retain(|item| {
                let title = item.node.title().to_lowercase();
                let id = item.node.id().to_lowercase();
                title.contains(&query_lower) || id.contains(&query_lower)
            });
        }
    }

    pub fn context_hints(&self) -> String {
        use super::super::tree::TreeNode;

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            match &item.node {
                TreeNode::Milestone { id, .. } => {
                    format!("{}: [e]dit", id)
                }
                TreeNode::Backlog { .. } => "[n]ew problem".to_string(),
                TreeNode::Problem { id, .. } => {
                    format!(
                        "{}: [n]ew solution [s]olve [d]issolve [e]dit [t]ags [E]dit in $EDITOR [x] delete",
                        id
                    )
                }
                TreeNode::Solution { id, .. } => {
                    format!(
                        "{}: [a]pprove [r] withdraw [g]o to change [n]ew critique [e]dit [t]ags [E]dit in $EDITOR [x] delete",
                        id
                    )
                }
                TreeNode::Critique { id, .. } => {
                    format!("{}: [a]ddress [d]ismiss [e]dit [E]dit in $EDITOR [x] delete", id)
                }
            }
        } else {
            "No selection".to_string()
        }
    }

    pub fn update_selected_detail(&mut self) {
        use super::super::tree::TreeNode;

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            self.cache.selected_detail = match &item.node {
                TreeNode::Milestone { id, .. } => self
                    .data
                    .milestones
                    .iter()
                    .find(|m| m.id == *id)
                    .cloned()
                    .map(super::super::DetailContent::Milestone)
                    .unwrap_or(super::super::DetailContent::None),
                TreeNode::Backlog { .. } => super::super::DetailContent::None,
                TreeNode::Problem { id, .. } => self
                    .data
                    .problems
                    .iter()
                    .find(|p| p.id == *id)
                    .cloned()
                    .map(super::super::DetailContent::Problem)
                    .unwrap_or(super::super::DetailContent::None),
                TreeNode::Solution { id, .. } => self
                    .data
                    .solutions
                    .iter()
                    .find(|s| s.id == *id)
                    .cloned()
                    .map(super::super::DetailContent::Solution)
                    .unwrap_or(super::super::DetailContent::None),
                TreeNode::Critique { id, .. } => self
                    .data
                    .critiques
                    .iter()
                    .find(|c| c.id == *id)
                    .cloned()
                    .map(super::super::DetailContent::Critique)
                    .unwrap_or(super::super::DetailContent::None),
            };
        }
        self.ui.detail_scroll = 0; // Reset scroll on new selection
        self.load_related_for_selected(); // Load related items for new selection
    }

    pub(super) fn get_selected_entity(
        &self,
    ) -> Option<(String, super::super::next_actions::EntityType)> {
        use super::super::tree::TreeNode;

        self.cache
            .tree_items
            .get(self.ui.tree_index)
            .and_then(|item| match &item.node {
                TreeNode::Problem { id, .. } => {
                    Some((id.clone(), super::super::next_actions::EntityType::Problem))
                }
                TreeNode::Solution { id, .. } => {
                    Some((id.clone(), super::super::next_actions::EntityType::Solution))
                }
                TreeNode::Critique { id, .. } => {
                    Some((id.clone(), super::super::next_actions::EntityType::Critique))
                }
                _ => None,
            })
    }

    pub(super) fn get_selected_entity_info(&self) -> Option<(String, String)> {
        use super::super::tree::TreeNode;

        self.cache
            .tree_items
            .get(self.ui.tree_index)
            .and_then(|item| match &item.node {
                TreeNode::Problem { id, .. } => Some(("problem".to_string(), id.clone())),
                TreeNode::Solution { id, .. } => Some(("solution".to_string(), id.clone())),
                TreeNode::Critique { id, .. } => Some(("critique".to_string(), id.clone())),
                TreeNode::Milestone { id, .. } => Some(("milestone".to_string(), id.clone())),
                TreeNode::Backlog { .. } => None,
            })
    }
}
