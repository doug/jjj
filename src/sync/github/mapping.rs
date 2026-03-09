//! Mapping between GitHub JSON responses and jjj entities.

use crate::error::{JjjError, Result};
use crate::models::{Priority, Problem};
use crate::sync::{ReviewInfo, ReviewState, ReviewThread};

/// Convert a GitHub issue JSON object to a Problem.
pub fn issue_to_problem(json: &serde_json::Value, number: u64) -> Result<Problem> {
    let title = json["title"]
        .as_str()
        .ok_or_else(|| JjjError::Validation("Issue missing title".to_string()))?;

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

/// Parse inline review threads from `gh pr view --json reviewThreads`.
///
/// Each thread element has `isResolved`, `isOutdated`, and a `comments`
/// array.  We extract only the first comment of each thread (the root),
/// skipping threads with empty bodies.
pub fn parse_review_threads(json: &serde_json::Value) -> Vec<ReviewThread> {
    let mut threads = Vec::new();

    let arr = match json.as_array() {
        Some(a) => a,
        None => return threads,
    };

    for thread in arr {
        let is_resolved = thread["isResolved"].as_bool().unwrap_or(false);
        let is_outdated = thread["isOutdated"].as_bool().unwrap_or(false);

        // `comments` is a direct array in `gh` CLI output
        let comments = match thread["comments"].as_array() {
            Some(c) => c,
            None => continue,
        };
        let first = match comments.first() {
            Some(c) => c,
            None => continue,
        };

        let comment_id = first["databaseId"].as_u64().unwrap_or(0);
        if comment_id == 0 {
            continue;
        }

        let author = first["author"]["login"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let body = first["body"].as_str().unwrap_or("").to_string();
        if body.is_empty() {
            continue;
        }

        let path = first["path"].as_str().unwrap_or("").to_string();

        // `line` is the right-side line; fall back to `originalLine` for
        // outdated threads where `line` may be null.
        let line = first["line"]
            .as_u64()
            .or_else(|| first["originalLine"].as_u64())
            .map(|l| l as usize);

        threads.push(ReviewThread {
            comment_id,
            author,
            body,
            path,
            line,
            is_resolved,
            is_outdated,
        });
    }

    threads
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use crate::sync::{IssueStatus, PrStatus, ReviewState};
    use serde_json::json;

    // ── issue_to_problem ──────────────────────────────────────────────

    #[test]
    fn test_issue_to_problem_basic() {
        let issue = json!({
            "title": "Login fails when session expires",
            "body": "Users are logged out unexpectedly after 30 minutes.",
            "number": 42,
            "labels": [
                { "name": "bug" },
                { "name": "auth" }
            ],
            "author": {
                "login": "octocat"
            }
        });

        let problem = issue_to_problem(&issue, 42).unwrap();

        assert_eq!(problem.title, "Login fails when session expires");
        assert_eq!(
            problem.description,
            "Users are logged out unexpectedly after 30 minutes."
        );
        assert_eq!(problem.github_issue, Some(42));
        assert!(problem.context.contains("#42"));
        assert!(problem.context.contains("@octocat"));
        // No priority label, so default Medium
        assert_eq!(problem.priority, Priority::Medium);
        assert_eq!(problem.status, ProblemStatus::Open);
    }

    #[test]
    fn test_issue_to_problem_priority_p0_critical() {
        let issue = json!({
            "title": "Data loss on save",
            "body": "",
            "labels": [{ "name": "p0" }],
            "author": { "login": "alice" }
        });
        let p = issue_to_problem(&issue, 1).unwrap();
        assert_eq!(p.priority, Priority::Critical);

        let issue2 = json!({
            "title": "Data loss on save",
            "body": "",
            "labels": [{ "name": "critical" }],
            "author": { "login": "alice" }
        });
        let p2 = issue_to_problem(&issue2, 2).unwrap();
        assert_eq!(p2.priority, Priority::Critical);
    }

    #[test]
    fn test_issue_to_problem_priority_p1_high() {
        for label in &["p1", "high", "priority: high"] {
            let issue = json!({
                "title": "Slow query",
                "body": "",
                "labels": [{ "name": *label }],
                "author": { "login": "bob" }
            });
            let p = issue_to_problem(&issue, 10).unwrap();
            assert_eq!(
                p.priority,
                Priority::High,
                "label '{}' should map to High",
                label
            );
        }
    }

    #[test]
    fn test_issue_to_problem_priority_p2_medium() {
        for label in &["p2", "medium", "priority: medium"] {
            let issue = json!({
                "title": "UI glitch",
                "body": "",
                "labels": [{ "name": *label }],
                "author": { "login": "carol" }
            });
            let p = issue_to_problem(&issue, 20).unwrap();
            assert_eq!(
                p.priority,
                Priority::Medium,
                "label '{}' should map to Medium",
                label
            );
        }
    }

    #[test]
    fn test_issue_to_problem_priority_p3_low() {
        for label in &["p3", "low", "priority: low"] {
            let issue = json!({
                "title": "Typo in docs",
                "body": "",
                "labels": [{ "name": *label }],
                "author": { "login": "dave" }
            });
            let p = issue_to_problem(&issue, 30).unwrap();
            assert_eq!(
                p.priority,
                Priority::Low,
                "label '{}' should map to Low",
                label
            );
        }
    }

    #[test]
    fn test_issue_to_problem_missing_title() {
        let issue = json!({
            "body": "No title here",
            "labels": [],
            "author": { "login": "eve" }
        });
        let result = issue_to_problem(&issue, 99);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("missing title"),
            "Error should mention missing title, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_issue_to_problem_null_body() {
        let issue = json!({
            "title": "Issue with null body",
            "body": null,
            "labels": [],
            "author": { "login": "frank" }
        });
        let p = issue_to_problem(&issue, 5).unwrap();
        assert_eq!(p.description, "");
    }

    #[test]
    fn test_issue_to_problem_no_author() {
        let issue = json!({
            "title": "Anonymous issue",
            "body": "some body",
            "labels": []
        });
        let p = issue_to_problem(&issue, 7).unwrap();
        // Without an author, context should not mention @
        assert!(!p.context.contains("@"));
    }

    // ── problem_to_issue_body ─────────────────────────────────────────

    #[test]
    fn test_problem_to_issue_body_with_description_and_context() {
        let mut problem = Problem::new("P-100".to_string(), "Auth tokens expire".to_string());
        problem.description = "Tokens expire after 15 min causing UX friction.".to_string();
        problem.context = "Reported by enterprise customers on SSO plan.".to_string();
        problem.priority = Priority::High;

        let body = problem_to_issue_body(&problem);

        assert!(body.contains("Tokens expire after 15 min causing UX friction."));
        assert!(body.contains("## Context"));
        assert!(body.contains("Reported by enterprise customers on SSO plan."));
        assert!(body.contains("p1"));
        assert!(body.contains("open"));
        assert!(body.contains("Synced from jjj"));
    }

    #[test]
    fn test_problem_to_issue_body_empty_description() {
        let problem = Problem::new("P-200".to_string(), "Bare problem".to_string());
        let body = problem_to_issue_body(&problem);

        // Should NOT have description section but should have footer
        assert!(!body.starts_with("\n\n"));
        assert!(body.contains("Synced from jjj"));
        assert!(body.contains("p2"));
    }

    #[test]
    fn test_problem_to_issue_body_empty_context() {
        let mut problem = Problem::new("P-300".to_string(), "No context".to_string());
        problem.description = "Has a description.".to_string();

        let body = problem_to_issue_body(&problem);

        assert!(body.contains("Has a description."));
        assert!(!body.contains("## Context"));
    }

    // ── format_pr_body ────────────────────────────────────────────────

    #[test]
    fn test_format_pr_body_with_critiques() {
        let mut problem = Problem::new("P-1".to_string(), "Slow database queries".to_string());
        problem.github_issue = Some(42);

        let mut solution = Solution::new(
            "S-1".to_string(),
            "Add query caching".to_string(),
            "P-1".to_string(),
        );
        solution.approach = "Use Redis as a read-through cache.".to_string();
        solution.tradeoffs = "Adds operational complexity of running Redis.".to_string();

        let critique = Critique::new(
            "C-1".to_string(),
            "Cache invalidation is error-prone".to_string(),
            "S-1".to_string(),
        );

        let body = format_pr_body(&solution, &problem, &[critique]);

        assert!(body.contains("Resolves problem: **Slow database queries**"));
        assert!(body.contains("Closes #42"));
        assert!(body.contains("## Approach"));
        assert!(body.contains("Use Redis as a read-through cache."));
        assert!(body.contains("## Trade-offs"));
        assert!(body.contains("Adds operational complexity of running Redis."));
        assert!(body.contains("## Open Critiques"));
        assert!(body.contains("Cache invalidation is error-prone"));
        assert!(body.contains("Synced from jjj"));
    }

    #[test]
    fn test_format_pr_body_no_critiques() {
        let problem = Problem::new("P-2".to_string(), "UI is too slow".to_string());

        let mut solution = Solution::new(
            "S-2".to_string(),
            "Virtual scrolling".to_string(),
            "P-2".to_string(),
        );
        solution.approach = "Implement virtual list rendering.".to_string();

        let body = format_pr_body(&solution, &problem, &[]);

        assert!(body.contains("Resolves problem: **UI is too slow**"));
        assert!(!body.contains("Closes #")); // no github_issue
        assert!(body.contains("## Approach"));
        assert!(!body.contains("## Open Critiques"));
    }

    #[test]
    fn test_format_pr_body_resolved_critiques_excluded() {
        let problem = Problem::new("P-3".to_string(), "Memory leak".to_string());
        let solution = Solution::new(
            "S-3".to_string(),
            "Fix allocator".to_string(),
            "P-3".to_string(),
        );

        let mut addressed = Critique::new(
            "C-2".to_string(),
            "Already addressed".to_string(),
            "S-3".to_string(),
        );
        addressed.address();

        let mut dismissed = Critique::new(
            "C-3".to_string(),
            "Not relevant".to_string(),
            "S-3".to_string(),
        );
        dismissed.dismiss();

        let body = format_pr_body(&solution, &problem, &[addressed, dismissed]);

        // Neither addressed nor dismissed critiques should appear
        assert!(!body.contains("## Open Critiques"));
        assert!(!body.contains("Already addressed"));
        assert!(!body.contains("Not relevant"));
    }

    // ── parse_reviews ─────────────────────────────────────────────────

    #[test]
    fn test_parse_reviews_mixed_states() {
        let reviews = json!([
            {
                "id": 1001,
                "author": { "login": "alice" },
                "state": "APPROVED",
                "body": "LGTM!"
            },
            {
                "id": 1002,
                "author": { "login": "bob" },
                "state": "CHANGES_REQUESTED",
                "body": "Please fix the error handling."
            },
            {
                "id": 1003,
                "author": { "login": "carol" },
                "state": "COMMENTED",
                "body": "Interesting approach."
            },
            {
                "id": 1004,
                "author": { "login": "dave" },
                "state": "DISMISSED",
                "body": "Superseded by new review."
            }
        ]);

        let parsed = parse_reviews(&reviews);

        assert_eq!(parsed.len(), 4);

        assert_eq!(parsed[0].id, 1001);
        assert_eq!(parsed[0].author, "alice");
        assert_eq!(parsed[0].state, ReviewState::Approved);
        assert_eq!(parsed[0].body, "LGTM!");

        assert_eq!(parsed[1].id, 1002);
        assert_eq!(parsed[1].author, "bob");
        assert_eq!(parsed[1].state, ReviewState::ChangesRequested);
        assert_eq!(parsed[1].body, "Please fix the error handling.");

        assert_eq!(parsed[2].id, 1003);
        assert_eq!(parsed[2].author, "carol");
        assert_eq!(parsed[2].state, ReviewState::Commented);

        assert_eq!(parsed[3].id, 1004);
        assert_eq!(parsed[3].author, "dave");
        assert_eq!(parsed[3].state, ReviewState::Dismissed);
    }

    #[test]
    fn test_parse_reviews_empty_array() {
        let reviews = json!([]);
        let parsed = parse_reviews(&reviews);
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_reviews_non_array_input() {
        let not_array = json!({ "error": "not found" });
        let parsed = parse_reviews(&not_array);
        assert!(parsed.is_empty());

        let null_input = json!(null);
        let parsed2 = parse_reviews(&null_input);
        assert!(parsed2.is_empty());
    }

    #[test]
    fn test_parse_reviews_unknown_state_defaults_to_commented() {
        let reviews = json!([
            {
                "id": 2001,
                "author": { "login": "ghost" },
                "state": "PENDING",
                "body": ""
            }
        ]);
        let parsed = parse_reviews(&reviews);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].state, ReviewState::Commented);
    }

    #[test]
    fn test_parse_reviews_string_id() {
        let reviews = json!([
            {
                "id": "12345",
                "author": { "login": "bot" },
                "state": "APPROVED",
                "body": "Auto-approved"
            }
        ]);
        let parsed = parse_reviews(&reviews);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, 12345);
    }

    // ── parse_review_threads ──────────────────────────────────────────

    #[test]
    fn test_parse_review_threads_basic() {
        let threads = json!([
            {
                "isResolved": false,
                "isOutdated": false,
                "comments": [
                    {
                        "databaseId": 111111,
                        "author": { "login": "alice" },
                        "body": "This needs error handling",
                        "path": "src/auth.rs",
                        "line": 42,
                        "originalLine": 42
                    }
                ]
            }
        ]);

        let parsed = parse_review_threads(&threads);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].comment_id, 111111);
        assert_eq!(parsed[0].author, "alice");
        assert_eq!(parsed[0].body, "This needs error handling");
        assert_eq!(parsed[0].path, "src/auth.rs");
        assert_eq!(parsed[0].line, Some(42));
        assert!(!parsed[0].is_resolved);
        assert!(!parsed[0].is_outdated);
    }

    #[test]
    fn test_parse_review_threads_resolved_included() {
        // Resolved threads are returned — caller decides whether to skip them
        let threads = json!([
            {
                "isResolved": true,
                "isOutdated": false,
                "comments": [
                    {
                        "databaseId": 222222,
                        "author": { "login": "bob" },
                        "body": "Already fixed this",
                        "path": "src/lib.rs",
                        "line": 10,
                        "originalLine": 10
                    }
                ]
            }
        ]);

        let parsed = parse_review_threads(&threads);
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].is_resolved);
    }

    #[test]
    fn test_parse_review_threads_outdated_uses_original_line() {
        let threads = json!([
            {
                "isResolved": false,
                "isOutdated": true,
                "comments": [
                    {
                        "databaseId": 333333,
                        "author": { "login": "carol" },
                        "body": "Comment on stale diff",
                        "path": "src/foo.rs",
                        "line": null,
                        "originalLine": 99
                    }
                ]
            }
        ]);

        let parsed = parse_review_threads(&threads);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].line, Some(99));
        assert!(parsed[0].is_outdated);
    }

    #[test]
    fn test_parse_review_threads_skips_empty_body() {
        let threads = json!([
            {
                "isResolved": false,
                "isOutdated": false,
                "comments": [
                    {
                        "databaseId": 444444,
                        "author": { "login": "dave" },
                        "body": "",
                        "path": "src/bar.rs",
                        "line": 5,
                        "originalLine": 5
                    }
                ]
            }
        ]);

        let parsed = parse_review_threads(&threads);
        assert!(parsed.is_empty(), "Empty body threads should be skipped");
    }

    #[test]
    fn test_parse_review_threads_skips_zero_database_id() {
        let threads = json!([
            {
                "isResolved": false,
                "isOutdated": false,
                "comments": [
                    {
                        "author": { "login": "eve" },
                        "body": "Some comment",
                        "path": "src/baz.rs",
                        "line": 1
                    }
                ]
            }
        ]);

        let parsed = parse_review_threads(&threads);
        assert!(
            parsed.is_empty(),
            "Threads without databaseId should be skipped"
        );
    }

    #[test]
    fn test_parse_review_threads_empty_array() {
        let threads = json!([]);
        let parsed = parse_review_threads(&threads);
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_review_threads_multiple() {
        let threads = json!([
            {
                "isResolved": false,
                "isOutdated": false,
                "comments": [
                    {
                        "databaseId": 555555,
                        "author": { "login": "alice" },
                        "body": "First comment",
                        "path": "src/a.rs",
                        "line": 10
                    }
                ]
            },
            {
                "isResolved": false,
                "isOutdated": false,
                "comments": [
                    {
                        "databaseId": 666666,
                        "author": { "login": "bob" },
                        "body": "Second comment",
                        "path": "src/b.rs",
                        "line": 20
                    }
                ]
            }
        ]);

        let parsed = parse_review_threads(&threads);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].comment_id, 555555);
        assert_eq!(parsed[1].comment_id, 666666);
    }

    // ── parse_issue_state ─────────────────────────────────────────────

    #[test]
    fn test_parse_issue_state_open() {
        assert_eq!(parse_issue_state("OPEN"), IssueStatus::Open);
    }

    #[test]
    fn test_parse_issue_state_closed() {
        assert_eq!(parse_issue_state("CLOSED"), IssueStatus::Closed);
    }

    #[test]
    fn test_parse_issue_state_lowercase() {
        assert_eq!(parse_issue_state("open"), IssueStatus::Open);
        assert_eq!(parse_issue_state("closed"), IssueStatus::Closed);
    }

    #[test]
    fn test_parse_issue_state_with_whitespace() {
        assert_eq!(parse_issue_state("  OPEN  "), IssueStatus::Open);
        assert_eq!(parse_issue_state("  CLOSED\n"), IssueStatus::Closed);
    }

    #[test]
    fn test_parse_issue_state_unknown_defaults_to_closed() {
        assert_eq!(parse_issue_state("INVALID"), IssueStatus::Closed);
        assert_eq!(parse_issue_state(""), IssueStatus::Closed);
    }

    // ── parse_pr_state ────────────────────────────────────────────────

    #[test]
    fn test_parse_pr_state_open() {
        assert_eq!(parse_pr_state("OPEN"), PrStatus::Open);
    }

    #[test]
    fn test_parse_pr_state_merged() {
        assert_eq!(parse_pr_state("MERGED"), PrStatus::Merged);
    }

    #[test]
    fn test_parse_pr_state_closed() {
        assert_eq!(parse_pr_state("CLOSED"), PrStatus::Closed);
    }

    #[test]
    fn test_parse_pr_state_unknown_defaults_to_closed() {
        assert_eq!(parse_pr_state("DRAFT"), PrStatus::Closed);
        assert_eq!(parse_pr_state("something_else"), PrStatus::Closed);
        assert_eq!(parse_pr_state(""), PrStatus::Closed);
    }

    #[test]
    fn test_parse_pr_state_case_insensitive() {
        assert_eq!(parse_pr_state("open"), PrStatus::Open);
        assert_eq!(parse_pr_state("merged"), PrStatus::Merged);
        assert_eq!(parse_pr_state("Closed"), PrStatus::Closed);
    }
}
