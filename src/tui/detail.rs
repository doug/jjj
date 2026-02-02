use crate::models::{Critique, Milestone, Problem, Solution};

pub enum DetailContent {
    None,
    Problem(Problem),
    Solution(Solution),
    Critique(Critique),
    Milestone(Milestone),
}

impl DetailContent {
    pub fn to_lines(&self) -> Vec<String> {
        match self {
            DetailContent::None => vec!["Select an item to see details".to_string()],
            DetailContent::Problem(p) => {
                let mut lines = vec![
                    format!("Problem: {}", p.id),
                    format!("Title: {}", p.title),
                    format!("Status: {}", p.status),
                    format!("Priority: {}", p.priority),
                    String::new(),
                ];
                if !p.description.is_empty() {
                    lines.push("Description:".to_string());
                    lines.extend(p.description.lines().map(String::from));
                    lines.push(String::new());
                }
                if !p.context.is_empty() {
                    lines.push("Context:".to_string());
                    lines.extend(p.context.lines().map(String::from));
                }
                if let Some(assignee) = &p.assignee {
                    lines.push(format!("Assignee: {}", assignee));
                }
                if let Some(milestone) = &p.milestone_id {
                    lines.push(format!("Milestone: {}", milestone));
                }
                lines
            }
            DetailContent::Solution(s) => {
                let mut lines = vec![
                    format!("Solution: {}", s.id),
                    format!("Title: {}", s.title),
                    format!("Problem: {}", s.problem_id),
                    format!("Status: {}", s.status),
                    String::new(),
                ];
                if !s.approach.is_empty() {
                    lines.push("Approach:".to_string());
                    lines.extend(s.approach.lines().map(String::from));
                    lines.push(String::new());
                }
                if !s.tradeoffs.is_empty() {
                    lines.push("Tradeoffs:".to_string());
                    lines.extend(s.tradeoffs.lines().map(String::from));
                }
                if !s.change_ids.is_empty() {
                    lines.push(String::new());
                    lines.push(format!("Changes: {}", s.change_ids.join(", ")));
                }
                lines
            }
            DetailContent::Critique(c) => {
                let mut lines = vec![
                    format!("Critique: {}", c.id),
                    format!("Title: {}", c.title),
                    format!("Solution: {}", c.solution_id),
                    format!("Status: {}", c.status),
                    format!("Severity: {}", c.severity),
                    String::new(),
                ];
                if !c.argument.is_empty() {
                    lines.push("Argument:".to_string());
                    lines.extend(c.argument.lines().map(String::from));
                    lines.push(String::new());
                }
                if !c.evidence.is_empty() {
                    lines.push("Evidence:".to_string());
                    lines.extend(c.evidence.lines().map(String::from));
                }
                if let Some(file) = &c.file_path {
                    lines.push(String::new());
                    lines.push(format!("Location: {}:{}", file, c.line_start.unwrap_or(0)));
                }
                if !c.replies.is_empty() {
                    lines.push(String::new());
                    lines.push("Replies:".to_string());
                    for reply in &c.replies {
                        lines.push(format!("  {} ({}): {}", reply.author, reply.created_at, reply.body));
                    }
                }
                lines
            }
            DetailContent::Milestone(m) => {
                let mut lines = vec![
                    format!("Milestone: {}", m.id),
                    format!("Title: {}", m.title),
                    format!("Status: {}", m.status),
                ];
                if let Some(date) = &m.target_date {
                    lines.push(format!("Target: {}", date));
                }
                lines.push(String::new());
                if !m.goals.is_empty() {
                    lines.push("Goals:".to_string());
                    lines.extend(m.goals.lines().map(String::from));
                    lines.push(String::new());
                }
                if !m.success_criteria.is_empty() {
                    lines.push("Success Criteria:".to_string());
                    lines.extend(m.success_criteria.lines().map(String::from));
                }
                lines
            }
        }
    }
}
