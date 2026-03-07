use crate::cli::EventsAction;
use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{CritiqueStatus, EventType, MilestoneStatus, ProblemStatus, SolutionStatus};
use chrono::{NaiveDate, TimeZone, Utc};
use std::collections::{HashMap, HashSet};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    ctx: &CommandContext,
    action: Option<EventsAction>,
    from: Option<String>,
    to: Option<String>,
    problem: Option<String>,
    solution: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    since: Option<String>,
    json: bool,
    limit: usize,
) -> Result<()> {
    match action {
        Some(EventsAction::Rebuild) => rebuild_events(ctx),
        Some(EventsAction::Validate) => validate_events(ctx),
        None => list_events(
            ctx, from, to, problem, solution, event_type, search, since, json, limit,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn list_events(
    ctx: &CommandContext,
    from: Option<String>,
    to: Option<String>,
    problem: Option<String>,
    solution: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    since: Option<String>,
    json: bool,
    limit: usize,
) -> Result<()> {
    let store = &ctx.store;

    let mut events = store.list_events()?;

    // Resolve --problem and --solution inputs to IDs (fuzzy title/prefix/UUID)
    let problem = match problem {
        Some(ref input) => {
            // Best-effort resolution: if it fails, fall back to raw string so
            // plain UUIDs still work in scripts that already have the ID.
            match ctx.resolve_problem(input) {
                Ok(id) => Some(id),
                Err(_) => Some(input.clone()),
            }
        }
        None => None,
    };
    let solution = match solution {
        Some(ref input) => match ctx.resolve_solution(input) {
            Ok(id) => Some(id),
            Err(_) => Some(input.clone()),
        },
        None => None,
    };

    // Since filter (RFC3339 timestamp, more precise than from/to)
    if let Some(ref since_str) = since {
        if let Ok(since_ts) = chrono::DateTime::parse_from_rfc3339(since_str) {
            let since_utc = since_ts.with_timezone(&Utc);
            events.retain(|e| e.when > since_utc);
        }
    }

    // Parse date filters
    let from_date = from.as_ref().and_then(|s| parse_date_filter(s));
    let to_date = to.as_ref().and_then(|s| parse_date_filter(s));

    // Apply filters
    events.retain(|e| {
        // Date filters
        if let Some(ref fd) = from_date {
            if e.when < *fd {
                return false;
            }
        }
        if let Some(ref td) = to_date {
            if e.when > *td {
                return false;
            }
        }

        // Entity filters
        if let Some(ref p) = problem {
            if e.entity != *p && !e.refs.contains(p) {
                return false;
            }
        }
        if let Some(ref s) = solution {
            if e.entity != *s && !e.refs.contains(s) {
                return false;
            }
        }

        // Type filter
        if let Some(ref t) = event_type {
            if !e.event_type.to_string().contains(t) {
                return false;
            }
        }

        // Search filter
        if let Some(ref q) = search {
            let q_lower = q.to_lowercase();
            let matches = e
                .rationale
                .as_ref()
                .map(|r| r.to_lowercase().contains(&q_lower))
                .unwrap_or(false);
            if !matches {
                return false;
            }
        }

        true
    });

    // Reverse to show most recent first, then limit
    events.reverse();
    events.truncate(limit);

    if json {
        println!("{}", serde_json::to_string_pretty(&events)?);
        return Ok(());
    }

    if events.is_empty() {
        println!("No events found");
        return Ok(());
    }

    for event in &events {
        let date = event.when.format("%Y-%m-%d %H:%M");
        let rationale = event
            .rationale
            .as_ref()
            .map(|r| format!(" - {}", crate::utils::truncate(r, 50)))
            .unwrap_or_default();

        println!(
            "{} {:20} {:8} {}{}",
            date,
            event.event_type.to_string(),
            event.entity,
            event.by,
            rationale
        );
    }

    Ok(())
}

fn parse_date_filter(s: &str) -> Option<chrono::DateTime<Utc>> {
    // Try YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|dt| Utc.from_utc_datetime(&dt));
    }
    // Try YYYY-MM (first of month)
    if let Ok(date) = NaiveDate::parse_from_str(&format!("{}-01", s), "%Y-%m-%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|dt| Utc.from_utc_datetime(&dt));
    }
    None
}

fn rebuild_events(ctx: &CommandContext) -> Result<()> {
    let store = &ctx.store;
    let events = store.rebuild_events_from_commits()?;
    println!(
        "Event log rebuilt from commit history: {} event{}.",
        events.len(),
        if events.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

fn validate_events(ctx: &CommandContext) -> Result<()> {
    let store = &ctx.store;
    let events = store.list_events()?;
    let mut errors: Vec<String> = Vec::new();

    // Collect all entity IDs
    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;
    let milestones = store.list_milestones()?;

    let mut all_entity_ids: HashSet<String> = HashSet::new();
    let mut problem_statuses: HashMap<String, ProblemStatus> = HashMap::new();
    let mut solution_statuses: HashMap<String, SolutionStatus> = HashMap::new();
    let mut critique_statuses: HashMap<String, CritiqueStatus> = HashMap::new();
    let mut milestone_statuses: HashMap<String, MilestoneStatus> = HashMap::new();

    for p in &problems {
        all_entity_ids.insert(p.id.clone());
        problem_statuses.insert(p.id.clone(), p.status.clone());
    }
    for s in &solutions {
        all_entity_ids.insert(s.id.clone());
        solution_statuses.insert(s.id.clone(), s.status.clone());
    }
    for c in &critiques {
        all_entity_ids.insert(c.id.clone());
        critique_statuses.insert(c.id.clone(), c.status.clone());
    }
    for m in &milestones {
        all_entity_ids.insert(m.id.clone());
        milestone_statuses.insert(m.id.clone(), m.status.clone());
    }

    // Check 1: Timestamp ordering (monotonic within event log)
    for window in events.windows(2) {
        if window[1].when < window[0].when {
            errors.push(format!(
                "Timestamp order violation: {} ({}) comes after {} ({})",
                window[1].entity,
                window[1].when.format("%Y-%m-%d %H:%M:%S"),
                window[0].entity,
                window[0].when.format("%Y-%m-%d %H:%M:%S"),
            ));
        }
    }

    // Check 2: Entity references exist
    for event in &events {
        if !all_entity_ids.contains(&event.entity) {
            errors.push(format!(
                "Event references non-existent entity '{}' (type: {})",
                event.entity, event.event_type,
            ));
        }
    }

    // Check 3: All entities have creation events
    let mut created_entities: HashSet<String> = HashSet::new();
    for event in &events {
        let is_creation = matches!(
            event.event_type,
            EventType::ProblemCreated
                | EventType::SolutionCreated
                | EventType::CritiqueRaised
                | EventType::MilestoneCreated
        );
        if is_creation {
            created_entities.insert(event.entity.clone());
        }
    }

    for id in &all_entity_ids {
        if !created_entities.contains(id) {
            errors.push(format!("Entity '{}' has no creation event", id));
        }
    }

    // Check 4: Terminal states match events
    for (id, status) in &problem_statuses {
        match status {
            ProblemStatus::Solved => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::ProblemSolved)
                {
                    errors.push(format!(
                        "Problem '{}' is Solved but has no problem_solved event",
                        id
                    ));
                }
            }
            ProblemStatus::Dissolved => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::ProblemDissolved)
                {
                    errors.push(format!(
                        "Problem '{}' is Dissolved but has no problem_dissolved event",
                        id
                    ));
                }
            }
            _ => {}
        }
    }

    for (id, status) in &solution_statuses {
        match status {
            SolutionStatus::Approved => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::SolutionApproved)
                {
                    errors.push(format!(
                        "Solution '{}' is Approved but has no solution_approved event",
                        id
                    ));
                }
            }
            SolutionStatus::Withdrawn => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::SolutionWithdrawn)
                {
                    errors.push(format!(
                        "Solution '{}' is Withdrawn but has no solution_withdrawn event",
                        id
                    ));
                }
            }
            _ => {}
        }
    }

    for (id, status) in &critique_statuses {
        match status {
            CritiqueStatus::Addressed => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::CritiqueAddressed)
                {
                    errors.push(format!(
                        "Critique '{}' is Addressed but has no critique_addressed event",
                        id
                    ));
                }
            }
            CritiqueStatus::Valid => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::CritiqueValidated)
                {
                    errors.push(format!(
                        "Critique '{}' is Valid but has no critique_validated event",
                        id
                    ));
                }
            }
            CritiqueStatus::Dismissed => {
                if !events
                    .iter()
                    .any(|e| e.entity == *id && e.event_type == EventType::CritiqueDismissed)
                {
                    errors.push(format!(
                        "Critique '{}' is Dismissed but has no critique_dismissed event",
                        id
                    ));
                }
            }
            _ => {}
        }
    }

    for (id, status) in &milestone_statuses {
        if *status == MilestoneStatus::Completed
            && !events
                .iter()
                .any(|e| e.entity == *id && e.event_type == EventType::MilestoneCompleted)
        {
            errors.push(format!(
                "Milestone '{}' is Completed but has no milestone_completed event",
                id
            ));
        }
    }

    // Report
    if errors.is_empty() {
        println!(
            "Event log is valid. ({} events, {} entities)",
            events.len(),
            all_entity_ids.len()
        );
    } else {
        println!("Event log validation found {} issue(s):\n", errors.len());
        for error in &errors {
            println!("  - {}", error);
        }
        println!("\nRun 'jjj events rebuild' to fix missing events.");
    }

    Ok(())
}
