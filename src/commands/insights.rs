use crate::context::CommandContext;
use crate::error::Result;
use crate::models::EventType;
use std::collections::HashMap;

fn format_duration(seconds: f64) -> String {
    if seconds < 3600.0 {
        "< 1h".to_string()
    } else if seconds < 86400.0 {
        format!("{:.0}h", seconds / 3600.0)
    } else if seconds < 604800.0 {
        let days = seconds / 86400.0;
        let hours = (seconds % 86400.0) / 3600.0;
        if hours >= 1.0 {
            format!("{:.0}d {:.0}h", days, hours)
        } else {
            format!("{:.0}d", days)
        }
    } else {
        let weeks = seconds / 604800.0;
        let days = (seconds % 604800.0) / 86400.0;
        if days >= 1.0 {
            format!("{:.0}w {:.0}d", weeks, days)
        } else {
            format!("{:.0}w", weeks)
        }
    }
}

pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;
    let events = store.list_events()?;

    if events.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "total_events": 0,
                    "message": "No events recorded yet"
                }))?
            );
        } else {
            println!("No events recorded yet.");
        }
        return Ok(());
    }

    // 1. Solution approval rate
    let approved = events
        .iter()
        .filter(|e| e.event_type == EventType::SolutionApproved)
        .count();
    let withdrawn = events
        .iter()
        .filter(|e| e.event_type == EventType::SolutionWithdrawn)
        .count();
    let total_outcomes = approved + withdrawn;
    let approval_rate = if total_outcomes > 0 {
        (approved as f64 / total_outcomes as f64) * 100.0
    } else {
        0.0
    };

    // 2. Avg time to solve: ProblemCreated → ProblemSolved per entity
    let mut problem_created: HashMap<&str, i64> = HashMap::new();
    let mut solve_durations: Vec<f64> = Vec::new();

    for event in &events {
        match event.event_type {
            EventType::ProblemCreated => {
                problem_created.insert(&event.entity, event.when.timestamp());
            }
            EventType::ProblemSolved => {
                if let Some(&created_ts) = problem_created.get(event.entity.as_str()) {
                    let duration = (event.when.timestamp() - created_ts) as f64;
                    if duration >= 0.0 {
                        solve_durations.push(duration);
                    }
                }
            }
            _ => {}
        }
    }

    let avg_solve_time = if solve_durations.is_empty() {
        None
    } else {
        Some(solve_durations.iter().sum::<f64>() / solve_durations.len() as f64)
    };

    // 3. Avg critique resolution time
    let mut critique_raised: HashMap<&str, i64> = HashMap::new();
    let mut critique_durations: Vec<f64> = Vec::new();

    for event in &events {
        match event.event_type {
            EventType::CritiqueRaised => {
                critique_raised.insert(&event.entity, event.when.timestamp());
            }
            EventType::CritiqueAddressed
            | EventType::CritiqueDismissed
            | EventType::CritiqueValidated => {
                if let Some(&raised_ts) = critique_raised.get(event.entity.as_str()) {
                    let duration = (event.when.timestamp() - raised_ts) as f64;
                    if duration >= 0.0 {
                        critique_durations.push(duration);
                    }
                }
            }
            _ => {}
        }
    }

    let avg_critique_time = if critique_durations.is_empty() {
        None
    } else {
        Some(critique_durations.iter().sum::<f64>() / critique_durations.len() as f64)
    };

    // 4. Critiques per solution
    let total_critiques = events
        .iter()
        .filter(|e| e.event_type == EventType::CritiqueRaised)
        .count();
    let total_solutions = events
        .iter()
        .filter(|e| e.event_type == EventType::SolutionCreated)
        .count();
    let critiques_per_solution = if total_solutions > 0 {
        total_critiques as f64 / total_solutions as f64
    } else {
        0.0
    };

    // 5. Top contributors (by event count)
    let mut contributor_counts: HashMap<&str, usize> = HashMap::new();
    for event in &events {
        if !event.by.is_empty() {
            *contributor_counts.entry(&event.by).or_insert(0) += 1;
        }
    }
    let mut contributors: Vec<(&&str, &usize)> = contributor_counts.iter().collect();
    contributors.sort_by(|a, b| b.1.cmp(a.1));
    contributors.truncate(5);

    // 6. Most active problems (by event count referencing them)
    let mut problem_event_counts: HashMap<&str, usize> = HashMap::new();
    for event in &events {
        let entity_prefix = event.entity.as_str();
        // Count events where the entity is a problem (created/solved/dissolved/reopened)
        match event.event_type {
            EventType::ProblemCreated
            | EventType::ProblemSolved
            | EventType::ProblemDissolved
            | EventType::ProblemReopened => {
                *problem_event_counts.entry(entity_prefix).or_insert(0) += 1;
            }
            _ => {
                // Also count refs that point to problems
                for r in &event.refs {
                    *problem_event_counts.entry(r.as_str()).or_insert(0) += 1;
                }
            }
        }
    }
    let mut active_problems: Vec<(&str, usize)> = problem_event_counts.into_iter().collect();
    active_problems.sort_by(|a, b| b.1.cmp(&a.1));
    active_problems.truncate(5);

    // Look up problem titles
    let problems = store.list_problems()?;
    let problem_titles: HashMap<&str, &str> = problems
        .iter()
        .map(|p| (p.id.as_str(), p.title.as_str()))
        .collect();

    if json {
        let output = serde_json::json!({
            "total_events": events.len(),
            "approval_rate": if total_outcomes > 0 { Some(format!("{:.0}%", approval_rate)) } else { None },
            "solutions_approved": approved,
            "solutions_withdrawn": withdrawn,
            "avg_time_to_solve": avg_solve_time.map(format_duration),
            "problems_solved": solve_durations.len(),
            "avg_critique_resolution_time": avg_critique_time.map(format_duration),
            "critiques_resolved": critique_durations.len(),
            "critiques_per_solution": format!("{:.1}", critiques_per_solution),
            "total_critiques": total_critiques,
            "total_solutions": total_solutions,
            "top_contributors": contributors.iter().map(|(name, count)| {
                serde_json::json!({ "name": name, "events": count })
            }).collect::<Vec<_>>(),
            "most_active_problems": active_problems.iter().map(|(id, count)| {
                let title = problem_titles.get(id).unwrap_or(&"(unknown)");
                let short_id = &id[..6.min(id.len())];
                serde_json::json!({ "id": short_id, "title": title, "events": count })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Text output
    println!("Project Insights");
    println!("{}", "─".repeat(40));

    println!("\nTotal events: {}", events.len());

    if total_outcomes > 0 {
        println!(
            "Approval rate: {:.0}% ({} approved, {} withdrawn)",
            approval_rate, approved, withdrawn
        );
    } else {
        println!("Approval rate: -- (no solutions finalized yet)");
    }

    match avg_solve_time {
        Some(t) => println!(
            "Avg time to solve: {} ({} problems)",
            format_duration(t),
            solve_durations.len()
        ),
        None => println!("Avg time to solve: -- (no problems solved yet)"),
    }

    match avg_critique_time {
        Some(t) => println!(
            "Avg critique resolution: {} ({} critiques)",
            format_duration(t),
            critique_durations.len()
        ),
        None => println!("Avg critique resolution: -- (no critiques resolved yet)"),
    }

    if total_solutions > 0 {
        println!(
            "Critiques per solution: {:.1} ({} critiques / {} solutions)",
            critiques_per_solution, total_critiques, total_solutions
        );
    } else {
        println!("Critiques per solution: -- (no solutions yet)");
    }

    if !contributors.is_empty() {
        println!("\nTop contributors:");
        for (name, count) in &contributors {
            println!("  {} ({} events)", name, count);
        }
    }

    if !active_problems.is_empty() {
        println!("\nMost active problems:");
        for (id, count) in &active_problems {
            let title = problem_titles.get(id).unwrap_or(&"(unknown)");
            let short_id = &id[..6.min(id.len())];
            println!("  p/{} \"{}\" ({} events)", short_id, title, count);
        }
    }

    Ok(())
}
