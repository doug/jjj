use crate::models::{
    Critique, CritiqueStatus, Milestone, MilestoneStatus, Priority, Problem, ProblemStatus,
    Solution, SolutionStatus,
};

use super::next_actions::{Category, NextAction};

#[derive(Debug, Clone)]
pub enum TreeNode {
    ProjectRoot {
        expanded: bool,
    },
    Milestone {
        id: String,
        title: String,
        status: MilestoneStatus,
        expanded: bool,
    },
    Backlog {
        expanded: bool,
    },
    Problem {
        id: String,
        title: String,
        status: ProblemStatus,
        priority: Priority,
        assignee: Option<String>,
        expanded: bool,
    },
    Solution {
        id: String,
        title: String,
        status: SolutionStatus,
        assignee: Option<String>,
        expanded: bool,
    },
    Critique {
        id: String,
        title: String,
        status: CritiqueStatus,
        severity: String,
    },
}

impl TreeNode {
    pub fn id(&self) -> &str {
        match self {
            TreeNode::ProjectRoot { .. } => "project-root",
            TreeNode::Milestone { id, .. } => id,
            TreeNode::Backlog { .. } => "backlog",
            TreeNode::Problem { id, .. } => id,
            TreeNode::Solution { id, .. } => id,
            TreeNode::Critique { id, .. } => id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            TreeNode::ProjectRoot { .. } => "Project",
            TreeNode::Milestone { title, .. } => title,
            TreeNode::Backlog { .. } => "Backlog",
            TreeNode::Problem { title, .. } => title,
            TreeNode::Solution { title, .. } => title,
            TreeNode::Critique { title, .. } => title,
        }
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            TreeNode::ProjectRoot { expanded } => *expanded,
            TreeNode::Milestone { expanded, .. } => *expanded,
            TreeNode::Backlog { expanded } => *expanded,
            TreeNode::Problem { expanded, .. } => *expanded,
            TreeNode::Solution { expanded, .. } => *expanded,
            TreeNode::Critique { .. } => false, // Critiques don't expand
        }
    }

    pub fn set_expanded(&mut self, value: bool) {
        match self {
            TreeNode::ProjectRoot { expanded } => *expanded = value,
            TreeNode::Milestone { expanded, .. } => *expanded = value,
            TreeNode::Backlog { expanded } => *expanded = value,
            TreeNode::Problem { expanded, .. } => *expanded = value,
            TreeNode::Solution { expanded, .. } => *expanded = value,
            TreeNode::Critique { .. } => {}
        }
    }

    pub fn can_expand(&self) -> bool {
        !matches!(self, TreeNode::Critique { .. })
    }
}

#[derive(Debug, Clone)]
pub struct FlatTreeItem {
    pub node: TreeNode,
    pub depth: usize,
    pub has_children: bool,
    pub action_symbol: Option<String>, // e.g., "⚡", "🚫", "⏳", "📋", "👀"
}

pub fn build_flat_tree(
    milestones: &[Milestone],
    problems: &[Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    expanded_nodes: &std::collections::HashSet<String>,
) -> Vec<FlatTreeItem> {
    let mut items = Vec::new();

    let root_expanded = expanded_nodes.contains("project-root");
    let has_children = !milestones.is_empty() || !problems.is_empty();
    items.push(FlatTreeItem {
        node: TreeNode::ProjectRoot {
            expanded: root_expanded,
        },
        depth: 0,
        has_children,
        action_symbol: None,
    });

    if !root_expanded {
        return items;
    }

    // Add milestones
    for milestone in milestones {
        let milestone_problems: Vec<_> = problems
            .iter()
            .filter(|p| p.milestone_id.as_ref() == Some(&milestone.id))
            .collect();

        let expanded = expanded_nodes.contains(&milestone.id);
        items.push(FlatTreeItem {
            node: TreeNode::Milestone {
                id: milestone.id.clone(),
                title: milestone.title.clone(),
                status: milestone.status.clone(),
                expanded,
            },
            depth: 1,
            has_children: !milestone_problems.is_empty(),
            action_symbol: None,
        });

        if expanded {
            add_problems(
                &mut items,
                &milestone_problems,
                solutions,
                critiques,
                expanded_nodes,
                2,
            );
        }
    }

    // Add backlog (problems without milestone)
    let backlog_problems: Vec<_> = problems
        .iter()
        .filter(|p| p.milestone_id.is_none())
        .collect();

    let backlog_expanded = expanded_nodes.contains("backlog");
    items.push(FlatTreeItem {
        node: TreeNode::Backlog {
            expanded: backlog_expanded,
        },
        depth: 1,
        has_children: !backlog_problems.is_empty(),
        action_symbol: None,
    });

    if backlog_expanded {
        add_problems(
            &mut items,
            &backlog_problems,
            solutions,
            critiques,
            expanded_nodes,
            2,
        );
    }

    items
}

fn add_problems(
    items: &mut Vec<FlatTreeItem>,
    problems: &[&Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    expanded_nodes: &std::collections::HashSet<String>,
    depth: usize,
) {
    for problem in problems {
        let problem_solutions: Vec<_> = solutions
            .iter()
            .filter(|s| s.problem_id == problem.id)
            .collect();

        let expanded = expanded_nodes.contains(&problem.id);
        items.push(FlatTreeItem {
            node: TreeNode::Problem {
                id: problem.id.clone(),
                title: problem.title.clone(),
                status: problem.status.clone(),
                priority: problem.priority.clone(),
                assignee: problem.assignee.clone(),
                expanded,
            },
            depth,
            has_children: !problem_solutions.is_empty(),
            action_symbol: None,
        });

        if expanded {
            for solution in problem_solutions {
                let solution_critiques: Vec<_> = critiques
                    .iter()
                    .filter(|c| c.solution_id == solution.id)
                    .collect();

                let sol_expanded = expanded_nodes.contains(&solution.id);
                items.push(FlatTreeItem {
                    node: TreeNode::Solution {
                        id: solution.id.clone(),
                        title: solution.title.clone(),
                        status: solution.status.clone(),
                        assignee: solution.assignee.clone(),
                        expanded: sol_expanded,
                    },
                    depth: depth + 1,
                    has_children: !solution_critiques.is_empty(),
                    action_symbol: None,
                });

                if sol_expanded {
                    for critique in solution_critiques {
                        items.push(FlatTreeItem {
                            node: TreeNode::Critique {
                                id: critique.id.clone(),
                                title: critique.title.clone(),
                                status: critique.status.clone(),
                                severity: format!("{}", critique.severity),
                            },
                            depth: depth + 2,
                            has_children: false,
                            action_symbol: None,
                        });
                    }
                }
            }
        }
    }
}

/// Annotates tree items with action symbols based on next_actions list
pub fn annotate_tree_with_actions(items: &mut [FlatTreeItem], next_actions: &[NextAction]) {
    use std::collections::HashMap;

    // Build lookup from entity_id -> category
    let action_map: HashMap<&str, Category> = next_actions
        .iter()
        .map(|a| (a.entity_id.as_str(), a.category))
        .collect();

    for item in items.iter_mut() {
        let id = item.node.id();
        if let Some(&category) = action_map.get(id) {
            item.action_symbol = Some(category_to_symbol(category).to_string());
        }
    }
}

fn category_to_symbol(category: Category) -> &'static str {
    match category {
        Category::Ready => "!",
        Category::Blocked => "X",
        Category::Waiting => "~",
        Category::Todo => "*",
        Category::Review => "?",
    }
}

/// Filters tree to only show action items and their ancestors
pub fn filter_tree_to_actions(items: &[FlatTreeItem]) -> Vec<FlatTreeItem> {
    // First pass: collect indices of items with action symbols
    let action_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.action_symbol.is_some())
        .map(|(i, _)| i)
        .collect();

    if action_indices.is_empty() {
        return Vec::new();
    }

    // Second pass: for each action item, walk backwards to mark ancestors (O(n) total)
    let mut needed: Vec<bool> = vec![false; items.len()];
    for &idx in &action_indices {
        needed[idx] = true;
        let mut current_depth = items[idx].depth;
        for ancestor_idx in (0..idx).rev() {
            if items[ancestor_idx].depth < current_depth {
                needed[ancestor_idx] = true;
                current_depth = items[ancestor_idx].depth;
                if current_depth == 0 {
                    break;
                }
            }
        }
    }

    // Third pass: keep only needed items
    items
        .iter()
        .enumerate()
        .filter(|(i, _)| needed[*i])
        .map(|(_, item)| item.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Critique, CritiqueStatus, Milestone, Priority, Problem, ProblemStatus, Solution,
    };
    use crate::tui::next_actions::{Category, EntityType, NextAction};
    use std::collections::HashSet;

    // --- Helper functions ---

    fn make_problem(id: &str, title: &str) -> Problem {
        Problem::new(id.to_string(), title.to_string())
    }

    fn make_problem_in_milestone(id: &str, title: &str, milestone_id: &str) -> Problem {
        let mut p = Problem::new(id.to_string(), title.to_string());
        p.milestone_id = Some(milestone_id.to_string());
        p
    }

    fn make_solution(id: &str, title: &str, problem_id: &str) -> Solution {
        Solution::new(id.to_string(), title.to_string(), problem_id.to_string())
    }

    fn make_critique(id: &str, title: &str, solution_id: &str) -> Critique {
        Critique::new(id.to_string(), title.to_string(), solution_id.to_string())
    }

    fn make_milestone(id: &str, title: &str) -> Milestone {
        Milestone::new(id.to_string(), title.to_string())
    }

    fn make_next_action(entity_id: &str, category: Category) -> NextAction {
        NextAction {
            category,
            entity_type: EntityType::Solution,
            entity_id: entity_id.to_string(),
            title: "Test action".to_string(),
            summary: "Test summary".to_string(),
            priority: Priority::Medium,
            details: vec![],
        }
    }

    fn expanded_set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    // --- build_flat_tree tests ---

    #[test]
    fn test_build_flat_tree_empty_inputs() {
        let tree = build_flat_tree(&[], &[], &[], &[], &HashSet::new());
        // Should contain exactly one item: the collapsed ProjectRoot node
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].node.id(), "project-root");
        assert!(!tree[0].has_children);
    }

    #[test]
    fn test_build_flat_tree_single_milestone_with_problems() {
        let milestones = vec![make_milestone("M-1", "v1.0 Release")];
        let problems = vec![
            make_problem_in_milestone("P-1", "Auth bug", "M-1"),
            make_problem_in_milestone("P-2", "Performance issue", "M-1"),
        ];
        let expanded = expanded_set(&["project-root", "M-1"]);

        let tree = build_flat_tree(&milestones, &problems, &[], &[], &expanded);

        // ProjectRoot + Milestone + 2 problems + Backlog = 5 items
        assert_eq!(tree.len(), 5);

        // First item: project root
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[0].depth, 0);

        // Second item: milestone
        assert_eq!(tree[1].node.id(), "M-1");
        assert_eq!(tree[1].depth, 1);
        assert!(tree[1].has_children);
        assert!(tree[1].node.is_expanded());

        // Third and fourth: problems under milestone
        assert_eq!(tree[2].node.id(), "P-1");
        assert_eq!(tree[2].depth, 2);
        assert_eq!(tree[3].node.id(), "P-2");
        assert_eq!(tree[3].depth, 2);

        // Last: backlog
        assert_eq!(tree[4].node.id(), "backlog");
        assert!(!tree[4].has_children);
    }

    #[test]
    fn test_build_flat_tree_backlog_problems() {
        // Problems without a milestone go into backlog
        let problems = vec![
            make_problem("P-1", "Backlog issue 1"),
            make_problem("P-2", "Backlog issue 2"),
        ];
        let expanded = expanded_set(&["project-root", "backlog"]);

        let tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        // ProjectRoot + Backlog + 2 problems = 4 items
        assert_eq!(tree.len(), 4);
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[1].node.id(), "backlog");
        assert!(tree[1].has_children);
        assert!(tree[1].node.is_expanded());
        assert_eq!(tree[2].node.id(), "P-1");
        assert_eq!(tree[2].depth, 2);
        assert_eq!(tree[3].node.id(), "P-2");
        assert_eq!(tree[3].depth, 2);
    }

    #[test]
    fn test_build_flat_tree_collapsed_milestone_hides_children() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![make_problem_in_milestone("P-1", "Bug", "M-1")];
        // project-root expanded, but M-1 is NOT in expanded set
        let expanded = expanded_set(&["project-root"]);

        let tree = build_flat_tree(&milestones, &problems, &[], &[], &expanded);

        // ProjectRoot + Milestone + Backlog = 3 items (problem is hidden because milestone collapsed)
        assert_eq!(tree.len(), 3);
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[1].node.id(), "M-1");
        assert!(tree[1].has_children);
        assert!(!tree[1].node.is_expanded());
        assert_eq!(tree[2].node.id(), "backlog");
    }

    #[test]
    fn test_build_flat_tree_collapsed_backlog_hides_children() {
        let problems = vec![make_problem("P-1", "Bug")];
        // project-root expanded, but backlog is NOT in expanded set
        let expanded = expanded_set(&["project-root"]);

        let tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        // ProjectRoot + Backlog = 2 items, problem is hidden
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[1].node.id(), "backlog");
        assert!(tree[1].has_children);
        assert!(!tree[1].node.is_expanded());
    }

    #[test]
    fn test_build_flat_tree_expanded_problem_shows_solutions() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![
            make_solution("S-1", "Fix A", "P-1"),
            make_solution("S-2", "Fix B", "P-1"),
        ];
        let expanded = expanded_set(&["project-root", "backlog", "P-1"]);

        let tree = build_flat_tree(&[], &problems, &solutions, &[], &expanded);

        // ProjectRoot + Backlog + Problem + 2 Solutions = 5 items
        assert_eq!(tree.len(), 5);
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[1].node.id(), "backlog");
        assert_eq!(tree[2].node.id(), "P-1");
        assert!(tree[2].node.is_expanded());
        assert_eq!(tree[3].node.id(), "S-1");
        assert_eq!(tree[3].depth, 3);
        assert_eq!(tree[4].node.id(), "S-2");
        assert_eq!(tree[4].depth, 3);
    }

    #[test]
    fn test_build_flat_tree_collapsed_problem_hides_solutions() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![make_solution("S-1", "Fix A", "P-1")];
        // Backlog expanded but problem is not
        let expanded = expanded_set(&["project-root", "backlog"]);

        let tree = build_flat_tree(&[], &problems, &solutions, &[], &expanded);

        // ProjectRoot + Backlog + Problem = 3 items (solution hidden)
        assert_eq!(tree.len(), 3);
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[1].node.id(), "backlog");
        assert_eq!(tree[2].node.id(), "P-1");
        assert!(tree[2].has_children);
        assert!(!tree[2].node.is_expanded());
    }

    #[test]
    fn test_build_flat_tree_expanded_solution_shows_critiques() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let critiques = vec![
            make_critique("C-1", "Flaw 1", "S-1"),
            make_critique("C-2", "Flaw 2", "S-1"),
        ];
        let expanded = expanded_set(&["project-root", "backlog", "P-1", "S-1"]);

        let tree = build_flat_tree(&[], &problems, &solutions, &critiques, &expanded);

        // ProjectRoot + Backlog + Problem + Solution + 2 Critiques = 6 items
        assert_eq!(tree.len(), 6);
        assert_eq!(tree[4].node.id(), "C-1");
        assert_eq!(tree[4].depth, 4);
        assert!(!tree[4].has_children);
        assert_eq!(tree[5].node.id(), "C-2");
        assert_eq!(tree[5].depth, 4);
    }

    #[test]
    fn test_build_flat_tree_full_hierarchy_depths() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![make_problem_in_milestone("P-1", "Bug", "M-1")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let critiques = vec![make_critique("C-1", "Flaw", "S-1")];
        let expanded = expanded_set(&["project-root", "M-1", "P-1", "S-1"]);

        let tree = build_flat_tree(&milestones, &problems, &solutions, &critiques, &expanded);

        // ProjectRoot (depth 0) -> M-1 (depth 1) -> P-1 (depth 2) -> S-1 (depth 3) -> C-1 (depth 4) + Backlog (depth 1)
        assert_eq!(tree.len(), 6);
        assert_eq!(tree[0].depth, 0); // project-root
        assert_eq!(tree[1].depth, 1); // M-1
        assert_eq!(tree[2].depth, 2); // P-1
        assert_eq!(tree[3].depth, 3); // S-1
        assert_eq!(tree[4].depth, 4); // C-1
        assert_eq!(tree[5].depth, 1); // backlog
    }

    #[test]
    fn test_build_flat_tree_mixed_milestone_and_backlog() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![
            make_problem_in_milestone("P-1", "Milestone bug", "M-1"),
            make_problem("P-2", "Backlog bug"),
        ];
        let expanded = expanded_set(&["project-root", "M-1", "backlog"]);

        let tree = build_flat_tree(&milestones, &problems, &[], &[], &expanded);

        // ProjectRoot + M-1 + P-1 + Backlog + P-2 = 5
        assert_eq!(tree.len(), 5);
        assert_eq!(tree[0].node.id(), "project-root");
        assert_eq!(tree[1].node.id(), "M-1");
        assert_eq!(tree[2].node.id(), "P-1");
        assert_eq!(tree[3].node.id(), "backlog");
        assert_eq!(tree[4].node.id(), "P-2");
    }

    // --- annotate_tree_with_actions tests ---

    #[test]
    fn test_annotate_tree_no_actions() {
        let problems = vec![make_problem("P-1", "Bug")];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let mut tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        annotate_tree_with_actions(&mut tree, &[]);

        // No symbols assigned
        for item in &tree {
            assert!(item.action_symbol.is_none());
        }
    }

    #[test]
    fn test_annotate_tree_blocked_symbol() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let expanded = expanded_set(&["project-root", "backlog", "P-1"]);
        let mut tree = build_flat_tree(&[], &problems, &solutions, &[], &expanded);

        let actions = vec![make_next_action("S-1", Category::Blocked)];
        annotate_tree_with_actions(&mut tree, &actions);

        let s1_item = tree.iter().find(|i| i.node.id() == "S-1").unwrap();
        assert_eq!(s1_item.action_symbol, Some("X".to_string()));
    }

    #[test]
    fn test_annotate_tree_ready_symbol() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let expanded = expanded_set(&["project-root", "backlog", "P-1"]);
        let mut tree = build_flat_tree(&[], &problems, &solutions, &[], &expanded);

        let actions = vec![make_next_action("S-1", Category::Ready)];
        annotate_tree_with_actions(&mut tree, &actions);

        let s1_item = tree.iter().find(|i| i.node.id() == "S-1").unwrap();
        assert_eq!(s1_item.action_symbol, Some("!".to_string()));
    }

    #[test]
    fn test_annotate_tree_review_symbol() {
        let problems = vec![make_problem("P-1", "Bug")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let critiques = vec![make_critique("C-1", "Flaw", "S-1")];
        let expanded = expanded_set(&["project-root", "backlog", "P-1", "S-1"]);
        let mut tree = build_flat_tree(&[], &problems, &solutions, &critiques, &expanded);

        let actions = vec![make_next_action("C-1", Category::Review)];
        annotate_tree_with_actions(&mut tree, &actions);

        let c1_item = tree.iter().find(|i| i.node.id() == "C-1").unwrap();
        assert_eq!(c1_item.action_symbol, Some("?".to_string()));
    }

    #[test]
    fn test_annotate_tree_todo_symbol() {
        let problems = vec![make_problem("P-1", "Bug")];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let mut tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        let actions = vec![make_next_action("P-1", Category::Todo)];
        annotate_tree_with_actions(&mut tree, &actions);

        let p1_item = tree.iter().find(|i| i.node.id() == "P-1").unwrap();
        assert_eq!(p1_item.action_symbol, Some("*".to_string()));
    }

    #[test]
    fn test_annotate_tree_waiting_symbol() {
        let problems = vec![make_problem("P-1", "Bug")];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let mut tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        let actions = vec![make_next_action("P-1", Category::Waiting)];
        annotate_tree_with_actions(&mut tree, &actions);

        let p1_item = tree.iter().find(|i| i.node.id() == "P-1").unwrap();
        assert_eq!(p1_item.action_symbol, Some("~".to_string()));
    }

    #[test]
    fn test_annotate_tree_multiple_actions() {
        let problems = vec![make_problem("P-1", "Bug 1"), make_problem("P-2", "Bug 2")];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let mut tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        let actions = vec![
            make_next_action("P-1", Category::Todo),
            make_next_action("P-2", Category::Blocked),
        ];
        annotate_tree_with_actions(&mut tree, &actions);

        let p1_item = tree.iter().find(|i| i.node.id() == "P-1").unwrap();
        assert_eq!(p1_item.action_symbol, Some("*".to_string()));

        let p2_item = tree.iter().find(|i| i.node.id() == "P-2").unwrap();
        assert_eq!(p2_item.action_symbol, Some("X".to_string()));
    }

    #[test]
    fn test_annotate_tree_unmatched_nodes_no_symbol() {
        let problems = vec![make_problem("P-1", "Bug 1"), make_problem("P-2", "Bug 2")];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let mut tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        // Only P-1 has an action
        let actions = vec![make_next_action("P-1", Category::Todo)];
        annotate_tree_with_actions(&mut tree, &actions);

        let p2_item = tree.iter().find(|i| i.node.id() == "P-2").unwrap();
        assert!(p2_item.action_symbol.is_none());
    }

    // --- filter_tree_to_actions tests ---

    #[test]
    fn test_filter_tree_empty_when_no_actions() {
        let problems = vec![make_problem("P-1", "Bug")];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        // No annotations -> all action_symbol is None
        let filtered = filter_tree_to_actions(&tree);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_tree_retains_action_items_and_ancestors() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![make_problem_in_milestone("P-1", "Bug", "M-1")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let expanded = expanded_set(&["project-root", "M-1", "P-1"]);
        let mut tree = build_flat_tree(&milestones, &problems, &solutions, &[], &expanded);

        // Annotate S-1 as blocked
        let actions = vec![make_next_action("S-1", Category::Blocked)];
        annotate_tree_with_actions(&mut tree, &actions);

        let filtered = filter_tree_to_actions(&tree);

        // Should include: project-root (ancestor), M-1 (ancestor), P-1 (ancestor), S-1 (action item)
        let ids: Vec<&str> = filtered.iter().map(|i| i.node.id()).collect();
        assert!(
            ids.contains(&"project-root"),
            "ProjectRoot ancestor should be retained"
        );
        assert!(
            ids.contains(&"M-1"),
            "Milestone ancestor should be retained"
        );
        assert!(ids.contains(&"P-1"), "Problem ancestor should be retained");
        assert!(ids.contains(&"S-1"), "Action item should be retained");
        // Backlog should NOT be included (no action items under it)
        assert!(!ids.contains(&"backlog"));
    }

    #[test]
    fn test_filter_tree_excludes_non_action_branches() {
        let problems = vec![
            make_problem("P-1", "Bug with action"),
            make_problem("P-2", "Bug without action"),
        ];
        let expanded = expanded_set(&["project-root", "backlog"]);
        let mut tree = build_flat_tree(&[], &problems, &[], &[], &expanded);

        // Only P-1 has an action
        let actions = vec![make_next_action("P-1", Category::Todo)];
        annotate_tree_with_actions(&mut tree, &actions);

        let filtered = filter_tree_to_actions(&tree);

        let ids: Vec<&str> = filtered.iter().map(|i| i.node.id()).collect();
        assert!(
            ids.contains(&"project-root"),
            "ProjectRoot is ancestor of P-1"
        );
        assert!(ids.contains(&"P-1"));
        assert!(ids.contains(&"backlog"), "Backlog is ancestor of P-1");
        assert!(
            !ids.contains(&"P-2"),
            "P-2 has no action and should be excluded"
        );
    }

    #[test]
    fn test_filter_tree_multiple_actions_in_different_branches() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![
            make_problem_in_milestone("P-1", "Milestone bug", "M-1"),
            make_problem("P-2", "Backlog bug"),
        ];
        let expanded = expanded_set(&["project-root", "M-1", "backlog"]);
        let mut tree = build_flat_tree(&milestones, &problems, &[], &[], &expanded);

        let actions = vec![
            make_next_action("P-1", Category::Blocked),
            make_next_action("P-2", Category::Todo),
        ];
        annotate_tree_with_actions(&mut tree, &actions);

        let filtered = filter_tree_to_actions(&tree);

        let ids: Vec<&str> = filtered.iter().map(|i| i.node.id()).collect();
        assert!(ids.contains(&"project-root"));
        assert!(ids.contains(&"M-1"));
        assert!(ids.contains(&"P-1"));
        assert!(ids.contains(&"backlog"));
        assert!(ids.contains(&"P-2"));
    }

    #[test]
    fn test_filter_tree_preserves_depth() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![make_problem_in_milestone("P-1", "Bug", "M-1")];
        let expanded = expanded_set(&["project-root", "M-1"]);
        let mut tree = build_flat_tree(&milestones, &problems, &[], &[], &expanded);

        let actions = vec![make_next_action("P-1", Category::Todo)];
        annotate_tree_with_actions(&mut tree, &actions);

        let filtered = filter_tree_to_actions(&tree);

        let root = filtered
            .iter()
            .find(|i| i.node.id() == "project-root")
            .unwrap();
        assert_eq!(root.depth, 0);

        let m1 = filtered.iter().find(|i| i.node.id() == "M-1").unwrap();
        assert_eq!(m1.depth, 1);

        let p1 = filtered.iter().find(|i| i.node.id() == "P-1").unwrap();
        assert_eq!(p1.depth, 2);
    }

    #[test]
    fn test_filter_tree_deep_action_includes_all_ancestors() {
        let milestones = vec![make_milestone("M-1", "v1.0")];
        let problems = vec![make_problem_in_milestone("P-1", "Bug", "M-1")];
        let solutions = vec![make_solution("S-1", "Fix", "P-1")];
        let critiques = vec![make_critique("C-1", "Flaw", "S-1")];
        let expanded = expanded_set(&["project-root", "M-1", "P-1", "S-1"]);
        let mut tree = build_flat_tree(&milestones, &problems, &solutions, &critiques, &expanded);

        // Action on the deepest node (critique)
        let actions = vec![make_next_action("C-1", Category::Review)];
        annotate_tree_with_actions(&mut tree, &actions);

        let filtered = filter_tree_to_actions(&tree);

        let ids: Vec<&str> = filtered.iter().map(|i| i.node.id()).collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(ids[0], "project-root");
        assert_eq!(ids[1], "M-1");
        assert_eq!(ids[2], "P-1");
        assert_eq!(ids[3], "S-1");
        assert_eq!(ids[4], "C-1");
    }

    #[test]
    fn test_filter_tree_on_empty_tree() {
        let tree: Vec<FlatTreeItem> = vec![];
        let filtered = filter_tree_to_actions(&tree);
        assert!(filtered.is_empty());
    }

    // --- TreeNode method tests ---

    #[test]
    fn test_tree_node_critique_cannot_expand() {
        let node = TreeNode::Critique {
            id: "C-1".to_string(),
            title: "Flaw".to_string(),
            status: CritiqueStatus::Open,
            severity: "medium".to_string(),
        };
        assert!(!node.can_expand());
        assert!(!node.is_expanded());
    }

    #[test]
    fn test_tree_node_set_expanded() {
        let mut node = TreeNode::Problem {
            id: "P-1".to_string(),
            title: "Bug".to_string(),
            status: ProblemStatus::Open,
            priority: Priority::Medium,
            assignee: None,
            expanded: false,
        };
        assert!(!node.is_expanded());
        node.set_expanded(true);
        assert!(node.is_expanded());
        node.set_expanded(false);
        assert!(!node.is_expanded());
    }

    #[test]
    fn test_tree_node_critique_set_expanded_noop() {
        let mut node = TreeNode::Critique {
            id: "C-1".to_string(),
            title: "Flaw".to_string(),
            status: CritiqueStatus::Open,
            severity: "medium".to_string(),
        };
        node.set_expanded(true);
        assert!(!node.is_expanded()); // Still false, critiques don't expand
    }
}
