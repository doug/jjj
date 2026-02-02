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
                details: open_critiques.iter().map(|c| ActionDetail {
                    id: c.id.clone(),
                    text: c.title.clone(),
                    severity: Some(format!("{}", c.severity)),
                }).collect(),
            });
        }
    }

    // 2. READY: Solutions ready to accept
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let has_open = critiques.iter()
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
    for critique in critiques.iter().filter(|c| c.status == CritiqueStatus::Open) {
        if let Some(reviewer) = &critique.reviewer {
            if user.contains(reviewer) || reviewer.contains(user) {
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

    // 4. TODO: Open problems with no active solutions
    for problem in problems.iter().filter(|p| p.is_open()) {
        let has_active = solutions.iter()
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
