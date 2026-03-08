use super::super::next_actions::EntityType;
use super::{App, InputAction, InputMode};
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
        }
        self.show_flash(&format!("Updated title: {}", new_title));
        self.refresh_data()?;
        Ok(())
    }

    pub(super) fn start_new_item(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

        let (prompt, action) = if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            match &item.node {
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

    pub(super) fn handle_action_a(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Solution => self.approve_solution(&id)?,
                EntityType::Critique => self.address_critique(&id)?,
                EntityType::Problem => {} // No 'a' action for problems
            }
        }
        Ok(())
    }

    pub(super) fn handle_action_r(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Solution {
                self.withdraw_solution(&id)?;
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
        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Critique {
                self.dismiss_critique(&id)?;
            }
            // For problems, 'd' would be dissolve - add later with input
        }
        Ok(())
    }

    pub(super) fn handle_action_s(&mut self) -> Result<()> {
        use crate::models::ProblemStatus;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Problem {
                let id_clone = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Solve problem {}", id), || {
                        let mut problem = self.store.load_problem(&id)?;
                        problem.set_status(ProblemStatus::Solved);
                        self.store.save_problem(&problem)
                    }) {
                    Ok(_) => {
                        self.show_flash(&format!("{} solved", id_clone));
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

    pub(super) fn handle_action_o(&mut self) -> Result<()> {
        use crate::models::ProblemStatus;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Problem {
                let id_clone = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Reopen problem {}", id), || {
                        let mut problem = self.store.load_problem(&id)?;
                        problem.set_status(ProblemStatus::Open);
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
        // Block if there are open critiques
        let open_critiques = self
            .store
            .list_critiques()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.solution_id == solution_id && c.status == CritiqueStatus::Open)
            .count();
        if open_critiques > 0 {
            self.show_flash(&format!(
                "Blocked: {} open critique(s) must be resolved first",
                open_critiques
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
                solution.approve();
                self.store.save_solution(&solution)?;
                // Auto-solve problem
                let (can_solve, _) = self.store.can_solve_problem(&solution.problem_id)?;
                if can_solve {
                    let mut problem = self.store.load_problem(&solution.problem_id)?;
                    if problem.status != ProblemStatus::Solved {
                        problem.set_status(ProblemStatus::Solved);
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
                solution.withdraw();
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
                solution.submit();
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
                    problem.set_status(ProblemStatus::InProgress);
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
        use super::ProjectData;
        self.data = ProjectData::load(&self.store)?;
        self.ui.related_cache.clear();
        self.rebuild_cache();
        // Clamp tree_index to valid range after data change
        let max_index = self.cache.tree_items.len().saturating_sub(1);
        if self.ui.tree_index > max_index {
            self.ui.tree_index = max_index;
        }
        Ok(())
    }

    fn rebuild_cache(&mut self) {
        self.cache.next_actions = super::super::build_next_actions(
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

    pub(super) fn handle_action_shift_d(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;
        use crate::models::ProblemStatus;

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            if let TreeNode::Problem { id, status, .. } = &item.node {
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
        Ok(())
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

        if let Some((id, entity_type)) = self.get_selected_entity() {
            let id_clone = id.clone();
            let user_clone = user.clone();
            match entity_type {
                EntityType::Problem => {
                    match self.store.with_metadata(
                        &format!("Assign problem {} to {}", id, user),
                        || {
                            let mut problem = self.store.load_problem(&id)?;
                            problem.assignee = Some(user.clone());
                            self.store.save_problem(&problem)
                        },
                    ) {
                        Ok(_) => {
                            self.show_flash(&format!("{} assigned to {}", id_clone, user_clone));
                            self.refresh_data()?;
                        }
                        Err(e) => self.show_flash(&format!("Error: {}", e)),
                    }
                }
                EntityType::Solution => {
                    match self.store.with_metadata(
                        &format!("Assign solution {} to {}", id, user),
                        || {
                            let mut solution = self.store.load_solution(&id)?;
                            solution.assignee = Some(user.clone());
                            self.store.save_solution(&solution)
                        },
                    ) {
                        Ok(_) => {
                            self.show_flash(&format!("{} assigned to {}", id_clone, user_clone));
                            self.refresh_data()?;
                        }
                        Err(e) => self.show_flash(&format!("Error: {}", e)),
                    }
                }
                EntityType::Critique => {}
            }
        }
        Ok(())
    }

    pub(super) fn start_delete(&mut self) -> Result<()> {
        use super::super::tree::TreeNode;

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            let (entity_type, entity_id, title) = match &item.node {
                TreeNode::Critique { id, title, .. } => ("critique".to_string(), id.clone(), title.clone()),
                TreeNode::Solution { id, title, .. } => {
                    // Block if has critiques
                    let has_critiques = self
                        .data
                        .critiques
                        .iter()
                        .any(|c| c.solution_id == *id);
                    if has_critiques {
                        self.show_flash("Delete critiques first");
                        return Ok(());
                    }
                    ("solution".to_string(), id.clone(), title.clone())
                }
                TreeNode::Problem { id, title, .. } => {
                    // Block if has solutions
                    let has_solutions = self
                        .data
                        .solutions
                        .iter()
                        .any(|s| s.problem_id == *id);
                    if has_solutions {
                        self.show_flash("Delete solutions first");
                        return Ok(());
                    }
                    ("problem".to_string(), id.clone(), title.clone())
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

    pub(super) fn start_new_milestone(&mut self) -> Result<()> {
        self.ui.input_mode = InputMode::Input {
            prompt: "New milestone title: ".to_string(),
            buffer: String::new(),
            action: InputAction::NewMilestone,
            cursor_pos: 0,
        };
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
