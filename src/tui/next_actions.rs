use crate::models::{Critique, CritiqueStatus, Priority, Problem, Solution};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub category: Category,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub title: String,
    pub summary: String,
    pub priority: Priority,
    pub details: Vec<ActionDetail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Blocked,
    Ready,
    Review,
    Waiting,
    Todo,
}

impl Category {
    pub fn sort_order(&self) -> i32 {
        match self {
            Category::Blocked => 0,
            Category::Ready => 1,
            Category::Review => 2,
            Category::Waiting => 3,
            Category::Todo => 4,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Category::Blocked => "BLOCKED",
            Category::Ready => "READY",
            Category::Review => "REVIEW",
            Category::Waiting => "WAITING",
            Category::Todo => "TODO",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Problem,
    Solution,
    Critique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDetail {
    pub id: String,
    pub text: String,
    pub severity: Option<String>,
}

pub fn build_next_actions(
    problems: &[Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    user: &str,
) -> Vec<NextAction> {
    let mut items = Vec::new();

    // 1. BLOCKED: Solutions with open critiques
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let open_critiques: Vec<&Critique> = critiques
            .iter()
            .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
            .collect();

        if !open_critiques.is_empty() {
            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| p.priority.clone()).unwrap_or_default();

            items.push(NextAction {
                category: Category::Blocked,
                entity_type: EntityType::Solution,
                entity_id: solution.id.clone(),
                title: solution.title.clone(),
                summary: format!("{} open critique(s)", open_critiques.len()),
                priority,
                details: open_critiques
                    .iter()
                    .map(|c| ActionDetail {
                        id: c.id.clone(),
                        text: c.title.clone(),
                        severity: Some(format!("{}", c.severity)),
                    })
                    .collect(),
            });
        }
    }

    // 2. READY: Solutions ready to accept
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let has_open = critiques
            .iter()
            .any(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open);

        if !has_open && !solution.critique_ids.is_empty() {
            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| p.priority.clone()).unwrap_or_default();

            items.push(NextAction {
                category: Category::Ready,
                entity_type: EntityType::Solution,
                entity_id: solution.id.clone(),
                title: solution.title.clone(),
                summary: "All critiques resolved".to_string(),
                priority,
                details: vec![],
            });
        }
    }

    // 3. REVIEW: Critiques assigned to user
    for critique in critiques
        .iter()
        .filter(|c| c.status == CritiqueStatus::Open)
    {
        if let Some(reviewer) = &critique.reviewer {
            if user == reviewer || user.contains(reviewer) {
                let solution = solutions.iter().find(|s| s.id == critique.solution_id);
                let problem = solution.and_then(|s| problems.iter().find(|p| p.id == s.problem_id));
                let priority = problem.map(|p| p.priority.clone()).unwrap_or_default();

                items.push(NextAction {
                    category: Category::Review,
                    entity_type: EntityType::Critique,
                    entity_id: critique.id.clone(),
                    title: critique.title.clone(),
                    summary: format!("Review on {}", critique.solution_id),
                    priority,
                    details: vec![],
                });
            }
        }
    }

    // 4. Open problems with no active solutions
    for problem in problems.iter().filter(|p| p.is_open()) {
        let has_active = solutions
            .iter()
            .any(|s| s.problem_id == problem.id && s.is_active());

        if !has_active {
            items.push(NextAction {
                category: Category::Todo,
                entity_type: EntityType::Problem,
                entity_id: problem.id.clone(),
                title: problem.title.clone(),
                summary: "No solutions proposed".to_string(),
                priority: problem.priority.clone(),
                details: vec![],
            });
        }
    }

    // Sort by category then priority
    items.sort_by(|a, b| {
        let cat_cmp = a.category.sort_order().cmp(&b.category.sort_order());
        if cat_cmp != std::cmp::Ordering::Equal {
            return cat_cmp;
        }
        b.priority.cmp(&a.priority)
    });

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Critique, CritiqueSeverity, CritiqueStatus, Priority, Problem, ProblemStatus, Solution,
        SolutionStatus,
    };

    // --- Helper functions ---

    fn make_problem(id: &str, title: &str) -> Problem {
        Problem::new(id.to_string(), title.to_string())
    }

    fn make_problem_with_priority(id: &str, title: &str, priority: Priority) -> Problem {
        let mut p = Problem::new(id.to_string(), title.to_string());
        p.priority = priority;
        p
    }

    fn make_solution(id: &str, title: &str, problem_id: &str) -> Solution {
        Solution::new(id.to_string(), title.to_string(), problem_id.to_string())
    }

    fn make_solution_with_critiques(
        id: &str,
        title: &str,
        problem_id: &str,
        critique_ids: Vec<&str>,
    ) -> Solution {
        let mut s = Solution::new(id.to_string(), title.to_string(), problem_id.to_string());
        for cid in critique_ids {
            s.add_critique(cid.to_string());
        }
        s
    }

    fn make_critique(id: &str, title: &str, solution_id: &str) -> Critique {
        Critique::new(id.to_string(), title.to_string(), solution_id.to_string())
    }

    fn make_critique_with_reviewer(
        id: &str,
        title: &str,
        solution_id: &str,
        reviewer: &str,
    ) -> Critique {
        let mut c = Critique::new(id.to_string(), title.to_string(), solution_id.to_string());
        c.reviewer = Some(reviewer.to_string());
        c
    }

    fn make_resolved_critique(
        id: &str,
        title: &str,
        solution_id: &str,
        status: CritiqueStatus,
    ) -> Critique {
        let mut c = Critique::new(id.to_string(), title.to_string(), solution_id.to_string());
        c.set_status(status);
        c
    }

    // --- Tests ---

    #[test]
    fn test_empty_inputs_return_empty_actions() {
        let actions = build_next_actions(&[], &[], &[], "alice");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_empty_problems_with_solutions_and_critiques() {
        // Solutions and critiques without matching problems should still work
        let solutions = vec![make_solution("S-1", "Fix auth", "P-1")];
        let critiques = vec![make_critique("C-1", "Flaw", "S-1")];
        let actions = build_next_actions(&[], &solutions, &critiques, "alice");
        // Should still produce a BLOCKED action for S-1 (open critique exists)
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].category, Category::Blocked);
    }

    #[test]
    fn test_solution_with_open_critiques_returns_blocked() {
        let problems = vec![make_problem("P-1", "Auth bug")];
        let solutions = vec![make_solution("S-1", "Fix with JWT", "P-1")];
        let critiques = vec![
            make_critique("C-1", "XSS vulnerability", "S-1"),
            make_critique("C-2", "Token size too large", "S-1"),
        ];

        let actions = build_next_actions(&problems, &solutions, &critiques, "alice");

        let blocked: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Blocked)
            .collect();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].entity_id, "S-1");
        assert_eq!(blocked[0].entity_type, EntityType::Solution);
        assert_eq!(blocked[0].details.len(), 2);
        assert!(blocked[0].summary.contains("2 open critique(s)"));
    }

    #[test]
    fn test_solution_with_all_critiques_resolved_returns_ready() {
        let problems = vec![make_problem("P-1", "Auth bug")];
        let solutions = vec![make_solution_with_critiques(
            "S-1",
            "Fix with JWT",
            "P-1",
            vec!["C-1", "C-2"],
        )];
        let critiques = vec![
            make_resolved_critique("C-1", "XSS fixed", "S-1", CritiqueStatus::Addressed),
            make_resolved_critique("C-2", "Size OK", "S-1", CritiqueStatus::Dismissed),
        ];

        let actions = build_next_actions(&problems, &solutions, &critiques, "alice");

        let ready: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Ready)
            .collect();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].entity_id, "S-1");
        assert_eq!(ready[0].entity_type, EntityType::Solution);
        assert_eq!(ready[0].summary, "All critiques resolved");
    }

    #[test]
    fn test_solution_with_no_critiques_not_ready() {
        // A solution with no critiques at all should NOT appear as READY
        // (READY requires critique_ids to be non-empty and all resolved)
        let problems = vec![make_problem("P-1", "Auth bug")];
        let solutions = vec![make_solution("S-1", "Fix with JWT", "P-1")];

        let actions = build_next_actions(&problems, &solutions, &[], "alice");

        let ready: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Ready)
            .collect();
        assert!(ready.is_empty());
    }

    #[test]
    fn test_critique_assigned_to_user_returns_review() {
        let problems = vec![make_problem("P-1", "Auth bug")];
        let solutions = vec![make_solution("S-1", "Fix with JWT", "P-1")];
        let critiques = vec![make_critique_with_reviewer(
            "C-1",
            "Check XSS",
            "S-1",
            "alice",
        )];

        let actions = build_next_actions(&problems, &solutions, &critiques, "alice");

        let review: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Review)
            .collect();
        assert_eq!(review.len(), 1);
        assert_eq!(review[0].entity_id, "C-1");
        assert_eq!(review[0].entity_type, EntityType::Critique);
        assert!(review[0].summary.contains("S-1"));
    }

    #[test]
    fn test_critique_assigned_to_different_user_not_review() {
        let problems = vec![make_problem("P-1", "Auth bug")];
        let solutions = vec![make_solution("S-1", "Fix with JWT", "P-1")];
        let critiques = vec![make_critique_with_reviewer(
            "C-1",
            "Check XSS",
            "S-1",
            "bob",
        )];

        let actions = build_next_actions(&problems, &solutions, &critiques, "alice");

        let review: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Review)
            .collect();
        assert!(review.is_empty());
    }

    #[test]
    fn test_open_problem_with_no_solutions_returns_todo() {
        let problems = vec![make_problem("P-1", "Auth bug")];

        let actions = build_next_actions(&problems, &[], &[], "alice");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].category, Category::Todo);
        assert_eq!(actions[0].entity_id, "P-1");
        assert_eq!(actions[0].entity_type, EntityType::Problem);
        assert_eq!(actions[0].summary, "No solutions proposed");
    }

    #[test]
    fn test_open_problem_with_active_solution_not_todo() {
        let problems = vec![make_problem("P-1", "Auth bug")];
        let solutions = vec![make_solution("S-1", "Fix it", "P-1")];

        let actions = build_next_actions(&problems, &solutions, &[], "alice");

        let todo: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Todo)
            .collect();
        assert!(todo.is_empty());
    }

    #[test]
    fn test_solved_problem_not_todo() {
        let mut p = make_problem("P-1", "Auth bug");
        p.set_status(ProblemStatus::Solved);

        let actions = build_next_actions(&[p], &[], &[], "alice");

        let todo: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Todo)
            .collect();
        assert!(todo.is_empty());
    }

    #[test]
    fn test_refuted_solution_not_blocking() {
        // A refuted solution should not appear in BLOCKED even with open critiques
        let problems = vec![make_problem("P-1", "Auth bug")];
        let mut sol = make_solution("S-1", "Bad approach", "P-1");
        sol.set_status(SolutionStatus::Withdrawn);
        let critiques = vec![make_critique("C-1", "Fatal flaw", "S-1")];

        let actions = build_next_actions(&problems, &[sol], &critiques, "alice");

        let blocked: Vec<_> = actions
            .iter()
            .filter(|a| a.category == Category::Blocked)
            .collect();
        assert!(blocked.is_empty());
    }

    #[test]
    fn test_sorting_blocked_before_ready_before_review_before_todo() {
        let problems = vec![
            make_problem("P-1", "Problem with solution"),
            make_problem("P-2", "Problem needing work"),
        ];
        // S-1 has open critiques -> BLOCKED
        let solutions = vec![
            make_solution("S-1", "Blocked solution", "P-1"),
            make_solution_with_critiques("S-2", "Ready solution", "P-1", vec!["C-2"]),
        ];
        let critiques = vec![
            make_critique("C-1", "Open flaw", "S-1"),
            make_resolved_critique("C-2", "Resolved", "S-2", CritiqueStatus::Addressed),
            make_critique_with_reviewer("C-3", "Review me", "S-1", "alice"),
        ];

        let actions = build_next_actions(&problems, &solutions, &critiques, "alice");

        // Verify order: BLOCKED, READY, REVIEW, TODO
        let categories: Vec<Category> = actions.iter().map(|a| a.category).collect();
        for i in 1..categories.len() {
            assert!(
                categories[i - 1].sort_order() <= categories[i].sort_order(),
                "Category {:?} should come before {:?}",
                categories[i - 1],
                categories[i]
            );
        }

        // Verify all expected categories are present
        assert!(categories.contains(&Category::Blocked));
        assert!(categories.contains(&Category::Ready));
        assert!(categories.contains(&Category::Review));
        assert!(categories.contains(&Category::Todo));
    }

    #[test]
    fn test_priority_sorting_within_same_category() {
        let problems = vec![
            make_problem_with_priority("P-1", "Low priority problem", Priority::Low),
            make_problem_with_priority("P-2", "Critical problem", Priority::Critical),
            make_problem_with_priority("P-3", "High priority problem", Priority::High),
        ];

        // All three are TODO (no solutions)
        let actions = build_next_actions(&problems, &[], &[], "alice");

        assert_eq!(actions.len(), 3);
        // All should be TODO
        for a in &actions {
            assert_eq!(a.category, Category::Todo);
        }
        // Sorted by priority descending: Critical, High, Low
        assert_eq!(actions[0].priority, Priority::Critical);
        assert_eq!(actions[1].priority, Priority::High);
        assert_eq!(actions[2].priority, Priority::Low);
    }

    #[test]
    fn test_blocked_inherits_problem_priority() {
        let problems = vec![make_problem_with_priority(
            "P-1",
            "Critical bug",
            Priority::Critical,
        )];
        let solutions = vec![make_solution("S-1", "Fix it", "P-1")];
        let critiques = vec![make_critique("C-1", "Flaw", "S-1")];

        let actions = build_next_actions(&problems, &solutions, &critiques, "alice");

        let blocked = actions
            .iter()
            .find(|a| a.category == Category::Blocked)
            .unwrap();
        assert_eq!(blocked.priority, Priority::Critical);
    }

    #[test]
    fn test_critique_severity_in_details() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let mut crit = make_critique("C-1", "Big flaw", "S-1");
        crit.set_severity(CritiqueSeverity::Critical);

        let actions = build_next_actions(&problems, &solutions, &[crit], "alice");

        let blocked = actions
            .iter()
            .find(|a| a.category == Category::Blocked)
            .unwrap();
        assert_eq!(blocked.details.len(), 1);
        assert_eq!(blocked.details[0].severity, Some("critical".to_string()));
    }
}
