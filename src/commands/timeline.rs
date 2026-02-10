use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Event, EventType};
use crate::storage::MetadataStore;

pub fn execute(problem_id: String, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Load problem for title
    let problem = store.load_problem(&problem_id)?;

    // Get all events related to this problem
    let all_events = store.list_events()?;

    // Find related entity IDs (solutions and critiques for this problem)
    let solutions = store.get_solutions_for_problem(&problem_id)?;
    let solution_ids: Vec<String> = solutions.iter().map(|s| s.id.clone()).collect();

    let mut critique_ids: Vec<String> = Vec::new();
    for s in &solutions {
        let critiques = store.get_critiques_for_solution(&s.id)?;
        for c in critiques {
            critique_ids.push(c.id.clone());
        }
    }

    // Filter events related to this problem
    let mut events: Vec<&Event> = all_events.iter().filter(|e| {
        if e.entity == problem_id { return true; }
        if solution_ids.contains(&e.entity) { return true; }
        if critique_ids.contains(&e.entity) { return true; }
        if e.refs.contains(&problem_id) { return true; }
        false
    }).collect();

    // Sort by timestamp
    events.sort_by_key(|e| e.when);

    if json {
        println!("{}", serde_json::to_string_pretty(&events)?);
        return Ok(());
    }

    // Print header
    println!("{}: {}", problem_id, problem.title);
    println!("{}", "━".repeat(50));
    println!();

    for event in &events {
        let date = event.when.format("%Y-%m-%d");
        let desc = format_event_description(event);
        let by = &event.by;

        println!("{:<12} {:<40} {}", date, desc, by);

        if let Some(ref rationale) = event.rationale {
            for line in rationale.lines() {
                println!("             \"{}\"", line);
            }
        }
    }

    Ok(())
}

fn format_event_description(event: &Event) -> String {
    match event.event_type {
        EventType::ProblemCreated => "problem created".to_string(),
        EventType::ProblemSolved => "problem solved".to_string(),
        EventType::ProblemDissolved => "problem dissolved".to_string(),
        EventType::ProblemReopened => "problem reopened".to_string(),
        EventType::SolutionCreated => {
            let supersedes = event.extra.supersedes.as_ref()
                .map(|s| format!(" (supersedes {})", s))
                .unwrap_or_default();
            format!("{} proposed{}", event.entity, supersedes)
        }
        EventType::SolutionAccepted => format!("{} accepted", event.entity),
        EventType::SolutionRefuted => format!("{} refuted", event.entity),
        EventType::CritiqueRaised => {
            let title = event.extra.title.as_ref()
                .map(|t| format!(": \"{}\"", truncate(t, 25)))
                .unwrap_or_default();
            format!("{} raised{}", event.entity, title)
        }
        EventType::CritiqueAddressed => format!("{} addressed", event.entity),
        EventType::CritiqueDismissed => format!("{} dismissed", event.entity),
        EventType::CritiqueValidated => format!("{} validated", event.entity),
        EventType::MilestoneCreated => format!("{} created", event.entity),
        EventType::MilestoneCompleted => format!("{} completed", event.entity),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
