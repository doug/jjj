use crate::cli::MilestoneAction;
use crate::context::CommandContext;
use crate::display::truncated_prefixes;
use crate::error::Result;
use crate::models::{Milestone, MilestoneStatus, ProblemStatus, SolutionStatus};
use chrono::NaiveDate;

pub fn execute(ctx: &CommandContext, action: MilestoneAction) -> Result<()> {
    match action {
        MilestoneAction::New { title, date } => create_milestone(ctx, title, date),
        MilestoneAction::Edit {
            milestone_id,
            title,
            date,
            status,
        } => edit_milestone(ctx, milestone_id, title, date, status),
        MilestoneAction::List { json } => list_milestones(ctx, json),
        MilestoneAction::Show { milestone_id, json } => show_milestone(ctx, milestone_id, json),
        MilestoneAction::AddProblem {
            milestone_id,
            problem_id,
        } => add_problem(ctx, milestone_id, problem_id),
        MilestoneAction::RemoveProblem {
            milestone_id,
            problem_id,
        } => remove_problem(ctx, milestone_id, problem_id),
        MilestoneAction::Roadmap { json } => show_roadmap(ctx, json),
        MilestoneAction::Assign { milestone_id, to } => assign_milestone(ctx, milestone_id, to),
        MilestoneAction::Status { milestone_id, json } => milestone_status(ctx, milestone_id, json),
    }
}

fn create_milestone(ctx: &CommandContext, title: String, date: Option<String>) -> Result<()> {
    let store = &ctx.store;

    store.with_metadata(&format!("Create milestone: {}", title), || {
        let milestone_id = store.next_milestone_id()?;

        // Parse date if provided
        let target_date = if let Some(date_str) = date {
            let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
                crate::error::JjjError::Validation(format!(
                    "Invalid date format: {}. Use YYYY-MM-DD",
                    date_str
                ))
            })?;
            match naive_date.and_hms_opt(0, 0, 0) {
                Some(dt) => Some(dt.and_utc()),
                None => {
                    return Err(crate::error::JjjError::Validation(format!(
                        "Invalid date: {}",
                        date_str
                    )));
                }
            }
        } else {
            None
        };

        let mut milestone = Milestone::new(milestone_id.clone(), title.clone());
        milestone.set_target_date(target_date);

        store.save_milestone(&milestone)?;
        println!("Created milestone {} ({})", milestone_id, title);
        if let Some(d) = target_date {
            println!("  Target date: {}", d.format("%Y-%m-%d"));
        }
        Ok(())
    })
}

fn edit_milestone(
    ctx: &CommandContext,
    milestone_input: String,
    title: Option<String>,
    date: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let milestone_id = ctx.resolve_milestone(&milestone_input)?;
    let store = &ctx.store;

    store.with_metadata(&format!("Edit milestone {}", milestone_id), || {
        let mut milestone = store.load_milestone(&milestone_id)?;

        if let Some(t) = title {
            milestone.title = t;
        }

        if let Some(d) = date {
            let naive_date = NaiveDate::parse_from_str(&d, "%Y-%m-%d").map_err(|_| {
                crate::error::JjjError::Validation(format!(
                    "Invalid date format: {}. Use YYYY-MM-DD",
                    d
                ))
            })?;
            let dt = naive_date.and_hms_opt(0, 0, 0).ok_or_else(|| {
                crate::error::JjjError::Validation(format!("Invalid date: {}", d))
            })?;
            milestone.set_target_date(Some(dt.and_utc()));
        }

        if let Some(s) = status {
            let new_status: MilestoneStatus = s
                .parse()
                .map_err(|e: String| crate::error::JjjError::Validation(e))?;
            milestone.set_status(new_status);
        }

        store.save_milestone(&milestone)?;
        println!("Updated milestone {}", milestone_id);
        Ok(())
    })
}

fn list_milestones(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;

    let milestones = store.list_milestones()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&milestones)?);
        return Ok(());
    }

    if milestones.is_empty() {
        println!("No milestones found.");
        return Ok(());
    }

    // Calculate truncated prefixes
    let uuids: Vec<&str> = milestones.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    println!(
        "{:<10} {:<12} {:<12} {:<6} TITLE",
        "ID", "STATUS", "TARGET", "PROBS"
    );
    println!("{}", "-".repeat(70));

    for (milestone, (_, prefix)) in milestones.iter().zip(prefixes.iter()) {
        let date_str = milestone
            .target_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "-".to_string());

        let status_icon = match milestone.status {
            MilestoneStatus::Planning => " ",
            MilestoneStatus::Active => ">",
            MilestoneStatus::Completed => "+",
            MilestoneStatus::Cancelled => "x",
        };

        println!(
            "{:<10} {}{:<11} {:<12} {:<6} {}",
            prefix,
            status_icon,
            milestone.status,
            date_str,
            milestone.problem_ids.len(),
            milestone.title
        );
    }

    Ok(())
}

fn show_milestone(ctx: &CommandContext, milestone_input: String, json: bool) -> Result<()> {
    let milestone_id = ctx.resolve_milestone(&milestone_input)?;
    let store = &ctx.store;

    let milestone = store.load_milestone(&milestone_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&milestone)?);
        return Ok(());
    }

    println!("Milestone: {} - {}", milestone.id, milestone.title);
    println!("Status: {}", milestone.status);

    if let Some(date) = milestone.target_date {
        println!("Target Date: {}", date.format("%Y-%m-%d"));
        if milestone.is_active() {
            if let Some(days) = milestone.days_until_target() {
                if days < 0 {
                    println!("  {} days overdue", -days);
                } else {
                    println!("  {} days remaining", days);
                }
            }
        }
    }

    if let Some(ref assignee) = milestone.assignee {
        println!("Assignee: {}", assignee);
    }

    // Show goals
    if !milestone.goals.is_empty() {
        println!("\n## Goals\n{}", milestone.goals);
    }

    // Show success criteria
    if !milestone.success_criteria.is_empty() {
        println!("\n## Success Criteria\n{}", milestone.success_criteria);
    }

    // Show problems and their solutions
    if !milestone.problem_ids.is_empty() {
        println!("\n## Problems ({})", milestone.problem_ids.len());

        let mut solved = 0;
        let mut in_progress = 0;
        let mut open = 0;

        for problem_id in &milestone.problem_ids {
            if let Ok(problem) = store.load_problem(problem_id) {
                let solutions = store
                    .list_solutions_for_problem(problem_id)
                    .unwrap_or_default();
                let approved_solutions = solutions
                    .iter()
                    .filter(|s| s.status == SolutionStatus::Approved)
                    .count();
                let submitted_solutions = solutions
                    .iter()
                    .filter(|s| s.status == SolutionStatus::Submitted)
                    .count();

                let status_icon = match problem.status {
                    ProblemStatus::Open => "[ ]",
                    ProblemStatus::InProgress => "[>]",
                    ProblemStatus::Solved => "[+]",
                    ProblemStatus::Dissolved => "[~]",
                };

                match problem.status {
                    ProblemStatus::Open => open += 1,
                    ProblemStatus::InProgress => in_progress += 1,
                    ProblemStatus::Solved | ProblemStatus::Dissolved => solved += 1,
                }

                println!(
                    "  {} {} - {} ({} solutions, {} approved, {} submitted)",
                    status_icon,
                    problem.id,
                    problem.title,
                    solutions.len(),
                    approved_solutions,
                    submitted_solutions
                );
            }
        }

        println!(
            "\n  Progress: {} solved, {} in progress, {} open",
            solved, in_progress, open
        );
    }

    println!(
        "\nCreated: {}",
        milestone.created_at.format("%Y-%m-%d %H:%M")
    );
    println!("Updated: {}", milestone.updated_at.format("%Y-%m-%d %H:%M"));

    crate::commands::show_related_items(ctx, "milestone", &milestone.id)?;

    Ok(())
}

fn add_problem(ctx: &CommandContext, milestone_input: String, problem_input: String) -> Result<()> {
    let milestone_id = ctx.resolve_milestone(&milestone_input)?;
    let problem_id = ctx.resolve_problem(&problem_input)?;
    let store = &ctx.store;

    // Check if problem is already in milestone
    let milestone = store.load_milestone(&milestone_id)?;
    if milestone.problem_ids.contains(&problem_id) {
        eprintln!(
            "Warning: Problem {} is already in milestone {}",
            problem_id, milestone_id
        );
        return Ok(());
    }

    store.with_metadata(
        &format!("Add problem {} to milestone {}", problem_id, milestone_id),
        || {
            let mut milestone = store.load_milestone(&milestone_id)?;
            let mut problem = store.load_problem(&problem_id)?;

            milestone.add_problem(problem_id.clone());
            store.save_milestone(&milestone)?;

            problem.set_milestone(Some(milestone_id.clone()));
            store.save_problem(&problem)?;

            println!("Added problem {} to milestone {}", problem_id, milestone_id);
            Ok(())
        },
    )
}

fn remove_problem(
    ctx: &CommandContext,
    milestone_input: String,
    problem_input: String,
) -> Result<()> {
    let milestone_id = ctx.resolve_milestone(&milestone_input)?;
    let problem_id = ctx.resolve_problem(&problem_input)?;
    let store = &ctx.store;

    store.with_metadata(
        &format!(
            "Remove problem {} from milestone {}",
            problem_id, milestone_id
        ),
        || {
            let mut milestone = store.load_milestone(&milestone_id)?;
            let mut problem = store.load_problem(&problem_id)?;

            milestone.remove_problem(&problem_id);
            store.save_milestone(&milestone)?;

            if problem.milestone_id.as_deref() == Some(&milestone_id) {
                problem.set_milestone(None);
                store.save_problem(&problem)?;
            }

            println!(
                "Removed problem {} from milestone {}",
                problem_id, milestone_id
            );
            Ok(())
        },
    )
}

fn show_roadmap(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;

    let mut milestones = store.list_milestones()?;

    // Sort by target date (None dates go last)
    milestones.sort_by(|a, b| match (&a.target_date, &b.target_date) {
        (Some(date_a), Some(date_b)) => date_a.cmp(date_b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&milestones)?);
        return Ok(());
    }

    if milestones.is_empty() {
        println!("No milestones found.");
        return Ok(());
    }

    println!("Roadmap:\n");

    for milestone in milestones {
        let date_str = milestone
            .target_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "No date".to_string());

        let status_icon = match milestone.status {
            MilestoneStatus::Planning => " ",
            MilestoneStatus::Active => ">",
            MilestoneStatus::Completed => "+",
            MilestoneStatus::Cancelled => "x",
        };

        // Calculate progress
        let mut solved = 0;
        let total = milestone.problem_ids.len();

        for problem_id in &milestone.problem_ids {
            if let Ok(problem) = store.load_problem(problem_id) {
                if problem.is_resolved() {
                    solved += 1;
                }
            }
        }

        let progress = if total > 0 {
            format!("{}/{} problems solved", solved, total)
        } else {
            "No problems".to_string()
        };

        println!(
            "{} {} - {} [{}]",
            status_icon, milestone.id, milestone.title, date_str
        );
        println!("    {}", progress);

        // Show overdue warning
        if milestone.is_overdue() {
            if let Some(days) = milestone.days_until_target() {
                println!("    {} days overdue", -days);
            }
        }

        println!();
    }

    Ok(())
}

fn assign_milestone(
    ctx: &CommandContext,
    milestone_input: String,
    assignee: Option<String>,
) -> Result<()> {
    let milestone_id = ctx.resolve_milestone(&milestone_input)?;
    let store = &ctx.store;

    let assignee_name = match assignee {
        Some(name) => name,
        None => store.jj_client.user_identity()?,
    };

    store.with_metadata(
        &format!("Assign milestone {} to {}", milestone_id, assignee_name),
        || {
            let mut milestone = store.load_milestone(&milestone_id)?;
            milestone.assignee = Some(assignee_name.clone());
            store.save_milestone(&milestone)?;
            println!("Assigned milestone {} to {}", milestone_id, assignee_name);
            Ok(())
        },
    )
}

fn milestone_status(ctx: &CommandContext, milestone_input: String, json: bool) -> Result<()> {
    let milestone_id = ctx.resolve_milestone(&milestone_input)?;
    let store = &ctx.store;
    let milestone = store.load_milestone(&milestone_id)?;

    // Count problems by status
    let mut total = 0usize;
    let mut open_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut solved_count = 0usize;
    let mut dissolved_count = 0usize;

    for problem_id in &milestone.problem_ids {
        total += 1;
        if let Ok(problem) = store.load_problem(problem_id) {
            match problem.status {
                ProblemStatus::Open => open_count += 1,
                ProblemStatus::InProgress => in_progress_count += 1,
                ProblemStatus::Solved => solved_count += 1,
                ProblemStatus::Dissolved => dissolved_count += 1,
            }
        } else {
            // Problem file missing — count as open for safety
            open_count += 1;
        }
    }

    let pct_complete = if total > 0 {
        (solved_count + dissolved_count) * 100 / total
    } else {
        0
    };

    // Target date and days remaining
    let target_date_str = milestone
        .target_date
        .map(|d| d.format("%Y-%m-%d").to_string());
    let days_remaining = milestone.days_until_target();

    if json {
        let obj = serde_json::json!({
            "title": milestone.title,
            "status": milestone.status.to_string(),
            "total": total,
            "solved": solved_count,
            "dissolved": dissolved_count,
            "in_progress": in_progress_count,
            "open": open_count,
            "pct_complete": pct_complete,
            "target_date": target_date_str,
            "days_remaining": days_remaining,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
        return Ok(());
    }

    let summary = format!(
        "Problems: {} total — {} solved, {} in-progress, {} open ({}% complete)",
        total, solved_count, in_progress_count, open_count, pct_complete
    );
    println!("{}: {}", milestone.title, milestone.status);
    println!("{}", summary);

    if let Some(ref date_str) = target_date_str {
        match days_remaining {
            Some(d) if d < 0 => println!("Target: {} ({} days overdue)", date_str, -d),
            Some(d) => println!("Target: {} ({} days remaining)", date_str, d),
            None => println!("Target: {}", date_str),
        }
    }

    Ok(())
}
