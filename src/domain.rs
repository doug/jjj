//! Shared domain operations.
//!
//! These functions encode the core business logic for entity lifecycle
//! transitions. Both the CLI command handlers and the TUI action handlers
//! call into this module, ensuring consistent validation, event emission,
//! and automation dispatch regardless of interface.

use crate::error::{JjjError, Result};
use crate::models::{
    CritiqueStatus, Event, EventExtra, EventType, ProblemStatus, SolutionStatus,
};
use crate::storage::MetadataStore;

// ── Solution operations ────────────────────────────────────────────

/// Approve a solution: check critiques, validate state, emit events,
/// auto-solve parent problem, and fire automation.
///
/// Does NOT merge PRs or squash code — that is the caller's responsibility.
pub fn approve_solution(
    store: &MetadataStore,
    solution_id: &str,
    force: bool,
    rationale: Option<&str>,
) -> Result<()> {
    let solution = store.load_solution(solution_id)?;

    // Already approved — idempotent
    if solution.status == SolutionStatus::Approved {
        return Ok(());
    }

    // Must be Submitted (or force)
    if solution.status != SolutionStatus::Submitted && !force {
        return Err(JjjError::Validation(format!(
            "Solution '{}' is {} — submit it for review first:\n  jjj solution submit {}",
            solution.title, solution.status, solution_id,
        )));
    }

    // Check open critiques
    let critiques = store.list_critiques()?;
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| {
            c.solution_id == solution_id
                && matches!(c.status, CritiqueStatus::Open | CritiqueStatus::Valid)
        })
        .collect();

    if !open_critiques.is_empty() && !force {
        return Err(JjjError::CannotApproveSolution(format!(
            "{} open critique(s) must be addressed first",
            open_critiques.len()
        )));
    }

    let user = store.get_current_user().unwrap_or_default();
    let mut event = Event::new(
        EventType::SolutionApproved,
        solution_id.to_string(),
        user.clone(),
    );
    if let Some(r) = rationale {
        event = event.with_rationale(r);
    }

    let problem_id = solution.problem_id.clone();

    store.with_metadata(&format!("Approve solution {}", solution_id), || {
        store.set_pending_event(event.clone());
        let mut sol = store.load_solution(solution_id)?;
        if force {
            sol.force_approved = true;
        }
        sol.approve().map_err(JjjError::Validation)?;
        store.save_solution(&sol)?;

        // Auto-solve parent problem
        let (can_solve, _) = store.can_solve_problem(&sol.problem_id)?;
        if can_solve {
            let mut problem = store.load_problem(&sol.problem_id)?;
            if problem.status != ProblemStatus::Solved {
                problem
                    .try_set_status(ProblemStatus::Solved)
                    .map_err(JjjError::Validation)?;
                store.save_problem(&problem)?;
                let solve_event = Event::new(
                    EventType::ProblemSolved,
                    problem.id.clone(),
                    event.by.clone(),
                );
                store.set_pending_event(solve_event);
            }
        }
        Ok(())
    })?;

    // Fire automation
    let approve_event = Event::new(
        EventType::SolutionApproved,
        solution_id.to_string(),
        user.clone(),
    );
    crate::automation::run(store, &approve_event, solution_id);

    // Fire problem-solved automation if auto-solve triggered
    if let Ok(problem) = store.load_problem(&problem_id) {
        if problem.status == ProblemStatus::Solved {
            let solve_event =
                Event::new(EventType::ProblemSolved, problem.id.clone(), user);
            crate::automation::run(store, &solve_event, &problem.id);
        }
    }

    Ok(())
}

/// Submit a solution for review: validate state, emit events,
/// auto-set parent problem to InProgress, and fire automation.
pub fn submit_solution(store: &MetadataStore, solution_id: &str) -> Result<()> {
    let user = store.get_current_user().unwrap_or_default();

    store.with_metadata(
        &format!("Submit solution {} for review", solution_id),
        || {
            let mut solution = store.load_solution(solution_id)?;
            solution.submit().map_err(JjjError::Validation)?;
            store.save_solution(&solution)?;

            let event = Event::new(
                EventType::SolutionSubmitted,
                solution_id.to_string(),
                user.clone(),
            )
            .with_extra(EventExtra {
                problem: Some(solution.problem_id.clone()),
                ..Default::default()
            });
            store.set_pending_event(event);

            // Auto-set problem to InProgress if still Open
            let mut problem = store.load_problem(&solution.problem_id)?;
            if problem.status == ProblemStatus::Open {
                problem
                    .try_set_status(ProblemStatus::InProgress)
                    .map_err(JjjError::Validation)?;
                store.save_problem(&problem)?;
            }
            Ok(())
        },
    )?;

    let event = Event::new(
        EventType::SolutionSubmitted,
        solution_id.to_string(),
        user,
    );
    crate::automation::run(store, &event, solution_id);

    Ok(())
}

/// Withdraw a solution: validate state, emit events, fire automation.
pub fn withdraw_solution(
    store: &MetadataStore,
    solution_id: &str,
    rationale: Option<&str>,
) -> Result<()> {
    let user = store.get_current_user().unwrap_or_default();
    let mut event = Event::new(
        EventType::SolutionWithdrawn,
        solution_id.to_string(),
        user,
    );
    if let Some(r) = rationale {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Withdraw solution {}", solution_id), || {
        store.set_pending_event(event.clone());
        let mut solution = store.load_solution(solution_id)?;
        solution.withdraw().map_err(JjjError::Validation)?;
        store.save_solution(&solution)?;
        Ok(())
    })?;

    let auto_event = Event::new(
        EventType::SolutionWithdrawn,
        solution_id.to_string(),
        event.by.clone(),
    );
    crate::automation::run(store, &auto_event, solution_id);

    Ok(())
}

// ── Problem operations ─────────────────────────────────────────────

/// Solve a problem: check for approved solution, emit events, fire automation.
pub fn solve_problem(store: &MetadataStore, problem_id: &str) -> Result<()> {
    let (can_solve, message) = store.can_solve_problem(problem_id)?;
    let problem = store.load_problem(problem_id)?;

    if !can_solve {
        if problem.status == ProblemStatus::Solved {
            return Ok(()); // idempotent
        }
        return Err(JjjError::CannotSolveProblem(message));
    }

    let user = store.get_current_user()?;
    let event = Event::new(EventType::ProblemSolved, problem_id.to_string(), user.clone());

    store.with_metadata(&format!("Solve problem {}", problem_id), || {
        store.set_pending_event(event.clone());
        let mut p = store.load_problem(problem_id)?;
        p.try_set_status(ProblemStatus::Solved)
            .map_err(JjjError::Validation)?;
        store.save_problem(&p)?;
        Ok(())
    })?;

    let event = Event::new(EventType::ProblemSolved, problem_id.to_string(), user);
    crate::automation::run(store, &event, problem_id);

    Ok(())
}

/// Dissolve a problem: mark as based on false premises, emit events, fire automation.
pub fn dissolve_problem(
    store: &MetadataStore,
    problem_id: &str,
    reason: Option<&str>,
) -> Result<()> {
    let user = store.get_current_user()?;
    let mut event = Event::new(
        EventType::ProblemDissolved,
        problem_id.to_string(),
        user.clone(),
    );
    if let Some(r) = reason {
        event = event.with_rationale(r);
    }

    store.with_metadata(&format!("Dissolve problem {}", problem_id), || {
        store.set_pending_event(event.clone());
        let mut problem = store.load_problem(problem_id)?;
        if let Some(r) = reason {
            problem.dissolve(r);
        } else {
            problem
                .try_set_status(ProblemStatus::Dissolved)
                .map_err(JjjError::Validation)?;
        }
        store.save_problem(&problem)?;
        Ok(())
    })?;

    let event = Event::new(
        EventType::ProblemDissolved,
        problem_id.to_string(),
        user,
    );
    crate::automation::run(store, &event, problem_id);

    Ok(())
}

/// Reopen a previously solved or dissolved problem, emit events, fire automation.
pub fn reopen_problem(store: &MetadataStore, problem_id: &str) -> Result<()> {
    let user = store.get_current_user()?;
    let event = Event::new(
        EventType::ProblemReopened,
        problem_id.to_string(),
        user.clone(),
    );

    store.with_metadata(&format!("Reopen problem {}", problem_id), || {
        store.set_pending_event(event.clone());
        let mut problem = store.load_problem(problem_id)?;
        problem
            .try_set_status(ProblemStatus::Open)
            .map_err(JjjError::Validation)?;
        store.save_problem(&problem)?;
        Ok(())
    })?;

    let event = Event::new(
        EventType::ProblemReopened,
        problem_id.to_string(),
        user,
    );
    crate::automation::run(store, &event, problem_id);

    Ok(())
}

// ── Critique operations ────────────────────────────────────────────

/// Address a critique: validate state, emit events.
pub fn address_critique(store: &MetadataStore, critique_id: &str) -> Result<()> {
    let user = store.get_current_user().unwrap_or_default();
    let event = Event::new(
        EventType::CritiqueAddressed,
        critique_id.to_string(),
        user,
    );

    store.with_metadata(&format!("Address critique {}", critique_id), || {
        store.set_pending_event(event.clone());
        let mut critique = store.load_critique(critique_id)?;
        critique.address().map_err(JjjError::Validation)?;
        store.save_critique(&critique)?;
        Ok(())
    })
}

/// Validate a critique: confirm it's correct, emit events.
pub fn validate_critique(store: &MetadataStore, critique_id: &str) -> Result<()> {
    let user = store.get_current_user().unwrap_or_default();
    let event = Event::new(
        EventType::CritiqueValidated,
        critique_id.to_string(),
        user,
    );

    store.with_metadata(&format!("Validate critique {}", critique_id), || {
        store.set_pending_event(event.clone());
        let mut critique = store.load_critique(critique_id)?;
        critique.validate().map_err(JjjError::Validation)?;
        store.save_critique(&critique)?;
        Ok(())
    })
}

/// Dismiss a critique: mark as incorrect or irrelevant, emit events.
pub fn dismiss_critique(store: &MetadataStore, critique_id: &str) -> Result<()> {
    let user = store.get_current_user().unwrap_or_default();
    let event = Event::new(
        EventType::CritiqueDismissed,
        critique_id.to_string(),
        user,
    );

    store.with_metadata(&format!("Dismiss critique {}", critique_id), || {
        store.set_pending_event(event.clone());
        let mut critique = store.load_critique(critique_id)?;
        critique.dismiss().map_err(JjjError::Validation)?;
        store.save_critique(&critique)?;
        Ok(())
    })
}
