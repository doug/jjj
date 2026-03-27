use super::super::next_actions::EntityType;
use super::{App, InputAction, InputMode};
use crate::display::short_id;
use crate::error::Result;
use crate::models::{CritiqueStatus, Event, EventExtra, EventType, ProblemStatus};

impl App {
    pub(super) fn create_problem(
        &mut self,
        title: &str,
        milestone_id: Option<String>,
    ) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Problem;

        let id = generate_id();
        let mut problem = Problem::new(id.clone(), title.to_string());
        problem.milestone_id = milestone_id;

        self.store
            .with_metadata(&format!("Create problem: {}", title), || {
                self.store.save_problem(&problem)
            })?;

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn create_solution(&mut self, title: &str, problem_id: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Solution;

        let id = generate_id();
        let solution = Solution::new(id.clone(), title.to_string(), problem_id.to_string());

        self.store
            .with_metadata(&format!("Create solution: {}", title), || {
                self.store.save_solution(&solution)
            })?;

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn create_critique(&mut self, title: &str, solution_id: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Critique;

        let id = generate_id();
        let critique = Critique::new(id.clone(), title.to_string(), solution_id.to_string());

        self.store
            .with_metadata(&format!("Create critique: {}", title), || {
                self.store.save_critique(&critique)
            })?;

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

        let cursor_pos = current_title.len();
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

        let cursor_pos = current_tags.len();
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

        // Batch mode
        let mut dismissed = 0usize;
        let mut withdrawn = 0usize;
        let mut dissolved = 0usize;
        let mut cancelled = 0usize;

        self.store
            .with_metadata(&format!("Batch decline {} items", targets.len()), || {
                for (id, entity_type) in &targets {
                    match entity_type {
                        EntityType::Critique => {
                            if let Ok(mut critique) = self.store.load_critique(id) {
                                critique.dismiss();
                                if self.store.save_critique(&critique).is_ok() {
                                    dismissed += 1;
                                }
                            }
                        }
                        EntityType::Solution => {
                            if let Ok(mut solution) = self.store.load_solution(id) {
                                if let Err(e) = solution.withdraw() {
                                    eprintln!("Warning: {}", e);
                                    continue;
                                }
                                if self.store.save_solution(&solution).is_ok() {
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
        use crate::models::{MilestoneStatus, ProblemStatus};

        let targets = self.action_targets();
        if targets.is_empty() {
            return Ok(());
        }

        let mut solved = 0usize;
        let mut completed = 0usize;
        let mut errors = Vec::new();

        self.store.with_metadata(
            &format!("Batch solve/complete {} items", targets.len()),
            || {
                for (id, entity_type) in &targets {
                    match entity_type {
                        EntityType::Problem => {
                            match (|| -> crate::error::Result<()> {
                                let mut problem = self.store.load_problem(id)?;
                                problem
                                    .try_set_status(ProblemStatus::Solved)
                                    .map_err(crate::error::JjjError::Validation)?;
                                self.store.save_problem(&problem)
                            })() {
                                Ok(_) => solved += 1,
                                Err(e) => errors.push(format!("{}: {}", short_id(id), e)),
                            }
                        }
                        EntityType::Milestone => {
                            match (|| -> crate::error::Result<()> {
                                let mut milestone = self.store.load_milestone(id)?;
                                milestone.set_status(MilestoneStatus::Completed);
                                self.store.save_milestone(&milestone)
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
        use crate::models::{MilestoneStatus, ProblemStatus};

        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Problem => {
                    let id_clone = id.clone();
                    match self
                        .store
                        .with_metadata(&format!("Reopen problem {}", id), || {
                            let mut problem = self.store.load_problem(&id)?;
                            problem
                                .try_set_status(ProblemStatus::Open)
                                .map_err(crate::error::JjjError::Validation)?;
                            self.store.save_problem(&problem)
                        }) {
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
                let id_clone = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Validate critique {}", id), || {
                        let mut critique = self.store.load_critique(&id)?;
                        critique.validate();
                        self.store.save_critique(&critique)
                    }) {
                    Ok(_) => {
                        self.show_flash(&format!("{} validated", id_clone));
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

    fn approve_solution(&mut self, solution_id: &str) -> Result<()> {
        // Block if there are open or valid critiques
        let blocking_critiques = self
            .store
            .list_critiques()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| {
                c.solution_id == solution_id
                    && matches!(c.status, CritiqueStatus::Open | CritiqueStatus::Valid)
            })
            .count();
        if blocking_critiques > 0 {
            self.show_flash(&format!(
                "Blocked: {} unresolved critique(s) must be addressed first",
                blocking_critiques
            ));
            return Ok(());
        }

        let id = solution_id.to_string();
        let user = self
            .store
            .get_current_user()
            .unwrap_or_else(|_| "unknown".to_string());
        match self
            .store
            .with_metadata(&format!("Approve solution {}", solution_id), || {
                let event = Event::new(
                    EventType::SolutionApproved,
                    solution_id.to_string(),
                    user.clone(),
                );
                self.store.set_pending_event(event.clone());
                let mut solution = self.store.load_solution(solution_id)?;
                solution
                    .approve()
                    .map_err(crate::error::JjjError::Validation)?;
                self.store.save_solution(&solution)?;
                // Auto-solve problem
                let (can_solve, _) = self.store.can_solve_problem(&solution.problem_id)?;
                if can_solve {
                    let mut problem = self.store.load_problem(&solution.problem_id)?;
                    if problem.status != ProblemStatus::Solved {
                        problem
                            .try_set_status(ProblemStatus::Solved)
                            .map_err(crate::error::JjjError::Validation)?;
                        self.store.save_problem(&problem)?;
                        let solve_event =
                            Event::new(EventType::ProblemSolved, problem.id.clone(), user.clone());
                        self.store.set_pending_event(solve_event);
                    }
                }
                Ok(())
            }) {
            Ok(_) => {
                self.show_flash(&format!("{} approved", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn withdraw_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        match self
            .store
            .with_metadata(&format!("Withdraw solution {}", solution_id), || {
                let mut solution = self.store.load_solution(solution_id)?;
                solution
                    .withdraw()
                    .map_err(crate::error::JjjError::Validation)?;
                self.store.save_solution(&solution)?;
                Ok(())
            }) {
            Ok(_) => {
                self.show_flash(&format!("{} withdrawn", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn submit_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        let user = self
            .store
            .get_current_user()
            .unwrap_or_else(|_| "unknown".to_string());
        match self.store.with_metadata(
            &format!("Submit solution {} for review", solution_id),
            || {
                let mut solution = self.store.load_solution(solution_id)?;
                solution
                    .submit()
                    .map_err(crate::error::JjjError::Validation)?;
                self.store.save_solution(&solution)?;
                let event = Event::new(
                    EventType::SolutionSubmitted,
                    solution_id.to_string(),
                    user.clone(),
                )
                .with_extra(EventExtra {
                    problem: Some(solution.problem_id.clone()),
                    ..Default::default()
                });
                self.store.set_pending_event(event);
                // Auto-set problem to InProgress if it's Open
                let mut problem = self.store.load_problem(&solution.problem_id)?;
                if problem.status == ProblemStatus::Open {
                    problem
                        .try_set_status(ProblemStatus::InProgress)
                        .map_err(crate::error::JjjError::Validation)?;
                    self.store.save_problem(&problem)?;
                }
                Ok(())
            },
        ) {
            Ok(_) => {
                self.show_flash(&format!("{} submitted for review", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn address_critique(&mut self, critique_id: &str) -> Result<()> {
        let id = critique_id.to_string();
        match self
            .store
            .with_metadata(&format!("Address critique {}", critique_id), || {
                let mut critique = self.store.load_critique(critique_id)?;
                critique.address();
                self.store.save_critique(&critique)?;
                Ok(())
            }) {
            Ok(_) => {
                self.show_flash(&format!("{} addressed", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn dismiss_critique(&mut self, critique_id: &str) -> Result<()> {
        let id = critique_id.to_string();
        match self
            .store
            .with_metadata(&format!("Dismiss critique {}", critique_id), || {
                let mut critique = self.store.load_critique(critique_id)?;
                critique.dismiss();
                self.store.save_critique(&critique)?;
                Ok(())
            }) {
            Ok(_) => {
                self.show_flash(&format!("{} dismissed", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
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
        match self
            .store
            .with_metadata(&format!("Dissolve problem {}", problem_id), || {
                let mut problem = self.store.load_problem(problem_id)?;
                problem.dissolve(reason.to_string());
                self.store.save_problem(&problem)
            }) {
            Ok(_) => {
                self.show_flash(&format!("{} dissolved", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
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
        let count = entities.len();

        self.store
            .with_metadata(&format!("Batch delete {} items", count), || {
                for (entity_type, entity_id) in entities {
                    match entity_type.as_str() {
                        "critique" => {
                            let _ = self.store.delete_critique(entity_id);
                        }
                        "solution" => {
                            let _ = self.store.delete_solution(entity_id);
                        }
                        "problem" => {
                            let _ = self.store.delete_problem(entity_id);
                        }
                        "milestone" => {
                            let _ = self.store.delete_milestone(entity_id);
                        }
                        _ => {}
                    }
                }
                Ok(())
            })?;

        self.show_flash(&format!("Deleted {} items", count));
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

}
