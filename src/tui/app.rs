use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, Milestone, Problem, Solution};
use crate::storage::MetadataStore;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    NextActions,
    ProjectTree,
}

pub struct App {
    pub should_quit: bool,
    pub focused_pane: FocusedPane,
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,
    pub next_actions: Vec<super::NextAction>,
    pub next_actions_index: usize,
    pub tree_items: Vec<super::FlatTreeItem>,
    pub expanded_nodes: HashSet<String>,
    pub tree_index: usize,
    pub detail_scroll: u16,
    pub selected_detail: super::DetailContent,
    pub flash_message: Option<(String, Instant)>,
    store: MetadataStore,
}

impl App {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        let milestones = store.list_milestones()?;
        let problems = store.list_problems()?;
        let solutions = store.list_solutions()?;
        let critiques = store.list_critiques()?;

        let user = store.jj_client.user_identity().unwrap_or_default();
        let next_actions = super::build_next_actions(&problems, &solutions, &critiques, &user);

        // Expand first milestone and backlog by default
        let mut expanded_nodes = HashSet::new();
        if let Some(m) = milestones.first() {
            expanded_nodes.insert(m.id.clone());
        }
        expanded_nodes.insert("backlog".to_string());

        let tree_items = super::build_flat_tree(
            &milestones, &problems, &solutions, &critiques, &expanded_nodes
        );

        let mut app = Self {
            should_quit: false,
            focused_pane: FocusedPane::NextActions,
            milestones,
            problems,
            solutions,
            critiques,
            next_actions,
            next_actions_index: 0,
            tree_items,
            expanded_nodes,
            tree_index: 0,
            detail_scroll: 0,
            selected_detail: super::DetailContent::None,
            flash_message: None,
            store,
        };
        app.update_selected_detail();
        Ok(app)
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
            self.clear_expired_flash();
            terminal.draw(|f| super::ui::draw(f, self))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn show_flash(&mut self, message: &str) {
        self.flash_message = Some((message.to_string(), Instant::now()));
    }

    fn clear_expired_flash(&mut self) {
        if let Some((_, time)) = &self.flash_message {
            if time.elapsed() > Duration::from_secs(2) {
                self.flash_message = None;
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Left => self.collapse_or_parent(),
            KeyCode::Right => self.expand_or_child(),
            KeyCode::Char('j') => self.scroll_detail_down(),
            KeyCode::Char('k') => self.scroll_detail_up(),
            KeyCode::Char(' ') => self.page_detail_down(),
            KeyCode::Char('a') => self.handle_action_a()?,
            KeyCode::Char('r') => self.handle_action_r()?,
            KeyCode::Char('d') => self.handle_action_d()?,
            _ => {}
        }
        Ok(())
    }

    fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::NextActions => FocusedPane::ProjectTree,
            FocusedPane::ProjectTree => FocusedPane::NextActions,
        };
        self.update_selected_detail();
    }

    fn navigate_up(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index > 0 {
                    self.next_actions_index -= 1;
                    self.sync_tree_to_selection();
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index > 0 {
                    self.tree_index -= 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index < self.next_actions.len().saturating_sub(1) {
                    self.next_actions_index += 1;
                    self.sync_tree_to_selection();
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index < self.tree_items.len().saturating_sub(1) {
                    self.tree_index += 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn sync_tree_to_selection(&mut self) {
        if self.focused_pane != FocusedPane::NextActions {
            return;
        }

        let target_id = match self.next_actions.get(self.next_actions_index) {
            Some(action) => action.entity_id.clone(),
            None => return,
        };

        // Find which nodes need to be expanded to show this item
        self.expand_to_reveal(&target_id);
        self.rebuild_tree();

        // Find the item in the tree
        for (i, item) in self.tree_items.iter().enumerate() {
            if item.node.id() == target_id {
                self.tree_index = i;
                break;
            }
        }
    }

    fn expand_to_reveal(&mut self, target_id: &str) {
        // For a solution, we need its problem expanded, and that problem's milestone expanded
        if let Some(solution) = self.solutions.iter().find(|s| s.id == target_id) {
            self.expanded_nodes.insert(solution.problem_id.clone());

            if let Some(problem) = self.problems.iter().find(|p| p.id == solution.problem_id) {
                if let Some(milestone_id) = &problem.milestone_id {
                    self.expanded_nodes.insert(milestone_id.clone());
                } else {
                    self.expanded_nodes.insert("backlog".to_string());
                }
            }
        }

        // For a problem, we need its milestone expanded
        if let Some(problem) = self.problems.iter().find(|p| p.id == target_id) {
            if let Some(milestone_id) = &problem.milestone_id {
                self.expanded_nodes.insert(milestone_id.clone());
            } else {
                self.expanded_nodes.insert("backlog".to_string());
            }
        }

        // For a critique, we need its solution and problem expanded
        if let Some(critique) = self.critiques.iter().find(|c| c.id == target_id) {
            self.expanded_nodes.insert(critique.solution_id.clone());

            if let Some(solution) = self.solutions.iter().find(|s| s.id == critique.solution_id) {
                self.expanded_nodes.insert(solution.problem_id.clone());

                if let Some(problem) = self.problems.iter().find(|p| p.id == solution.problem_id) {
                    if let Some(milestone_id) = &problem.milestone_id {
                        self.expanded_nodes.insert(milestone_id.clone());
                    } else {
                        self.expanded_nodes.insert("backlog".to_string());
                    }
                }
            }
        }
    }

    fn collapse_or_parent(&mut self) {
        if self.focused_pane != FocusedPane::ProjectTree {
            return;
        }

        if let Some(item) = self.tree_items.get(self.tree_index) {
            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Collapse current node
                self.expanded_nodes.remove(&node_id);
                self.rebuild_tree();
            } else if item.depth > 0 {
                // Move to parent
                for i in (0..self.tree_index).rev() {
                    if self.tree_items[i].depth < item.depth {
                        self.tree_index = i;
                        break;
                    }
                }
            }
        }
    }

    fn expand_or_child(&mut self) {
        if self.focused_pane != FocusedPane::ProjectTree {
            return;
        }

        if let Some(item) = self.tree_items.get(self.tree_index) {
            if !item.has_children {
                return;
            }

            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Move to first child
                if self.tree_index + 1 < self.tree_items.len() {
                    self.tree_index += 1;
                }
            } else {
                // Expand
                self.expanded_nodes.insert(node_id);
                self.rebuild_tree();
            }
        }
    }

    fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(1);
    }

    fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    fn page_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(10);
    }

    pub fn rebuild_tree(&mut self) {
        self.tree_items = super::build_flat_tree(
            &self.milestones,
            &self.problems,
            &self.solutions,
            &self.critiques,
            &self.expanded_nodes,
        );
    }

    pub fn context_hints(&self) -> String {
        use super::tree::TreeNode;

        match self.focused_pane {
            FocusedPane::NextActions => {
                if let Some(action) = self.next_actions.get(self.next_actions_index) {
                    match action.entity_type {
                        super::next_actions::EntityType::Problem => {
                            format!("{}: [n]ew solution [s]olve [d]issolve [e]dit", action.entity_id)
                        }
                        super::next_actions::EntityType::Solution => {
                            format!("{}: [a]ccept [r]efute [n]ew critique [e]dit", action.entity_id)
                        }
                        super::next_actions::EntityType::Critique => {
                            format!("{}: [a]ddress [d]ismiss [e]dit", action.entity_id)
                        }
                    }
                } else {
                    "No selection".to_string()
                }
            }
            FocusedPane::ProjectTree => {
                if let Some(item) = self.tree_items.get(self.tree_index) {
                    match &item.node {
                        TreeNode::Milestone { id, .. } => {
                            format!("{}: [e]dit", id)
                        }
                        TreeNode::Backlog { .. } => {
                            "[p]roblem new".to_string()
                        }
                        TreeNode::Problem { id, .. } => {
                            format!("{}: [n]ew solution [s]olve [d]issolve [e]dit", id)
                        }
                        TreeNode::Solution { id, .. } => {
                            format!("{}: [a]ccept [r]efute [n]ew critique [e]dit", id)
                        }
                        TreeNode::Critique { id, .. } => {
                            format!("{}: [a]ddress [d]ismiss [e]dit", id)
                        }
                    }
                } else {
                    "No selection".to_string()
                }
            }
        }
    }

    pub fn update_selected_detail(&mut self) {
        use super::tree::TreeNode;

        // Check focused pane and get relevant selection
        match self.focused_pane {
            FocusedPane::NextActions => {
                if let Some(action) = self.next_actions.get(self.next_actions_index) {
                    self.selected_detail = match action.entity_type {
                        super::next_actions::EntityType::Problem => {
                            self.problems.iter()
                                .find(|p| p.id == action.entity_id)
                                .cloned()
                                .map(super::DetailContent::Problem)
                                .unwrap_or(super::DetailContent::None)
                        }
                        super::next_actions::EntityType::Solution => {
                            self.solutions.iter()
                                .find(|s| s.id == action.entity_id)
                                .cloned()
                                .map(super::DetailContent::Solution)
                                .unwrap_or(super::DetailContent::None)
                        }
                        super::next_actions::EntityType::Critique => {
                            self.critiques.iter()
                                .find(|c| c.id == action.entity_id)
                                .cloned()
                                .map(super::DetailContent::Critique)
                                .unwrap_or(super::DetailContent::None)
                        }
                    };
                }
            }
            FocusedPane::ProjectTree => {
                if let Some(item) = self.tree_items.get(self.tree_index) {
                    self.selected_detail = match &item.node {
                        TreeNode::Milestone { id, .. } => {
                            self.milestones.iter()
                                .find(|m| m.id == *id)
                                .cloned()
                                .map(super::DetailContent::Milestone)
                                .unwrap_or(super::DetailContent::None)
                        }
                        TreeNode::Backlog { .. } => super::DetailContent::None,
                        TreeNode::Problem { id, .. } => {
                            self.problems.iter()
                                .find(|p| p.id == *id)
                                .cloned()
                                .map(super::DetailContent::Problem)
                                .unwrap_or(super::DetailContent::None)
                        }
                        TreeNode::Solution { id, .. } => {
                            self.solutions.iter()
                                .find(|s| s.id == *id)
                                .cloned()
                                .map(super::DetailContent::Solution)
                                .unwrap_or(super::DetailContent::None)
                        }
                        TreeNode::Critique { id, .. } => {
                            self.critiques.iter()
                                .find(|c| c.id == *id)
                                .cloned()
                                .map(super::DetailContent::Critique)
                                .unwrap_or(super::DetailContent::None)
                        }
                    };
                }
            }
        }
        self.detail_scroll = 0; // Reset scroll on new selection
    }

    fn get_selected_entity(&self) -> Option<(String, super::next_actions::EntityType)> {
        use super::tree::TreeNode;

        match self.focused_pane {
            FocusedPane::NextActions => {
                self.next_actions.get(self.next_actions_index)
                    .map(|a| (a.entity_id.clone(), a.entity_type))
            }
            FocusedPane::ProjectTree => {
                self.tree_items.get(self.tree_index).and_then(|item| {
                    match &item.node {
                        TreeNode::Problem { id, .. } => Some((id.clone(), super::next_actions::EntityType::Problem)),
                        TreeNode::Solution { id, .. } => Some((id.clone(), super::next_actions::EntityType::Solution)),
                        TreeNode::Critique { id, .. } => Some((id.clone(), super::next_actions::EntityType::Critique)),
                        _ => None,
                    }
                })
            }
        }
    }

    fn handle_action_a(&mut self) -> Result<()> {
        use super::next_actions::EntityType;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Solution => self.accept_solution(&id)?,
                EntityType::Critique => self.address_critique(&id)?,
                EntityType::Problem => {} // No 'a' action for problems
            }
        }
        Ok(())
    }

    fn handle_action_r(&mut self) -> Result<()> {
        use super::next_actions::EntityType;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Solution {
                self.refute_solution(&id)?;
            }
        }
        Ok(())
    }

    fn handle_action_d(&mut self) -> Result<()> {
        use super::next_actions::EntityType;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Critique {
                self.dismiss_critique(&id)?;
            }
            // For problems, 'd' would be dissolve - add later with input
        }
        Ok(())
    }

    fn accept_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        match self.store.with_metadata(&format!("Accept solution {}", solution_id), || {
            let mut solution = self.store.load_solution(solution_id)?;
            solution.accept();
            self.store.save_solution(&solution)?;
            Ok(())
        }) {
            Ok(_) => {
                self.show_flash(&format!("{} accepted", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn refute_solution(&mut self, solution_id: &str) -> Result<()> {
        let id = solution_id.to_string();
        match self.store.with_metadata(&format!("Refute solution {}", solution_id), || {
            let mut solution = self.store.load_solution(solution_id)?;
            solution.refute();
            self.store.save_solution(&solution)?;
            Ok(())
        }) {
            Ok(_) => {
                self.show_flash(&format!("{} refuted", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn address_critique(&mut self, critique_id: &str) -> Result<()> {
        let id = critique_id.to_string();
        match self.store.with_metadata(&format!("Address critique {}", critique_id), || {
            let mut critique = self.store.load_critique(critique_id)?;
            critique.address();
            self.store.save_critique(&critique)?;
            Ok(())
        }) {
            Ok(_) => {
                self.show_flash(&format!("{} addressed", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn dismiss_critique(&mut self, critique_id: &str) -> Result<()> {
        let id = critique_id.to_string();
        match self.store.with_metadata(&format!("Dismiss critique {}", critique_id), || {
            let mut critique = self.store.load_critique(critique_id)?;
            critique.dismiss();
            self.store.save_critique(&critique)?;
            Ok(())
        }) {
            Ok(_) => {
                self.show_flash(&format!("{} dismissed", id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn refresh_data(&mut self) -> Result<()> {
        self.milestones = self.store.list_milestones()?;
        self.problems = self.store.list_problems()?;
        self.solutions = self.store.list_solutions()?;
        self.critiques = self.store.list_critiques()?;

        let user = self.store.jj_client.user_identity().unwrap_or_default();
        self.next_actions = super::build_next_actions(&self.problems, &self.solutions, &self.critiques, &user);
        self.rebuild_tree();
        self.update_selected_detail();

        Ok(())
    }
}
