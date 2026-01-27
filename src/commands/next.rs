use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, Critique, Priority};
use crate::storage::MetadataStore;

fn priority_sort_value(priority: &Priority) -> i32 {
    match priority {
        Priority::Critical => 3,
        Priority::High => 2,
        Priority::Medium => 1,
        Priority::Low => 0,
    }
}

pub fn execute(all: bool, mine: bool, limit: Option<usize>, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let user = store.jj_client.user_identity().unwrap_or_default();
    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;

    let mut items: Vec<serde_json::Value> = Vec::new();
    let effective_limit = if all { usize::MAX } else { limit.unwrap_or(5) };

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

    // 3. REVIEW: Solutions where user is a requested reviewer but hasn't LGTM'd
    if !mine {
        for solution in solutions.iter().filter(|s| s.is_active()) {
            if solution.requested_reviewers.iter().any(|r| user == *r)
                && !solution.reviewed_by.iter().any(|r| user == *r)
            {
                let problem = problems.iter().find(|p| p.id == solution.problem_id);
                let priority = problem.map(|p| &p.priority).cloned().unwrap_or_default();

                items.push(serde_json::json!({
                    "category": "review",
                    "entity_type": "solution",
                    "entity_id": solution.id,
                    "title": solution.title,
                    "summary": format!("Review requested by {}", solution.assignee.as_deref().unwrap_or("unknown")),
                    "suggested_command": format!("jjj solution show {}", solution.id),
                    "priority": format!("{}", priority),
                    "priority_sort": priority_sort_value(&priority),
                    "details": [],
                }));
            }
        }
    }

    // 4. WAITING: User's solutions awaiting review
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let is_mine = solution.assignee.as_ref().map(|a| user == *a).unwrap_or(false);
        if is_mine && !solution.requested_reviewers.is_empty()
            && !solution.has_lgtm_from_requested_reviewer()
        {
            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| &p.priority).cloned().unwrap_or_default();

            items.push(serde_json::json!({
                "category": "waiting",
                "entity_type": "solution",
                "entity_id": solution.id,
                "title": solution.title,
                "summary": format!("Awaiting review from {}", solution.requested_reviewers.join(", ")),
                "suggested_command": "",
                "priority": format!("{}", priority),
                "priority_sort": priority_sort_value(&priority),
                "details": [],
            }));
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
                "suggested_command": format!("jjj start \"solution title\" --problem {}", problem.id),
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

    // Apply limit
    let total_count = items.len();
    items.truncate(effective_limit);

    if json {
        let output = serde_json::json!({
            "items": items,
            "total_count": total_count,
            "user": user,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if items.is_empty() {
            println!("No pending actions. All caught up!");
            return Ok(());
        }

        println!("Next actions:\n");
        for (i, item) in items.iter().enumerate() {
            let category = item["category"].as_str().unwrap_or("").to_uppercase();
            let entity_id = item["entity_id"].as_str().unwrap_or("");
            let title = item["title"].as_str().unwrap_or("");
            let summary = item["summary"].as_str().unwrap_or("");

            println!("{}. [{}] {}: {} — {}", i + 1, category, entity_id, title, summary);

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

    Ok(())
}
