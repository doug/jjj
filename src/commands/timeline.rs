use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{Event, EventType};

pub fn execute(ctx: &CommandContext, problem_input: String, json: bool) -> Result<()> {
    let store = &ctx.store;

    // Resolve problem ID from title, prefix, or UUID
    let problem_id = ctx.resolve_problem(&problem_input)?;

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
    let mut events: Vec<&Event> = all_events
        .iter()
        .filter(|e| {
            if e.entity == problem_id {
                return true;
            }
            if solution_ids.contains(&e.entity) {
                return true;
            }
            if critique_ids.contains(&e.entity) {
                return true;
            }
            if e.refs.contains(&problem_id) {
                return true;
            }
            false
        })
        .collect();

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
            let supersedes = event
                .extra
                .supersedes
                .as_ref()
                .map(|s| format!(" (supersedes {})", s))
                .unwrap_or_default();
            format!("{} proposed{}", event.entity, supersedes)
        }
        EventType::SolutionApproved => format!("{} accepted", event.entity),
        EventType::SolutionWithdrawn => format!("{} refuted", event.entity),
        EventType::CritiqueRaised => {
            let title = event
                .extra
                .title
                .as_ref()
                .map(|t| format!(": \"{}\"", truncate(t, 25)))
                .unwrap_or_default();
            format!("{} raised{}", event.entity, title)
        }
        EventType::CritiqueAddressed => format!("{} addressed", event.entity),
        EventType::CritiqueDismissed => format!("{} dismissed", event.entity),
        EventType::CritiqueValidated => format!("{} validated", event.entity),
        EventType::CritiqueReplied => format!("{} replied to", event.entity),
        EventType::SolutionSubmitted => format!("{} moved to review", event.entity),
        EventType::MilestoneCreated => format!("{} created", event.entity),
        EventType::MilestoneCompleted => format!("{} completed", event.entity),
        EventType::GithubIssueCreated => {
            let num = event
                .extra
                .github_number
                .map(|n| format!(" #{}", n))
                .unwrap_or_default();
            format!("GitHub issue{} created", num)
        }
        EventType::GithubIssueImported => {
            let num = event
                .extra
                .github_number
                .map(|n| format!(" #{}", n))
                .unwrap_or_default();
            format!("imported from GitHub issue{}", num)
        }
        EventType::GithubIssueClosed => {
            let num = event
                .extra
                .github_number
                .map(|n| format!(" #{}", n))
                .unwrap_or_default();
            format!("GitHub issue{} closed", num)
        }
        EventType::GithubPrCreated => {
            let num = event
                .extra
                .github_number
                .map(|n| format!(" #{}", n))
                .unwrap_or_default();
            format!("GitHub PR{} created", num)
        }
        EventType::GithubPrMerged => {
            let num = event
                .extra
                .github_number
                .map(|n| format!(" #{}", n))
                .unwrap_or_default();
            format!("GitHub PR{} merged", num)
        }
        EventType::GithubReviewImported => {
            let num = event
                .extra
                .github_number
                .map(|n| format!(" PR #{}", n))
                .unwrap_or_default();
            format!("review imported from{}", num)
        }
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
