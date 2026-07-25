use crate::error::Result;
use crate::storage::MetadataStore;
use crate::tui::app::App;

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
    pub(in crate::tui::app) fn create_problem(
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

    pub(in crate::tui::app) fn create_solution(
        &mut self,
        title: &str,
        problem_id: &str,
    ) -> Result<()> {
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

    pub(in crate::tui::app) fn create_critique(
        &mut self,
        title: &str,
        solution_id: &str,
    ) -> Result<()> {
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

    pub(in crate::tui::app) fn create_milestone(&mut self, title: &str) -> Result<()> {
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
