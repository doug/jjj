//! TUI action handlers, split by concern:
//! - [`create`]: entity creation (problems, solutions, critiques, milestones)
//! - [`edit`]: title/tag editing and confidence cycling
//! - [`lifecycle`]: state-machine transitions dispatched to the domain layer
//! - [`organize`]: delete and move-to-milestone flows (single + batch)
//! - [`ordering`]: personal ranking — nudge/fling, sized gaps, undo
//!
//! `refresh_data`/`rebuild_cache` live here: every action funnels through
//! them to re-derive the tree, next-actions, and selection state.

mod create;
mod edit;
mod lifecycle;
mod ordering;
mod organize;

use super::App;
use crate::error::Result;

impl App {
    pub(super) fn refresh_data(&mut self) -> Result<()> {
        use std::collections::HashSet;

        use super::ProjectData;
        self.data = ProjectData::load(&self.store)?;
        self.ui.related_cache.clear();
        self.rebuild_cache();
        // Clamp tree_index to valid range after data change
        let max_index = self.cache.tree_items.len().saturating_sub(1);
        if self.ui.tree_index > max_index {
            self.ui.tree_index = max_index;
        }
        // Skip past non-navigable nodes (tier separators)
        while self.ui.tree_index > 0
            && !self
                .cache
                .tree_items
                .get(self.ui.tree_index)
                .map(|i| i.node.is_navigable())
                .unwrap_or(false)
        {
            self.ui.tree_index -= 1;
        }
        // Prune selected_ids that no longer exist in the tree
        let valid_ids: HashSet<String> = self
            .cache
            .tree_items
            .iter()
            .map(|item| item.node.id().to_string())
            .collect();
        self.ui.selected_ids.retain(|id| valid_ids.contains(id));
        Ok(())
    }

    fn rebuild_cache(&mut self) {
        self.cache.next_actions = super::super::next_actions::build_next_actions(
            &self.data.problems,
            &self.data.solutions,
            &self.data.critiques,
            &self.user,
        );
        self.rebuild_tree();
        // Annotate tree with action symbols
        super::super::annotate_tree_with_actions(
            &mut self.cache.tree_items,
            &self.cache.next_actions,
        );
        self.update_selected_detail();
    }
}
