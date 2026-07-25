use crate::display::short_id;
use crate::error::Result;
use crate::tui::app::App;

impl App {
    /// Toggle between personal and global ordering view.
    pub(in crate::tui::app) fn toggle_ordering_view(&mut self) {
        self.ui.show_personal_ordering = !self.ui.show_personal_ordering;
        self.refresh_data().ok();
        let view = if self.ui.show_personal_ordering {
            "Personal"
        } else {
            "Global"
        };
        self.show_flash(&format!("Showing {} ordering", view));
    }

    /// Get (milestone_id, problem_id) if the selected tree item is a problem under a milestone.
    fn selected_milestone_problem(&self) -> Option<(String, String)> {
        let item = self.cache.tree_items.get(self.ui.tree_index)?;
        let problem_id = match &item.node {
            crate::tui::tree::TreeNode::Problem { id, .. } => id.clone(),
            _ => return None,
        };
        let problem = self.data.problems.iter().find(|p| p.id == problem_id)?;
        let milestone_id = problem.milestone_id.clone()?;
        Some((milestone_id, problem_id))
    }

    /// Create a default ordering for a milestone from current problem list.
    fn default_ordering_for_milestone(
        &self,
        milestone_id: &str,
    ) -> crate::ranking::ordering::UserOrdering {
        let order: Vec<String> = self
            .data
            .problems
            .iter()
            .filter(|p| p.milestone_id.as_deref() == Some(milestone_id))
            .map(|p| p.id.clone())
            .collect();
        crate::ranking::ordering::UserOrdering {
            order,
            gaps: std::collections::HashMap::new(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Ensure a personal ordering exists for the given milestone, creating a
    /// default from current problem order if needed. Also syncs the ordering
    /// with the current problem list — any problems added since the ordering
    /// was created are appended to the end; any removed are pruned.
    fn ensure_ordering(&mut self, milestone_id: &str) {
        if !self.ui.personal_orderings.contains_key(milestone_id) {
            let default = self.default_ordering_for_milestone(milestone_id);
            self.ui
                .personal_orderings
                .insert(milestone_id.to_string(), default);
        } else {
            // Sync: add any new problems not yet in the ordering, remove stale ones
            let current_ids: Vec<String> = self
                .data
                .problems
                .iter()
                .filter(|p| p.milestone_id.as_deref() == Some(milestone_id))
                .map(|p| p.id.clone())
                .collect();
            let ordering = self
                .ui
                .personal_orderings
                .get_mut(milestone_id)
                .expect("ensure_ordering guarantees entry");
            let existing: std::collections::HashSet<String> =
                ordering.order.iter().cloned().collect();
            // Append new problems
            for id in &current_ids {
                if !existing.contains(id) {
                    ordering.order.push(id.clone());
                }
            }
            // Remove problems no longer in the milestone
            let current_set: std::collections::HashSet<&str> =
                current_ids.iter().map(|s| s.as_str()).collect();
            ordering
                .order
                .retain(|id| current_set.contains(id.as_str()));
        }
    }

    /// Ensure personal-ordering view is active before a reorder, returning the
    /// selected (milestone_id, problem_id) and the item's current position.
    fn prepare_reorder(&mut self) -> Option<(String, String, usize)> {
        let (milestone_id, problem_id) = self.selected_milestone_problem()?;

        if !self.ui.show_personal_ordering {
            self.ui.show_personal_ordering = true;
        }
        self.ensure_ordering(&milestone_id);

        let pos = self
            .ui
            .personal_orderings
            .get(&milestone_id)
            .and_then(|o| o.order.iter().position(|id| *id == problem_id))?;
        Some((milestone_id, problem_id, pos))
    }

    /// Persist a milestone's personal ordering and refresh the view, keeping the
    /// cursor on the moved problem.
    fn save_personal_ordering(&mut self, milestone_id: &str, problem_id: &str) -> Result<()> {
        let ordering = self
            .ui
            .personal_orderings
            .get(milestone_id)
            .expect("ensure_ordering guarantees entry");
        crate::ranking::ordering::save_user_ordering(
            self.store.meta_path(),
            milestone_id,
            &self.user,
            ordering,
        )?;
        self.refresh_data()?;
        self.move_cursor_to_problem(problem_id);
        Ok(())
    }

    /// Handle Shift+K: tap nudges up one slot; a second Shift+K within 400ms
    /// flings the item to the top.
    pub(in crate::tui::app) fn rank_shift_up(&mut self) -> Result<()> {
        use crate::tui::app::RankMoveDir;
        let now = std::time::Instant::now();
        let double_tap = matches!(
            self.ui.last_rank_move,
            Some((RankMoveDir::Up, t)) if now.duration_since(t) < std::time::Duration::from_millis(400)
        );
        if double_tap {
            self.ui.last_rank_move = None;
            self.send_to_top()
        } else {
            self.ui.last_rank_move = Some((RankMoveDir::Up, now));
            self.nudge_up()
        }
    }

    /// Handle Shift+J: tap nudges down one slot; a second Shift+J within 400ms
    /// flings the item to the bottom.
    pub(in crate::tui::app) fn rank_shift_down(&mut self) -> Result<()> {
        use crate::tui::app::RankMoveDir;
        let now = std::time::Instant::now();
        let double_tap = matches!(
            self.ui.last_rank_move,
            Some((RankMoveDir::Down, t)) if now.duration_since(t) < std::time::Duration::from_millis(400)
        );
        if double_tap {
            self.ui.last_rank_move = None;
            self.send_to_bottom()
        } else {
            self.ui.last_rank_move = Some((RankMoveDir::Down, now));
            self.nudge_down()
        }
    }

    /// Nudge the selected problem one slot up in the personal ordering.
    pub(in crate::tui::app) fn nudge_up(&mut self) -> Result<()> {
        let (milestone_id, problem_id, pos) = match self.prepare_reorder() {
            Some(x) => x,
            None => return Ok(()),
        };
        if pos == 0 {
            self.show_flash("Already at top");
            return Ok(());
        }
        self.push_ordering_undo(&milestone_id);
        let ordering = self
            .ui
            .personal_orderings
            .get_mut(&milestone_id)
            .expect("ensure_ordering guarantees entry");
        ordering.order.swap(pos, pos - 1);
        ordering.updated_at = chrono::Utc::now();
        self.save_personal_ordering(&milestone_id, &problem_id)
    }

    /// Nudge the selected problem one slot down in the personal ordering.
    pub(in crate::tui::app) fn nudge_down(&mut self) -> Result<()> {
        let (milestone_id, problem_id, pos) = match self.prepare_reorder() {
            Some(x) => x,
            None => return Ok(()),
        };
        let len = self
            .ui
            .personal_orderings
            .get(&milestone_id)
            .map(|o| o.order.len())
            .unwrap_or(0);
        if pos + 1 >= len {
            self.show_flash("Already at bottom");
            return Ok(());
        }
        self.push_ordering_undo(&milestone_id);
        let ordering = self
            .ui
            .personal_orderings
            .get_mut(&milestone_id)
            .expect("ensure_ordering guarantees entry");
        ordering.order.swap(pos, pos + 1);
        ordering.updated_at = chrono::Utc::now();
        self.save_personal_ordering(&milestone_id, &problem_id)
    }

    /// Send the selected problem to the top of the personal ordering ("fling up").
    pub(in crate::tui::app) fn send_to_top(&mut self) -> Result<()> {
        let (milestone_id, problem_id, pos) = match self.prepare_reorder() {
            Some(x) => x,
            None => return Ok(()),
        };
        if pos == 0 {
            self.show_flash("Already at top");
            return Ok(());
        }
        self.push_ordering_undo(&milestone_id);
        let ordering = self
            .ui
            .personal_orderings
            .get_mut(&milestone_id)
            .expect("ensure_ordering guarantees entry");
        let id = ordering.order.remove(pos);
        ordering.order.insert(0, id);
        ordering.updated_at = chrono::Utc::now();
        self.save_personal_ordering(&milestone_id, &problem_id)?;
        self.show_flash("→ Top");
        Ok(())
    }

    /// Send the selected problem to the bottom of the personal ordering ("fling down").
    pub(in crate::tui::app) fn send_to_bottom(&mut self) -> Result<()> {
        let (milestone_id, problem_id, pos) = match self.prepare_reorder() {
            Some(x) => x,
            None => return Ok(()),
        };
        let len = self
            .ui
            .personal_orderings
            .get(&milestone_id)
            .map(|o| o.order.len())
            .unwrap_or(0);
        if pos + 1 >= len {
            self.show_flash("Already at bottom");
            return Ok(());
        }
        self.push_ordering_undo(&milestone_id);
        let ordering = self
            .ui
            .personal_orderings
            .get_mut(&milestone_id)
            .expect("ensure_ordering guarantees entry");
        let id = ordering.order.remove(pos);
        ordering.order.push(id);
        ordering.updated_at = chrono::Utc::now();
        self.save_personal_ordering(&milestone_id, &problem_id)?;
        self.show_flash("→ Bottom");
        Ok(())
    }

    /// Cycle the sized gap *below* the selected problem: none → S → M → L → XL → none.
    pub(in crate::tui::app) fn cycle_gap(&mut self) -> Result<()> {
        use crate::ranking::ordering::GapSize;

        let (milestone_id, problem_id, _pos) = match self.prepare_reorder() {
            Some(x) => x,
            None => return Ok(()),
        };

        self.push_ordering_undo(&milestone_id);
        let ordering = self
            .ui
            .personal_orderings
            .get_mut(&milestone_id)
            .expect("ensure_ordering guarantees entry");

        let current = ordering.gaps.get(&problem_id).copied();
        let next = GapSize::cycle(current);
        match next {
            Some(g) => {
                ordering.gaps.insert(problem_id.clone(), g);
            }
            None => {
                ordering.gaps.remove(&problem_id);
            }
        }
        ordering.updated_at = chrono::Utc::now();
        self.save_personal_ordering(&milestone_id, &problem_id)?;

        match next {
            Some(g) => self.show_flash(&format!(
                "Gap below {}: {}",
                short_id(&problem_id),
                g.label()
            )),
            None => self.show_flash(&format!("Gap below {} cleared", short_id(&problem_id))),
        }
        Ok(())
    }

    /// Save the current ordering for a milestone onto the undo stack.
    fn push_ordering_undo(&mut self, milestone_id: &str) {
        if let Some(ordering) = self.ui.personal_orderings.get(milestone_id) {
            if self.ui.ordering_undo.len() >= 50 {
                self.ui.ordering_undo.pop_front();
            }
            self.ui
                .ordering_undo
                .push_back((milestone_id.to_string(), ordering.clone()));
        }
    }

    /// Undo the last ordering operation.
    pub(in crate::tui::app) fn undo_ordering(&mut self) -> Result<()> {
        let (milestone_id, previous) = match self.ui.ordering_undo.pop_back() {
            Some(entry) => entry,
            None => {
                self.show_flash("Nothing to undo");
                return Ok(());
            }
        };

        crate::ranking::ordering::save_user_ordering(
            self.store.meta_path(),
            &milestone_id,
            &self.user,
            &previous,
        )?;

        self.ui.personal_orderings.insert(milestone_id, previous);

        self.show_flash("Undone");
        self.refresh_data()?;
        self.update_selected_detail();
        Ok(())
    }

    /// Move the cursor to the tree item matching the given problem ID.
    fn move_cursor_to_problem(&mut self, problem_id: &str) {
        if let Some(idx) = self
            .cache
            .tree_items
            .iter()
            .position(|item| item.node.id() == problem_id)
        {
            self.ui.tree_index = idx;
            self.update_selected_detail();
        }
    }
}
