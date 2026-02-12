use crate::models::{
    Critique, CritiqueStatus, Milestone, MilestoneStatus, Problem, ProblemStatus, Solution,
    SolutionStatus,
};

#[derive(Debug, Clone)]
pub enum TreeNode {
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
        expanded: bool,
    },
    Solution {
        id: String,
        title: String,
        status: SolutionStatus,
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
            TreeNode::Milestone { id, .. } => id,
            TreeNode::Backlog { .. } => "backlog",
            TreeNode::Problem { id, .. } => id,
            TreeNode::Solution { id, .. } => id,
            TreeNode::Critique { id, .. } => id,
        }
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            TreeNode::Milestone { expanded, .. } => *expanded,
            TreeNode::Backlog { expanded } => *expanded,
            TreeNode::Problem { expanded, .. } => *expanded,
            TreeNode::Solution { expanded, .. } => *expanded,
            TreeNode::Critique { .. } => false, // Critiques don't expand
        }
    }

    pub fn set_expanded(&mut self, value: bool) {
        match self {
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
            depth: 0,
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
                1,
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
        depth: 0,
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
            1,
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
