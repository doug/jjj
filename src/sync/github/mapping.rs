//! Mapping between GitHub JSON responses and jjj entities.

use crate::error::{JjjError, Result};
use crate::models::{Priority, Problem};
use crate::sync::{ReviewInfo, ReviewState};

/// Convert a GitHub issue JSON object to a Problem.
pub fn issue_to_problem(json: &serde_json::Value, number: u64) -> Result<Problem> {
    let title = json["title"]
        .as_str()
        .ok_or_else(|| JjjError::Other("Issue missing title".to_string()))?;

    let body = json["body"].as_str().unwrap_or("");
    let id = crate::id::generate_id();

    let mut problem = Problem::new(id, title.to_string());
    problem.description = body.to_string();
    problem.github_issue = Some(number);

    // Map GitHub labels to priority
    if let Some(labels) = json["labels"].as_array() {
        for label in labels {
            if let Some(name) = label["name"].as_str() {
                match name.to_lowercase().as_str() {
                    "p0" | "critical" => problem.priority = Priority::Critical,
                    "p1" | "high" | "priority: high" => problem.priority = Priority::High,
                    "p2" | "medium" | "priority: medium" => problem.priority = Priority::Medium,
                    "p3" | "low" | "priority: low" => problem.priority = Priority::Low,
                    _ => {}
                }
            }
        }
    }

    // Map author to assignee context
    if let Some(author) = json["author"]["login"].as_str() {
        problem.context = format!("Imported from GitHub issue #{} by @{}", number, author);
    }

    Ok(problem)
}

/// Format a Problem as a GitHub issue body.
pub fn problem_to_issue_body(problem: &Problem) -> String {
    let mut body = String::new();

    if !problem.description.is_empty() {
        body.push_str(&problem.description);
        body.push_str("\n\n");
    }

    if !problem.context.is_empty() {
        body.push_str("## Context\n\n");
        body.push_str(&problem.context);
        body.push_str("\n\n");
    }

    body.push_str(&format!(
        "---\n*Synced from jjj (priority: {}, status: {})*",
        problem.priority, problem.status
    ));

    body
}

/// Format a PR body from a solution and its problem.
pub fn format_pr_body(
    solution: &crate::models::Solution,
    problem: &Problem,
    critiques: &[crate::models::Critique],
) -> String {
    let mut body = String::new();

    body.push_str(&format!("Resolves problem: **{}**\n\n", problem.title));

    if let Some(issue_num) = problem.github_issue {
        body.push_str(&format!("Closes #{}\n\n", issue_num));
    }

    if !solution.approach.is_empty() {
        body.push_str("## Approach\n\n");
        body.push_str(&solution.approach);
        body.push_str("\n\n");
    }

    if !solution.tradeoffs.is_empty() {
        body.push_str("## Trade-offs\n\n");
        body.push_str(&solution.tradeoffs);
        body.push_str("\n\n");
    }

    let open_critiques: Vec<_> = critiques.iter().filter(|c| c.is_active()).collect();

    if !open_critiques.is_empty() {
        body.push_str("## Open Critiques\n\n");
        for c in &open_critiques {
            body.push_str(&format!("- **{}** ({})\n", c.title, c.severity));
        }
        body.push('\n');
    }

    body.push_str(&format!(
        "---\n*Synced from jjj (status: {})*",
        solution.status
    ));

    body
}

/// Parse a review JSON array into ReviewInfo structs.
pub fn parse_reviews(json: &serde_json::Value) -> Vec<ReviewInfo> {
    let mut reviews = Vec::new();

    if let Some(arr) = json.as_array() {
        for review in arr {
            let id = review["id"]
                .as_u64()
                .or_else(|| {
                    // gh sometimes returns ID as a string
                    review["id"].as_str().and_then(|s| s.parse().ok())
                })
                .unwrap_or(0);

            let author = review["author"]["login"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            let state_str = review["state"].as_str().unwrap_or("");
            let state = match state_str {
                "APPROVED" => ReviewState::Approved,
                "CHANGES_REQUESTED" => ReviewState::ChangesRequested,
                "COMMENTED" => ReviewState::Commented,
                "DISMISSED" => ReviewState::Dismissed,
                _ => ReviewState::Commented,
            };

            let body = review["body"].as_str().unwrap_or("").to_string();

            reviews.push(ReviewInfo {
                id,
                author,
                state,
                body,
            });
        }
    }

    reviews
}

/// Parse issue state string to IssueStatus.
pub fn parse_issue_state(state: &str) -> crate::sync::IssueStatus {
    match state.trim().to_uppercase().as_str() {
        "OPEN" => crate::sync::IssueStatus::Open,
        _ => crate::sync::IssueStatus::Closed,
    }
}

/// Parse PR state string to PrStatus.
pub fn parse_pr_state(state: &str) -> crate::sync::PrStatus {
    match state.trim().to_uppercase().as_str() {
        "OPEN" => crate::sync::PrStatus::Open,
        "MERGED" => crate::sync::PrStatus::Merged,
        _ => crate::sync::PrStatus::Closed,
    }
}
