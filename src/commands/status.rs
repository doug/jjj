use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{CritiqueStatus, Critique, Priority, ProblemStatus, SolutionStatus};

fn priority_sort_value(priority: &Priority) -> i32 {
    match priority {
        Priority::Critical => 3,
        Priority::High => 2,
        Priority::Medium => 1,
        Priority::Low => 0,
    }
}

pub fn execute(ctx: &CommandContext, all: bool, mine: bool, limit: Option<usize>, json: bool) -> Result<()> {
    let store = &ctx.store;
    let jj_client = ctx.jj();
    let user = store.jj_client.user_identity().unwrap_or_default();

    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;

    if json {
        // Build JSON output with active_solution, next_actions, summary
        let current_change = jj_client.current_change_id().ok();
        let active_solution = current_change.as_ref().and_then(|change_id| {
            solutions.iter().find(|s| s.change_ids.contains(change_id))
        });

        let active_json = active_solution.map(|s| {
            serde_json::json!({
                "id": s.id,
                "title": s.title,
                "problem_id": s.problem_id,
                "status": format!("{}", s.status),
            })
        });

        let mut items = build_next_actions(&problems, &solutions, &critiques, &user, mine);
        let total_count = items.len();
        let effective_limit = if all { usize::MAX } else { limit.unwrap_or(5) };
        items.truncate(effective_limit);

        let open_problems = problems.iter().filter(|p| p.is_open()).count();
        let testing_solutions = solutions.iter().filter(|s| s.status == SolutionStatus::Testing).count();
        let open_critiques = critiques.iter().filter(|c| c.status == CritiqueStatus::Open).count();

        let output = serde_json::json!({
            "active_solution": active_json,
            "items": items,
            "total_count": total_count,
            "user": user,
            "summary": {
                "open_problems": open_problems,
                "testing_solutions": testing_solutions,
                "open_critiques": open_critiques,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // 1. Active Solution section
        let current_change = jj_client.current_change_id().ok();
        if let Some(change_id) = &current_change {
            if let Some(active) = solutions.iter().find(|s| s.change_ids.contains(change_id)) {
                println!("Active: {} \"{}\" -> {} [{}]", active.id, active.title, active.problem_id, active.status);

                // Show open critiques on active solution
                let active_critiques: Vec<_> = critiques.iter()
                    .filter(|c| c.solution_id == active.id && c.status == CritiqueStatus::Open)
                    .collect();
                if !active_critiques.is_empty() {
                    println!("  Open critiques: {}", active_critiques.len());
                    for c in &active_critiques {
                        println!("    {}: {} [{}]", c.id, c.title, c.severity);
                    }
                }
                println!();
            }
        }

        // 2. Next Actions section
        let mut items = build_next_actions(&problems, &solutions, &critiques, &user, mine);
        let total_count = items.len();
        let effective_limit = if all { usize::MAX } else { limit.unwrap_or(5) };
        items.truncate(effective_limit);

        if items.is_empty() {
            println!("No pending actions. All caught up!");
        } else {
            println!("Next actions:\n");
            for (i, item) in items.iter().enumerate() {
                let category = item["category"].as_str().unwrap_or("").to_uppercase();
                let entity_id = item["entity_id"].as_str().unwrap_or("");
                let title = item["title"].as_str().unwrap_or("");
                let summary = item["summary"].as_str().unwrap_or("");

                println!("{}. [{}] {}: {} -- {}", i + 1, category, entity_id, title, summary);

                if let Some(details) = item["details"].as_array() {
                    for detail in details {
                        let id = detail["id"].as_str().unwrap_or("");
                        let text = detail["text"].as_str().unwrap_or("");
                        let severity = detail["severity"].as_str().unwrap_or("");
                        println!("   {}: {} [{}]", id, text, severity);
                    }
                }

                if let Some(cmd) = item["suggested_command"].as_str() {
                    if !cmd.is_empty() {
                        println!("   -> {}", cmd);
                    }
                }
                println!();
            }

            if !all && total_count > effective_limit {
                println!("Showing {} of {} items. Use --all to see everything.", effective_limit, total_count);
            }
        }

        // 3. Summary section
        let open_problems = problems.iter().filter(|p| p.status == ProblemStatus::Open || p.status == ProblemStatus::InProgress).count();
        let testing_solutions = solutions.iter().filter(|s| s.status == SolutionStatus::Testing).count();
        let open_critiques = critiques.iter().filter(|c| c.status == CritiqueStatus::Open).count();

        println!("\nSummary: {} open problems, {} testing solutions, {} open critiques",
            open_problems, testing_solutions, open_critiques);
    }

    Ok(())
}

fn build_next_actions(
    problems: &[crate::models::Problem],
    solutions: &[crate::models::Solution],
    critiques: &[Critique],
    user: &str,
    mine: bool,
) -> Vec<serde_json::Value> {
    let mut items: Vec<serde_json::Value> = Vec::new();

    // 1. BLOCKED: Solutions with open critiques
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let open_critiques: Vec<&Critique> = critiques
            .iter()
            .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
            .collect();

        if !open_critiques.is_empty() {
            let top_critique = open_critiques.iter()
                .max_by_key(|c| c.severity.clone())
                .unwrap();

            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| &p.priority).cloned().unwrap_or_default();

            items.push(serde_json::json!({
                "category": "blocked",
                "entity_type": "solution",
                "entity_id": solution.id,
                "title": solution.title,
                "summary": format!("{} open critique(s)", open_critiques.len()),
                "suggested_command": format!("jjj critique show {}", top_critique.id),
                "priority": format!("{}", priority),
                "priority_sort": priority_sort_value(&priority),
                "details": open_critiques.iter().map(|c| serde_json::json!({
                    "id": c.id,
                    "text": c.title,
                    "severity": format!("{}", c.severity),
                })).collect::<Vec<_>>(),
            }));
        }
    }

    // 2. READY: Solutions with all critiques resolved + review satisfied
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let has_open = critiques.iter()
            .any(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open);

        if !has_open && !solution.critique_ids.is_empty() {
            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| &p.priority).cloned().unwrap_or_default();

            items.push(serde_json::json!({
                "category": "ready",
                "entity_type": "solution",
                "entity_id": solution.id,
                "title": solution.title,
                "summary": "All critiques resolved",
                "suggested_command": format!("jjj solution accept {}", solution.id),
                "priority": format!("{}", priority),
                "priority_sort": priority_sort_value(&priority),
                "details": [],
            }));
        }
    }

    // 3. REVIEW: Critiques assigned to user that need response
    if !mine {
        for critique in critiques.iter().filter(|c| c.status == CritiqueStatus::Open) {
            if let Some(reviewer) = &critique.reviewer {
                if user.contains(reviewer) || reviewer.contains(user) {
                    let solution = solutions.iter().find(|s| s.id == critique.solution_id);
                    let problem = solution.and_then(|s| problems.iter().find(|p| p.id == s.problem_id));
                    let priority = problem.map(|p| &p.priority).cloned().unwrap_or_default();

                    items.push(serde_json::json!({
                        "category": "review",
                        "entity_type": "critique",
                        "entity_id": critique.id,
                        "title": critique.title,
                        "summary": format!("Review requested on {}", critique.solution_id),
                        "suggested_command": format!("jjj critique show {}", critique.id),
                        "priority": format!("{}", priority),
                        "priority_sort": priority_sort_value(&priority),
                        "details": [],
                    }));
                }
            }
        }
    }

    // 4. WAITING: User's solutions with pending review critiques (assigned to others)
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let is_mine = solution.assignee.as_ref().map(|a| user == *a).unwrap_or(false);
        if is_mine {
            let pending_reviews: Vec<_> = critiques.iter()
                .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
                .filter(|c| c.reviewer.is_some() && c.reviewer.as_ref().map(|r| !user.contains(r)).unwrap_or(false))
                .collect();

            if !pending_reviews.is_empty() {
                let problem = problems.iter().find(|p| p.id == solution.problem_id);
                let priority = problem.map(|p| &p.priority).cloned().unwrap_or_default();
                let reviewers: Vec<_> = pending_reviews.iter()
                    .filter_map(|c| c.reviewer.as_ref())
                    .map(|r| format!("@{}", r))
                    .collect();

                items.push(serde_json::json!({
                    "category": "waiting",
                    "entity_type": "solution",
                    "entity_id": solution.id,
                    "title": solution.title,
                    "summary": format!("Awaiting review from {}", reviewers.join(", ")),
                    "suggested_command": "",
                    "priority": format!("{}", priority),
                    "priority_sort": priority_sort_value(&priority),
                    "details": [],
                }));
            }
        }
    }

    // 5. TODO: Open problems with no active solutions
    for problem in problems.iter().filter(|p| p.is_open()) {
        let has_active_solution = solutions.iter()
            .any(|s| s.problem_id == problem.id && s.is_active());

        if !has_active_solution {
            items.push(serde_json::json!({
                "category": "todo",
                "entity_type": "problem",
                "entity_id": problem.id,
                "title": problem.title,
                "summary": "No solutions proposed",
                "suggested_command": format!("jjj solution new \"title\" --problem {}", problem.id),
                "priority": format!("{}", problem.priority),
                "priority_sort": priority_sort_value(&problem.priority),
                "details": [],
            }));
        }
    }

    // Sort: category order first, then priority descending within each category
    items.sort_by(|a, b| {
        let cat_order = |cat: &str| -> i32 {
            match cat {
                "blocked" => 0,
                "ready" => 1,
                "review" => 2,
                "waiting" => 3,
                "todo" => 4,
                _ => 5,
            }
        };
        let a_cat = cat_order(a["category"].as_str().unwrap_or(""));
        let b_cat = cat_order(b["category"].as_str().unwrap_or(""));
        if a_cat != b_cat {
            return a_cat.cmp(&b_cat);
        }
        // Within same category, sort by priority descending
        let a_pri = a["priority_sort"].as_i64().unwrap_or(0);
        let b_pri = b["priority_sort"].as_i64().unwrap_or(0);
        b_pri.cmp(&a_pri)
    });

    items
}
