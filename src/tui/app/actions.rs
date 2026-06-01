use super::super::next_actions::EntityType;
use super::{App, InputAction, InputMode};
use crate::display::short_id;
use crate::error::Result;
use crate::storage::MetadataStore;

/// Resolve the current user for TUI-originated events.
///
/// Mirrors `domain::current_user`: never returns an empty string, so the audit
/// log always has a non-empty `by` field even when jj has no user configured.
fn tui_current_user(store: &MetadataStore) -> String {
    match store.get_current_user() {
        Ok(name) if !name.trim().is_empty() => name,
        _ => "unknown".to_string(),
    }
}

impl App {
    pub(super) fn create_problem(
        &mut self,
        title: &str,
        milestone_id: Option<String>,
    ) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::{Event, EventType, Problem};

        let id = generate_id();
        let mut problem = Problem::new(id.clone(), title.to_string());
        problem.milestone_id = milestone_id.clone();

        let user = tui_current_user(&self.store);
        let event = Event::new(EventType::ProblemCreated, id.clone(), user);

        self.store
            .with_metadata(&format!("Create problem: {}", title), || {
                self.store.set_pending_event(event.clone());
                self.store.save_problem(&problem)?;
                // Mirror the CLI: register the problem on its milestone so
                // milestone.problem_ids stays in sync.
                if let Some(ref ms_id) = milestone_id {
                    let mut ms = self.store.load_milestone(ms_id)?;
                    ms.add_problem(id.clone());
                    self.store.save_milestone(&ms)?;
                }
                Ok(())
            })?;

        crate::automation::run(&self.store, &event, &id);

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn create_solution(&mut self, title: &str, problem_id: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::{Event, EventExtra, EventType, Solution};

        let id = generate_id();
        let solution = Solution::new(id.clone(), title.to_string(), problem_id.to_string());

        let user = tui_current_user(&self.store);
        let event =
            Event::new(EventType::SolutionCreated, id.clone(), user).with_extra(EventExtra {
                problem: Some(problem_id.to_string()),
                ..Default::default()
            });

        self.store
            .with_metadata(&format!("Create solution: {}", title), || {
                self.store.set_pending_event(event.clone());
                self.store.save_solution(&solution)?;
                // Mirror the CLI: register the back-reference and move the
                // problem Open -> InProgress on its first solution. Without
                // this, TUI-created solutions leave problem.solution_ids stale
                // and the problem stuck rendering as Open.
                let mut problem = self.store.load_problem(problem_id)?;
                problem.add_solution(id.clone());
                if problem.status == crate::models::ProblemStatus::Open {
                    let _ = problem.try_set_status(crate::models::ProblemStatus::InProgress);
                }
                self.store.save_problem(&problem)
            })?;

        crate::automation::run(&self.store, &event, &id);

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn create_critique(&mut self, title: &str, solution_id: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::{Critique, Event, EventExtra, EventType};

        let id = generate_id();
        let critique = Critique::new(id.clone(), title.to_string(), solution_id.to_string());

        let user = tui_current_user(&self.store);
        let event =
            Event::new(EventType::CritiqueRaised, id.clone(), user).with_extra(EventExtra {
                target: Some(solution_id.to_string()),
                title: Some(title.to_string()),
                ..Default::default()
            });

        self.store
            .with_metadata(&format!("Create critique: {}", title), || {
                self.store.set_pending_event(event.clone());
                self.store.save_critique(&critique)?;
                // Mirror the CLI: register the back-reference so the solution
                // shows the READY next-action once critiques are resolved
                // (build_next_actions keys off solution.critique_ids).
                let mut solution = self.store.load_solution(solution_id)?;
                solution.add_critique(id.clone());
                self.store.save_solution(&solution)
            })?;

        crate::automation::run(&self.store, &event, &id);

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn update_title(
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

    pub(super) fn start_new_item(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

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

    pub(super) fn start_edit_title(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

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

    pub(super) fn start_edit_tags(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

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

    pub(super) fn update_tags(
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
    pub(super) fn batch_update_tags(
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

    pub(super) fn cycle_confidence(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

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

    pub(super) fn handle_action_a(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Solution => self.approve_solution(&id)?,
                EntityType::Critique => self.address_critique(&id)?,
                EntityType::Problem | EntityType::Milestone => {}
            }
        }
        Ok(())
    }

    pub(super) fn handle_action_u(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Solution {
                self.submit_solution(&id)?;
            }
        }
        Ok(())
    }

    pub(super) fn handle_action_d(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;
        use crate::models::{MilestoneStatus, ProblemStatus};

        let targets = self.action_targets();
        if targets.is_empty() {
            return Ok(());
        }

        // Single item without multi-select: use original behavior (dissolve reason prompt for problems)
        if self.ui.selected_ids.is_empty() && targets.len() == 1 {
            let (ref id, ref entity_type) = targets[0];
            match entity_type {
                EntityType::Critique => {
                    self.dismiss_critique(id)?;
                    return Ok(());
                }
                EntityType::Solution => {
                    self.withdraw_solution(id)?;
                    return Ok(());
                }
                EntityType::Problem => {
                    if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                        if let TreeNode::Problem { status, .. } = &item.node {
                            if matches!(status, ProblemStatus::Open | ProblemStatus::InProgress) {
                                self.ui.input_mode = super::InputMode::Input {
                                    prompt: "Dissolve reason: ".to_string(),
                                    buffer: String::new(),
                                    action: super::InputAction::DissolveP {
                                        problem_id: id.clone(),
                                    },
                                    cursor_pos: 0,
                                };
                            }
                        }
                    }
                    return Ok(());
                }
                EntityType::Milestone => {
                    if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                        if let TreeNode::Milestone { status, .. } = &item.node {
                            if matches!(status, MilestoneStatus::Planning | MilestoneStatus::Active)
                            {
                                let id_clone = id.clone();
                                match self.store.with_metadata(
                                    &format!("Cancel milestone {}", id),
                                    || {
                                        let mut milestone = self.store.load_milestone(&id_clone)?;
                                        milestone.set_status(MilestoneStatus::Cancelled);
                                        self.store.save_milestone(&milestone)
                                    },
                                ) {
                                    Ok(_) => {
                                        self.show_flash(&format!("{} cancelled", short_id(id)));
                                        self.refresh_data()?;
                                    }
                                    Err(e) => self.show_flash(&format!("Error: {}", e)),
                                }
                            }
                        }
                    }
                    return Ok(());
                }
            }
        }

        // Batch mode — collect events to fire automation after commit
        use crate::models::{Event, EventType};
        let mut dismissed = 0usize;
        let mut withdrawn = 0usize;
        let mut dissolved = 0usize;
        let mut cancelled = 0usize;
        let mut batch_events: Vec<(Event, String)> = Vec::new();
        let user = self
            .store
            .get_current_user()
            .unwrap_or_else(|_| "unknown".to_string());

        self.store
            .with_metadata(&format!("Batch decline {} items", targets.len()), || {
                for (id, entity_type) in &targets {
                    match entity_type {
                        EntityType::Critique => {
                            if let Ok(mut critique) = self.store.load_critique(id) {
                                if critique.dismiss().is_ok()
                                    && self.store.save_critique(&critique).is_ok()
                                {
                                    let event = Event::new(
                                        EventType::CritiqueDismissed,
                                        id.clone(),
                                        user.clone(),
                                    );
                                    self.store.set_pending_event(event.clone());
                                    batch_events.push((event, id.clone()));
                                    dismissed += 1;
                                }
                            }
                        }
                        EntityType::Solution => {
                            if let Ok(mut solution) = self.store.load_solution(id) {
                                if let Err(e) = solution.withdraw() {
                                    crate::output::warn(&e.to_string());
                                    continue;
                                }
                                if self.store.save_solution(&solution).is_ok() {
                                    let event = Event::new(
                                        EventType::SolutionWithdrawn,
                                        id.clone(),
                                        user.clone(),
                                    );
                                    self.store.set_pending_event(event.clone());
                                    batch_events.push((event, id.clone()));
                                    withdrawn += 1;
                                }
                            }
                        }
                        EntityType::Problem => {
                            if let Ok(mut problem) = self.store.load_problem(id) {
                                if matches!(
                                    problem.status,
                                    ProblemStatus::Open | ProblemStatus::InProgress
                                ) {
                                    problem.dissolve("Batch dissolved".to_string());
                                    if self.store.save_problem(&problem).is_ok() {
                                        let event = Event::new(
                                            EventType::ProblemDissolved,
                                            id.clone(),
                                            user.clone(),
                                        );
                                        self.store.set_pending_event(event.clone());
                                        batch_events.push((event, id.clone()));
                                        dissolved += 1;
                                    }
                                }
                            }
                        }
                        EntityType::Milestone => {
                            if let Ok(mut milestone) = self.store.load_milestone(id) {
                                milestone.set_status(MilestoneStatus::Cancelled);
                                if self.store.save_milestone(&milestone).is_ok() {
                                    cancelled += 1;
                                }
                            }
                        }
                    }
                }
                Ok(())
            })?;

        // Fire automation for each batch event
        for (event, entity_id) in &batch_events {
            crate::automation::run(&self.store, event, entity_id);
        }

        let mut parts = Vec::new();
        if dismissed > 0 {
            parts.push(format!("{} dismissed", dismissed));
        }
        if withdrawn > 0 {
            parts.push(format!("{} withdrawn", withdrawn));
        }
        if dissolved > 0 {
            parts.push(format!("{} dissolved", dissolved));
        }
        if cancelled > 0 {
            parts.push(format!("{} cancelled", cancelled));
        }
        if !parts.is_empty() {
            self.show_flash(&parts.join(", "));
        }

        self.ui.selected_ids.clear();
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn handle_action_s(&mut self) -> Result<()> {
        use crate::models::{Event, EventType, MilestoneStatus, ProblemStatus};

        let targets = self.action_targets();
        if targets.is_empty() {
            return Ok(());
        }

        let mut solved = 0usize;
        let mut completed = 0usize;
        let mut errors = Vec::new();
        let mut batch_events: Vec<(Event, String)> = Vec::new();
        let user = self
            .store
            .get_current_user()
            .unwrap_or_else(|_| "unknown".to_string());

        self.store.with_metadata(
            &format!("Batch solve/complete {} items", targets.len()),
            || {
                for (id, entity_type) in &targets {
                    match entity_type {
                        EntityType::Problem => {
                            match (|| -> crate::error::Result<()> {
                                let (can_solve, message) = self.store.can_solve_problem(id)?;
                                if !can_solve {
                                    return Err(crate::error::JjjError::CannotSolveProblem(
                                        message,
                                    ));
                                }
                                let mut problem = self.store.load_problem(id)?;
                                problem
                                    .try_set_status(ProblemStatus::Solved)
                                    .map_err(crate::error::JjjError::Validation)?;
                                self.store.save_problem(&problem)?;
                                let event =
                                    Event::new(EventType::ProblemSolved, id.clone(), user.clone());
                                self.store.set_pending_event(event.clone());
                                batch_events.push((event, id.clone()));
                                Ok(())
                            })() {
                                Ok(_) => solved += 1,
                                Err(e) => errors.push(format!("{}: {}", short_id(id), e)),
                            }
                        }
                        EntityType::Milestone => {
                            match (|| -> crate::error::Result<()> {
                                let mut milestone = self.store.load_milestone(id)?;
                                let was_completed = milestone.status == MilestoneStatus::Completed;
                                milestone.set_status(MilestoneStatus::Completed);
                                self.store.save_milestone(&milestone)?;
                                // Emit a completion event (matching the CLI) so
                                // the consistency checker stays satisfied.
                                if !was_completed {
                                    let event = Event::new(
                                        EventType::MilestoneCompleted,
                                        id.clone(),
                                        user.clone(),
                                    );
                                    self.store.set_pending_event(event.clone());
                                    batch_events.push((event, id.clone()));
                                }
                                Ok(())
                            })() {
                                Ok(_) => completed += 1,
                                Err(e) => errors.push(format!("{}: {}", short_id(id), e)),
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            },
        )?;

        // Fire automation for each batch event
        for (event, entity_id) in &batch_events {
            crate::automation::run(&self.store, event, entity_id);
        }

        // Build flash message
        let mut parts = Vec::new();
        if solved > 0 {
            parts.push(format!("{} solved", solved));
        }
        if completed > 0 {
            parts.push(format!("{} completed", completed));
        }
        if !errors.is_empty() {
            parts.push(format!("{} errors", errors.len()));
        }
        if !parts.is_empty() {
            self.show_flash(&parts.join(", "));
        }

        self.ui.selected_ids.clear();
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn handle_action_o(&mut self) -> Result<()> {
        use crate::models::MilestoneStatus;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Problem => {
                    let id_clone = id.clone();
                    match crate::domain::reopen_problem(&self.store, &id) {
                        Ok(_) => {
                            self.show_flash(&format!("{} reopened", id_clone));
                            self.refresh_data()?;
                        }
                        Err(e) => {
                            self.show_flash(&format!("Error: {}", e));
                        }
                    }
                }
                EntityType::Milestone => {
                    let id_clone = id.clone();
                    match self
                        .store
                        .with_metadata(&format!("Activate milestone {}", id), || {
                            let mut milestone = self.store.load_milestone(&id)?;
                            milestone.set_status(MilestoneStatus::Active);
                            self.store.save_milestone(&milestone)
                        }) {
                        Ok(_) => {
                            self.show_flash(&format!("{} activated", id_clone));
                            self.refresh_data()?;
                        }
                        Err(e) => {
                            self.show_flash(&format!("Error: {}", e));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn handle_action_v(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Critique {
                let id = id.clone();
                return self.dispatch_domain(&id, "validated", |store| {
                    crate::domain::validate_critique(store, &id)
                });
            }
        }
        Ok(())
    }

    /// Run a domain action, then either flash a success message + refresh or
    /// flash the error message. Centralizes the "domain call → flash →
    /// refresh" pattern shared by every transition action below.
    fn dispatch_domain<F>(&mut self, id: &str, verb: &str, f: F) -> Result<()>
    where
        F: FnOnce(&crate::storage::MetadataStore) -> Result<()>,
    {
        match f(&self.store) {
            Ok(_) => {
                self.show_flash(&format!("{} {}", id, verb));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn approve_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        self.dispatch_domain(&id, "approved", |store| {
            crate::domain::approve_solution(store, &id, false, None)
        })
    }

    fn withdraw_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        self.dispatch_domain(&id, "withdrawn", |store| {
            crate::domain::withdraw_solution(store, &id, None)
        })
    }

    fn submit_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        self.dispatch_domain(&id, "submitted for review", |store| {
            crate::domain::submit_solution(store, &id)
        })
    }

    fn address_critique(&mut self, critique_id: &str) -> Result<()> {
        let id = critique_id.to_string();
        self.dispatch_domain(&id, "addressed", |store| {
            crate::domain::address_critique(store, &id)
        })
    }

    fn dismiss_critique(&mut self, critique_id: &str) -> Result<()> {
        let id = critique_id.to_string();
        self.dispatch_domain(&id, "dismissed", |store| {
            crate::domain::dismiss_critique(store, &id)
        })
    }

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

    pub(super) fn dissolve_problem(&mut self, problem_id: &str, reason: &str) -> Result<()> {
        let id = problem_id.to_string();
        let reason = reason.to_string();
        self.dispatch_domain(&id, "dissolved", |store| {
            crate::domain::dissolve_problem(store, &id, Some(&reason))
        })
    }

    pub(super) fn handle_action_shift_a(&mut self) -> Result<()> {
        let user = self
            .store
            .get_current_user()
            .unwrap_or_else(|_| "unknown".to_string());

        let targets = self.action_targets();
        if targets.is_empty() {
            return Ok(());
        }

        let mut assigned = 0usize;

        self.store.with_metadata(
            &format!("Batch assign {} items to {}", targets.len(), user),
            || {
                for (id, entity_type) in &targets {
                    let result: crate::error::Result<()> = match entity_type {
                        EntityType::Problem => {
                            let mut problem = self.store.load_problem(id)?;
                            problem.assignee = Some(user.clone());
                            self.store.save_problem(&problem)
                        }
                        EntityType::Solution => {
                            let mut solution = self.store.load_solution(id)?;
                            solution.assignee = Some(user.clone());
                            self.store.save_solution(&solution)
                        }
                        EntityType::Milestone => {
                            let mut milestone = self.store.load_milestone(id)?;
                            milestone.assignee = Some(user.clone());
                            self.store.save_milestone(&milestone)
                        }
                        EntityType::Critique => continue,
                    };
                    if result.is_ok() {
                        assigned += 1;
                    }
                }
                Ok(())
            },
        )?;

        // Extract short name for flash
        let name = user.split('<').next().unwrap_or(&user).trim();
        self.show_flash(&format!("{} assigned to {}", assigned, name));
        self.ui.selected_ids.clear();
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn start_delete(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

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

    pub(super) fn batch_delete(&mut self, entities: &[(String, String)]) -> Result<()> {
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

    pub(super) fn delete_entity(&mut self, entity_type: &str, entity_id: &str) -> Result<()> {
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

    pub(super) fn start_move_to_milestone(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

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

    pub(super) fn batch_move_to_milestone(
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

    pub(super) fn move_problem_to_milestone(
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

    pub(super) fn create_milestone(&mut self, title: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Milestone;

        let id = generate_id();
        let milestone = Milestone::new(id.clone(), title);

        self.store
            .with_metadata(&format!("Create milestone: {}", title), || {
                self.store.save_milestone(&milestone)
            })?;

        self.show_flash(&format!("Created milestone {}", id));
        self.refresh_data()?;
        Ok(())
    }

    /// Toggle between personal and global ordering view.
    pub(super) fn toggle_ordering_view(&mut self) {
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
    pub(super) fn rank_shift_up(&mut self) -> Result<()> {
        use super::RankMoveDir;
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
    pub(super) fn rank_shift_down(&mut self) -> Result<()> {
        use super::RankMoveDir;
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
    pub(super) fn nudge_up(&mut self) -> Result<()> {
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
    pub(super) fn nudge_down(&mut self) -> Result<()> {
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
    pub(super) fn send_to_top(&mut self) -> Result<()> {
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
    pub(super) fn send_to_bottom(&mut self) -> Result<()> {
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
    pub(super) fn cycle_gap(&mut self) -> Result<()> {
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
    pub(super) fn undo_ordering(&mut self) -> Result<()> {
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
