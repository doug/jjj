use crate::display::short_id;
use crate::error::Result;
use crate::tui::app::App;
use crate::tui::next_actions::EntityType;

impl App {
    pub(in crate::tui::app) fn handle_action_a(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Solution => self.approve_solution(&id)?,
                EntityType::Critique => self.address_critique(&id)?,
                // A finding has no approval state: evidence is cited or
                // contradicted, never accepted.
                EntityType::Problem | EntityType::Milestone | EntityType::Finding => {}
            }
        }
        Ok(())
    }

    pub(in crate::tui::app) fn handle_action_u(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Solution {
                self.submit_solution(&id)?;
            }
        }
        Ok(())
    }

    pub(in crate::tui::app) fn handle_action_d(&mut self) -> Result<()> {
        use crate::models::{MilestoneStatus, ProblemStatus};
        use crate::tui::tree::TreeNode;

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
                                self.ui.input_mode = crate::tui::app::InputMode::Input {
                                    prompt: "Dissolve reason: ".to_string(),
                                    buffer: String::new(),
                                    action: crate::tui::app::InputAction::DissolveP {
                                        problem_id: id.clone(),
                                    },
                                    cursor_pos: 0,
                                };
                            }
                        }
                    }
                    return Ok(());
                }
                EntityType::Finding => return Ok(()),
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
                        // Nothing to decline: a finding is superseded by a
                        // better measurement, not rejected by a decision.
                        EntityType::Finding => {}
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

    pub(in crate::tui::app) fn handle_action_s(&mut self) -> Result<()> {
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

    pub(in crate::tui::app) fn handle_action_o(&mut self) -> Result<()> {
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

    pub(in crate::tui::app) fn handle_action_v(&mut self) -> Result<()> {
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

    pub(in crate::tui::app) fn dissolve_problem(
        &mut self,
        problem_id: &str,
        reason: &str,
    ) -> Result<()> {
        let id = problem_id.to_string();
        let reason = reason.to_string();
        self.dispatch_domain(&id, "dissolved", |store| {
            crate::domain::dissolve_problem(store, &id, Some(&reason))
        })
    }

    pub(in crate::tui::app) fn handle_action_shift_a(&mut self) -> Result<()> {
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
                        // Neither carries an assignee: a critique is addressed
                        // by the solution's author, and a finding is already
                        // attributed to whoever measured it.
                        EntityType::Critique | EntityType::Finding => continue,
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
}
