use crate::cli::CritiqueAction;
use crate::context::CommandContext;
use crate::db::{search, Database};
use crate::display::truncated_prefixes;
use crate::error::Result;
use crate::models::{
    Critique, CritiqueSeverity, CritiqueStatus, Event, EventExtra, EventType, SolutionStatus,
};

pub fn execute(ctx: &CommandContext, action: CritiqueAction) -> Result<()> {
    match action {
        CritiqueAction::New {
            solution_id,
            title,
            severity,
            file,
            line,
            reviewer,
        } => new_critique(ctx, solution_id, title, severity, file, line, reviewer),
        CritiqueAction::List {
            solution,
            status,
            reviewer,
            search,
            json,
        } => list_critiques(ctx, solution, status, reviewer, search.as_deref(), json),
        CritiqueAction::Show { critique_id, json } => show_critique(ctx, critique_id, json),
        CritiqueAction::Edit {
            critique_id,
            title,
            severity,
            status,
        } => edit_critique(ctx, critique_id, title, severity, status),
        CritiqueAction::Address { critique_id } => address_critique(ctx, critique_id),
        CritiqueAction::Validate { critique_id } => validate_critique(ctx, critique_id),
        CritiqueAction::Dismiss { critique_id } => dismiss_critique(ctx, critique_id),
        CritiqueAction::Reply { critique_id, body } => reply_to_critique(ctx, critique_id, body),
    }
}

fn new_critique(
    ctx: &CommandContext,
    solution_input: String,
    title: String,
    severity_str: String,
    file: Option<String>,
    line: Option<usize>,
    reviewer: Option<String>,
) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;

    // Parse severity
    let severity: CritiqueSeverity = severity_str.parse().map_err(|e: String| crate::error::JjjError::Validation(e))?;

    // Validate solution exists
    let solution = store.load_solution(&solution_id)?;

    // Warn if solution is already finalized
    if solution.is_finalized() {
        println!(
            "Warning: Solution {} is already {:?}. Creating critique anyway.",
            solution_id, solution.status
        );
    }

    // Get user for event
    let user = store.get_current_user()?;

    store.with_metadata(
        &format!("Create critique on {}: {}", solution_id, title),
        || {
            let critique_id = store.next_critique_id()?;
            let mut critique =
                Critique::new(critique_id.clone(), title.clone(), solution_id.clone());
            critique.set_severity(severity.clone());

            // Set author to current user
            let author = store.jj_client.user_identity()?;
            critique.author = Some(author);

            // Set reviewer if provided
            if let Some(ref r) = reviewer {
                critique.reviewer = Some(r.trim_start_matches('@').to_string());
            }

            // Set location if provided
            if let (Some(file_path), Some(line_num)) = (file.clone(), line) {
                // Try to read context from file
                let context = store
                    .jj_client
                    .file_at_revision("@", &file_path)
                    .ok()
                    .map(|content| {
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let start = line_num.saturating_sub(2);
                        let end = (line_num + 2).min(lines.len());
                        lines[start..end].to_vec()
                    })
                    .unwrap_or_default();

                critique.set_location(file_path, line_num, None, context);
            }

            // Create event for decision log
            let extra = EventExtra {
                target: Some(solution_id.clone()),
                severity: Some(severity.to_string()),
                title: Some(title.clone()),
                ..Default::default()
            };
            let event = Event::new(EventType::CritiqueRaised, critique_id.clone(), user.clone())
                .with_extra(extra);
            store.set_pending_event(event);

            store.save_critique(&critique)?;

            // Update solution's critique_ids
            let mut solution = store.load_solution(&solution_id)?;
            solution.add_critique(critique_id.clone());
            store.save_solution(&solution)?;

            println!(
                "Created critique {} ({}) on solution {}",
                critique.id, critique.title, solution_id
            );
            println!("  Severity: {}", severity);

            if let Some(ref fp) = file {
                println!("  Location: {}:{}", fp, line.unwrap_or(0));
            }

            Ok(())
        },
    )
}

fn list_critiques(
    ctx: &CommandContext,
    solution_filter: Option<String>,
    status_filter: Option<String>,
    reviewer_filter: Option<String>,
    search_query: Option<&str>,
    json: bool,
) -> Result<()> {
    let store = &ctx.store;

    let mut critiques = store.list_critiques()?;

    // Filter by solution (resolve input if provided)
    if let Some(ref solution_input) = solution_filter {
        let solution_id = ctx.resolve_solution(solution_input)?;
        critiques.retain(|c| c.solution_id == solution_id);
    }

    // Filter by status
    if let Some(status_str) = status_filter {
        let status: CritiqueStatus = status_str.parse().map_err(|e: String| crate::error::JjjError::Validation(e))?;
        critiques.retain(|c| c.status == status);
    }

    // Filter by reviewer
    if let Some(ref reviewer) = reviewer_filter {
        let reviewer = reviewer.trim_start_matches('@');
        critiques.retain(|c| c.reviewer.as_deref() == Some(reviewer));
    }

    // Filter by search query using FTS
    if let Some(query) = search_query {
        let jj_client = ctx.jj();
        let db_path = jj_client.repo_root().join(".jj").join("jjj.db");

        if db_path.exists() {
            let db = Database::open(&db_path)?;
            let results = search::search(db.conn(), query, Some("critique"))?;
            let matching_ids: std::collections::HashSet<_> =
                results.iter().map(|r| r.entity_id.as_str()).collect();
            critiques.retain(|c| matching_ids.contains(c.id.as_str()));
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&critiques)?);
        return Ok(());
    }

    if critiques.is_empty() {
        println!("No critiques found.");
        return Ok(());
    }

    // Calculate truncated prefixes for critiques
    let critique_uuids: Vec<&str> = critiques.iter().map(|c| c.id.as_str()).collect();
    let critique_prefixes = truncated_prefixes(&critique_uuids);

    // Calculate truncated prefixes for solutions (for display)
    let solution_uuids: Vec<&str> = critiques.iter().map(|c| c.solution_id.as_str()).collect();
    let solution_prefixes = truncated_prefixes(&solution_uuids);

    println!(
        "{:<10} {:<12} {:<10} {:<10} TITLE",
        "ID", "STATUS", "SEVERITY", "SOLUTION"
    );
    println!("{}", "-".repeat(80));

    for ((critique, (_, crit_prefix)), (_, sol_prefix)) in critiques
        .iter()
        .zip(critique_prefixes.iter())
        .zip(solution_prefixes.iter())
    {
        let status_icon = match critique.status {
            CritiqueStatus::Open => "?",
            CritiqueStatus::Addressed => "+",
            CritiqueStatus::Valid => "!",
            CritiqueStatus::Dismissed => "-",
        };

        println!(
            "{:<10} {}{:<11} {:<10} {:<10} {}",
            crit_prefix,
            status_icon,
            critique.status,
            critique.severity,
            sol_prefix,
            critique.title
        );
    }

    Ok(())
}

fn show_critique(ctx: &CommandContext, critique_input: String, json: bool) -> Result<()> {
    let critique_id = ctx.resolve_critique(&critique_input)?;
    let store = &ctx.store;

    let critique = store.load_critique(&critique_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&critique)?);
        return Ok(());
    }

    println!("Critique: {} - {}", critique.id, critique.title);
    println!("Status: {}", critique.status);
    println!("Severity: {}", critique.severity);
    println!("Target Solution: {}", critique.solution_id);

    if let Some(ref author) = critique.author {
        println!("Author: {}", author);
    }

    // Show argument
    if !critique.argument.is_empty() {
        println!("\n## Argument\n{}", critique.argument);
    }

    // Show evidence
    if !critique.evidence.is_empty() {
        println!("\n## Evidence\n{}", critique.evidence);
    }

    // Show replies
    if !critique.replies.is_empty() {
        println!("\n## Discussion ({} replies)", critique.replies.len());
        for reply in &critique.replies {
            println!(
                "\n### {} @ {}",
                reply.author,
                reply.created_at.format("%Y-%m-%d %H:%M")
            );
            println!("{}", reply.body);
        }
    }

    println!(
        "\nCreated: {}",
        critique.created_at.format("%Y-%m-%d %H:%M")
    );
    println!("Updated: {}", critique.updated_at.format("%Y-%m-%d %H:%M"));

    crate::commands::show_related_items(ctx, "critique", &critique.id)?;

    Ok(())
}

fn edit_critique(
    ctx: &CommandContext,
    critique_input: String,
    title: Option<String>,
    severity: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let critique_id = ctx.resolve_critique(&critique_input)?;
    let store = &ctx.store;

    store.with_metadata(&format!("Edit critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;

        if let Some(new_title) = title {
            critique.title = new_title;
        }

        if let Some(severity_str) = severity {
            let new_severity: CritiqueSeverity = severity_str.parse().map_err(|e: String| crate::error::JjjError::Validation(e))?;
            critique.set_severity(new_severity);
        }

        if let Some(status_str) = status {
            let new_status: CritiqueStatus = status_str.parse().map_err(|e: String| crate::error::JjjError::Validation(e))?;
            critique.set_status(new_status);
        }

        store.save_critique(&critique)?;
        println!("Updated critique {}", critique_id);
        Ok(())
    })
}

fn address_critique(ctx: &CommandContext, critique_input: String) -> Result<()> {
    let critique_id = ctx.resolve_critique(&critique_input)?;
    let store = &ctx.store;

    // Create event for decision log
    let user = store.get_current_user()?;
    let event = Event::new(EventType::CritiqueAddressed, critique_id.clone(), user);

    store.with_metadata(&format!("Address critique {}", critique_id), || {
        store.set_pending_event(event.clone());
        let mut critique = store.load_critique(&critique_id)?;
        critique.address();
        store.save_critique(&critique)?;
        println!(
            "Critique {} marked as addressed (solution was modified to address it)",
            critique_id
        );
        Ok(())
    })
}

fn validate_critique(ctx: &CommandContext, critique_input: String) -> Result<()> {
    let critique_id = ctx.resolve_critique(&critique_input)?;
    let store = &ctx.store;

    // Create event for decision log
    let user = store.get_current_user()?;
    let event = Event::new(EventType::CritiqueValidated, critique_id.clone(), user);

    store.with_metadata(&format!("Validate critique {}", critique_id), || {
        store.set_pending_event(event.clone());
        let mut critique = store.load_critique(&critique_id)?;
        let solution_id = critique.solution_id.clone();

        critique.validate();
        store.save_critique(&critique)?;

        println!(
            "Critique {} validated (it's correct - the solution has a flaw)",
            critique_id
        );

        // Optionally refute the solution
        let solution = store.load_solution(&solution_id)?;
        if solution.status != SolutionStatus::Refuted {
            println!(
                "\nThe target solution {} should likely be refuted.",
                solution_id
            );
            println!(
                "Use 'jjj solution refute {}' to mark it as refuted.",
                solution_id
            );
        }

        Ok(())
    })
}

fn dismiss_critique(ctx: &CommandContext, critique_input: String) -> Result<()> {
    let critique_id = ctx.resolve_critique(&critique_input)?;
    let store = &ctx.store;

    // Create event for decision log
    let user = store.get_current_user()?;
    let event = Event::new(EventType::CritiqueDismissed, critique_id.clone(), user);

    store.with_metadata(&format!("Dismiss critique {}", critique_id), || {
        store.set_pending_event(event.clone());
        let mut critique = store.load_critique(&critique_id)?;
        critique.dismiss();
        store.save_critique(&critique)?;
        println!(
            "Critique {} dismissed (shown to be incorrect or irrelevant)",
            critique_id
        );
        Ok(())
    })
}

fn reply_to_critique(ctx: &CommandContext, critique_input: String, body: String) -> Result<()> {
    let critique_id = ctx.resolve_critique(&critique_input)?;
    let store = &ctx.store;

    store.with_metadata(&format!("Reply to critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        let author = store.jj_client.user_identity()?;

        critique.add_reply(author.clone(), body.clone());
        store.save_critique(&critique)?;

        if let Some(reply) = critique.replies.last() {
            println!("Added reply {} to critique {}", reply.id, critique_id);
        } else {
            println!("Added reply to critique {}", critique_id);
        }

        Ok(())
    })
}
