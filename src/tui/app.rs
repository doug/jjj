use crate::db::search::SimilarityResult;
use crate::db::Database;
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

/// Raw data from storage - single source of truth
pub struct ProjectData {
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,
}

impl ProjectData {
    pub fn load(store: &MetadataStore) -> Result<Self> {
        Ok(Self {
            milestones: store.list_milestones()?,
            problems: store.list_problems()?,
            solutions: store.list_solutions()?,
            critiques: store.list_critiques()?,
        })
    }
}

/// UI navigation and display state
pub struct UiState {
    pub focused_pane: FocusedPane,
    pub next_actions_index: usize,
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
    pub show_related: bool,
    pub related_items: Vec<SimilarityResult>,
    pub related_selected: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

impl UiState {
    pub fn new() -> Self {
        let mut expanded_nodes = HashSet::new();
        expanded_nodes.insert("backlog".to_string());
        Self {
            focused_pane: FocusedPane::NextActions,
            next_actions_index: 0,
            tree_index: 0,
            expanded_nodes,
            detail_scroll: 0,
            flash_message: None,
            show_related: true,
            related_items: Vec::new(),
            related_selected: 0,
        }
    }
}

/// Cached derived data for rendering
pub(crate) struct RenderCache {
    pub(crate) next_actions: Vec<super::NextAction>,
    pub(crate) tree_items: Vec<super::FlatTreeItem>,
    pub(crate) selected_detail: super::DetailContent,
}

pub struct App {
    pub should_quit: bool,
    pub data: ProjectData,
    pub ui: UiState,
    pub(crate) cache: RenderCache,
    #[allow(dead_code)] // Prepared for future cache invalidation
    dirty: bool,
    store: MetadataStore,
    db: Option<Database>,
}

impl App {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        let data = ProjectData::load(&store)?;
        let mut ui = UiState::new();

        // Also expand first milestone by default
        if let Some(m) = data.milestones.first() {
            ui.expanded_nodes.insert(m.id.clone());
        }

        let user = store.jj_client.user_identity().unwrap_or_default();
        let next_actions =
            super::build_next_actions(&data.problems, &data.solutions, &data.critiques, &user);
        let tree_items = super::build_flat_tree(
            &data.milestones,
            &data.problems,
            &data.solutions,
            &data.critiques,
            &ui.expanded_nodes,
        );

        let cache = RenderCache {
            next_actions,
            tree_items,
            selected_detail: super::DetailContent::None,
        };

        // Try to open the database for related items
        let db_path = store.jj_client.repo_root().join(".jj").join("jjj.db");
        let db = if db_path.exists() {
            Database::open(&db_path).ok()
        } else {
            None
        };

        let mut app = Self {
            should_quit: false,
            data,
            ui,
            cache,
            dirty: false,
            store,
            db,
        };
        app.update_selected_detail();
        app.load_related_for_selected();
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
        self.ui.flash_message = Some((message.to_string(), Instant::now()));
    }

    fn clear_expired_flash(&mut self) {
        if let Some((_, time)) = &self.ui.flash_message {
            if time.elapsed() > Duration::from_secs(2) {
                self.ui.flash_message = None;
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
            KeyCode::Char('R') => self.toggle_related_panel(),
            _ => {}
        }
        Ok(())
    }

    fn toggle_focus(&mut self) {
        self.ui.focused_pane = match self.ui.focused_pane {
            FocusedPane::NextActions => FocusedPane::ProjectTree,
            FocusedPane::ProjectTree => FocusedPane::NextActions,
        };
        self.update_selected_detail();
    }

    fn navigate_up(&mut self) {
        match self.ui.focused_pane {
            FocusedPane::NextActions => {
                if self.ui.next_actions_index > 0 {
                    self.ui.next_actions_index -= 1;
                    self.sync_tree_to_selection();
                }
            }
            FocusedPane::ProjectTree => {
                if self.ui.tree_index > 0 {
                    self.ui.tree_index -= 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn navigate_down(&mut self) {
        match self.ui.focused_pane {
            FocusedPane::NextActions => {
                if self.ui.next_actions_index < self.cache.next_actions.len().saturating_sub(1) {
                    self.ui.next_actions_index += 1;
                    self.sync_tree_to_selection();
                }
            }
            FocusedPane::ProjectTree => {
                if self.ui.tree_index < self.cache.tree_items.len().saturating_sub(1) {
                    self.ui.tree_index += 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn sync_tree_to_selection(&mut self) {
        if self.ui.focused_pane != FocusedPane::NextActions {
            return;
        }

        let target_id = match self.cache.next_actions.get(self.ui.next_actions_index) {
            Some(action) => action.entity_id.clone(),
            None => return,
        };

        // Find which nodes need to be expanded to show this item
        self.expand_to_reveal(&target_id);
        self.rebuild_tree();

        // Find the item in the tree
        for (i, item) in self.cache.tree_items.iter().enumerate() {
            if item.node.id() == target_id {
                self.ui.tree_index = i;
                break;
            }
        }
    }

    fn expand_to_reveal(&mut self, target_id: &str) {
        // For a solution, we need its problem expanded, and that problem's milestone expanded
        if let Some(solution) = self.data.solutions.iter().find(|s| s.id == target_id) {
            self.ui.expanded_nodes.insert(solution.problem_id.clone());

            if let Some(problem) = self
                .data
                .problems
                .iter()
                .find(|p| p.id == solution.problem_id)
            {
                if let Some(milestone_id) = &problem.milestone_id {
                    self.ui.expanded_nodes.insert(milestone_id.clone());
                } else {
                    self.ui.expanded_nodes.insert("backlog".to_string());
                }
            }
        }

        // For a problem, we need its milestone expanded
        if let Some(problem) = self.data.problems.iter().find(|p| p.id == target_id) {
            if let Some(milestone_id) = &problem.milestone_id {
                self.ui.expanded_nodes.insert(milestone_id.clone());
            } else {
                self.ui.expanded_nodes.insert("backlog".to_string());
            }
        }

        // For a critique, we need its solution and problem expanded
        if let Some(critique) = self.data.critiques.iter().find(|c| c.id == target_id) {
            self.ui.expanded_nodes.insert(critique.solution_id.clone());

            if let Some(solution) = self
                .data
                .solutions
                .iter()
                .find(|s| s.id == critique.solution_id)
            {
                self.ui.expanded_nodes.insert(solution.problem_id.clone());

                if let Some(problem) = self
                    .data
                    .problems
                    .iter()
                    .find(|p| p.id == solution.problem_id)
                {
                    if let Some(milestone_id) = &problem.milestone_id {
                        self.ui.expanded_nodes.insert(milestone_id.clone());
                    } else {
                        self.ui.expanded_nodes.insert("backlog".to_string());
                    }
                }
            }
        }
    }

    fn collapse_or_parent(&mut self) {
        if self.ui.focused_pane != FocusedPane::ProjectTree {
            return;
        }

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Collapse current node
                self.ui.expanded_nodes.remove(&node_id);
                self.rebuild_tree();
            } else if item.depth > 0 {
                // Move to parent
                for i in (0..self.ui.tree_index).rev() {
                    if self.cache.tree_items[i].depth < item.depth {
                        self.ui.tree_index = i;
                        break;
                    }
                }
            }
        }
    }

    fn expand_or_child(&mut self) {
        if self.ui.focused_pane != FocusedPane::ProjectTree {
            return;
        }

        if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            if !item.has_children {
                return;
            }

            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Move to first child
                if self.ui.tree_index + 1 < self.cache.tree_items.len() {
                    self.ui.tree_index += 1;
                }
            } else {
                // Expand
                self.ui.expanded_nodes.insert(node_id);
                self.rebuild_tree();
            }
        }
    }

    fn scroll_detail_down(&mut self) {
        self.ui.detail_scroll = self.ui.detail_scroll.saturating_add(1);
    }

    fn scroll_detail_up(&mut self) {
        self.ui.detail_scroll = self.ui.detail_scroll.saturating_sub(1);
    }

    fn page_detail_down(&mut self) {
        self.ui.detail_scroll = self.ui.detail_scroll.saturating_add(10);
    }

    fn toggle_related_panel(&mut self) {
        self.ui.show_related = !self.ui.show_related;
    }

    /// Load related items for the currently selected entity
    pub fn load_related_for_selected(&mut self) {
        use crate::db::search::find_similar;

        // Clear existing
        self.ui.related_items.clear();
        self.ui.related_selected = 0;

        // Get current selected entity info
        let (entity_type, entity_id) = match self.get_selected_entity_info() {
            Some(info) => info,
            None => return,
        };

        // Try to load related items from database
        if let Some(ref db) = self.db {
            if let Ok(results) = find_similar(db.conn(), &entity_type, &entity_id, None, 5) {
                self.ui.related_items = results
                    .into_iter()
                    .filter(|r| r.similarity > 0.5)
                    .collect();
            }
        }
    }

    fn get_selected_entity_info(&self) -> Option<(String, String)> {
        use super::tree::TreeNode;

        match self.ui.focused_pane {
            FocusedPane::NextActions => self
                .cache
                .next_actions
                .get(self.ui.next_actions_index)
                .map(|a| {
                    let entity_type = match a.entity_type {
                        super::next_actions::EntityType::Problem => "problem",
                        super::next_actions::EntityType::Solution => "solution",
                        super::next_actions::EntityType::Critique => "critique",
                    };
                    (entity_type.to_string(), a.entity_id.clone())
                }),
            FocusedPane::ProjectTree => {
                self.cache
                    .tree_items
                    .get(self.ui.tree_index)
                    .and_then(|item| match &item.node {
                        TreeNode::Problem { id, .. } => Some(("problem".to_string(), id.clone())),
                        TreeNode::Solution { id, .. } => Some(("solution".to_string(), id.clone())),
                        TreeNode::Critique { id, .. } => Some(("critique".to_string(), id.clone())),
                        TreeNode::Milestone { id, .. } => {
                            Some(("milestone".to_string(), id.clone()))
                        }
                        TreeNode::Backlog { .. } => None,
                    })
            }
        }
    }

    pub fn rebuild_tree(&mut self) {
        self.cache.tree_items = super::build_flat_tree(
            &self.data.milestones,
            &self.data.problems,
            &self.data.solutions,
            &self.data.critiques,
            &self.ui.expanded_nodes,
        );
    }

    pub fn context_hints(&self) -> String {
        use super::tree::TreeNode;

        match self.ui.focused_pane {
            FocusedPane::NextActions => {
                if let Some(action) = self.cache.next_actions.get(self.ui.next_actions_index) {
                    match action.entity_type {
                        super::next_actions::EntityType::Problem => {
                            format!(
                                "{}: [n]ew solution [s]olve [d]issolve [e]dit",
                                action.entity_id
                            )
                        }
                        super::next_actions::EntityType::Solution => {
                            format!(
                                "{}: [a]ccept [r]efute [n]ew critique [e]dit",
                                action.entity_id
                            )
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
                if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                    match &item.node {
                        TreeNode::Milestone { id, .. } => {
                            format!("{}: [e]dit", id)
                        }
                        TreeNode::Backlog { .. } => "[p]roblem new".to_string(),
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
        match self.ui.focused_pane {
            FocusedPane::NextActions => {
                if let Some(action) = self.cache.next_actions.get(self.ui.next_actions_index) {
                    self.cache.selected_detail = match action.entity_type {
                        super::next_actions::EntityType::Problem => self
                            .data
                            .problems
                            .iter()
                            .find(|p| p.id == action.entity_id)
                            .cloned()
                            .map(super::DetailContent::Problem)
                            .unwrap_or(super::DetailContent::None),
                        super::next_actions::EntityType::Solution => self
                            .data
                            .solutions
                            .iter()
                            .find(|s| s.id == action.entity_id)
                            .cloned()
                            .map(super::DetailContent::Solution)
                            .unwrap_or(super::DetailContent::None),
                        super::next_actions::EntityType::Critique => self
                            .data
                            .critiques
                            .iter()
                            .find(|c| c.id == action.entity_id)
                            .cloned()
                            .map(super::DetailContent::Critique)
                            .unwrap_or(super::DetailContent::None),
                    };
                }
            }
            FocusedPane::ProjectTree => {
                if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                    self.cache.selected_detail = match &item.node {
                        TreeNode::Milestone { id, .. } => self
                            .data
                            .milestones
                            .iter()
                            .find(|m| m.id == *id)
                            .cloned()
                            .map(super::DetailContent::Milestone)
                            .unwrap_or(super::DetailContent::None),
                        TreeNode::Backlog { .. } => super::DetailContent::None,
                        TreeNode::Problem { id, .. } => self
                            .data
                            .problems
                            .iter()
                            .find(|p| p.id == *id)
                            .cloned()
                            .map(super::DetailContent::Problem)
                            .unwrap_or(super::DetailContent::None),
                        TreeNode::Solution { id, .. } => self
                            .data
                            .solutions
                            .iter()
                            .find(|s| s.id == *id)
                            .cloned()
                            .map(super::DetailContent::Solution)
                            .unwrap_or(super::DetailContent::None),
                        TreeNode::Critique { id, .. } => self
                            .data
                            .critiques
                            .iter()
                            .find(|c| c.id == *id)
                            .cloned()
                            .map(super::DetailContent::Critique)
                            .unwrap_or(super::DetailContent::None),
                    };
                }
            }
        }
        self.ui.detail_scroll = 0; // Reset scroll on new selection
        self.load_related_for_selected(); // Load related items for new selection
    }

    fn get_selected_entity(&self) -> Option<(String, super::next_actions::EntityType)> {
        use super::tree::TreeNode;

        match self.ui.focused_pane {
            FocusedPane::NextActions => self
                .cache
                .next_actions
                .get(self.ui.next_actions_index)
                .map(|a| (a.entity_id.clone(), a.entity_type)),
            FocusedPane::ProjectTree => {
                self.cache
                    .tree_items
                    .get(self.ui.tree_index)
                    .and_then(|item| match &item.node {
                        TreeNode::Problem { id, .. } => {
                            Some((id.clone(), super::next_actions::EntityType::Problem))
                        }
                        TreeNode::Solution { id, .. } => {
                            Some((id.clone(), super::next_actions::EntityType::Solution))
                        }
                        TreeNode::Critique { id, .. } => {
                            Some((id.clone(), super::next_actions::EntityType::Critique))
                        }
                        _ => None,
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
        match self
            .store
            .with_metadata(&format!("Accept solution {}", solution_id), || {
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
        match self
            .store
            .with_metadata(&format!("Refute solution {}", solution_id), || {
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
        match self
            .store
            .with_metadata(&format!("Address critique {}", critique_id), || {
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
        match self
            .store
            .with_metadata(&format!("Dismiss critique {}", critique_id), || {
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
        self.data = ProjectData::load(&self.store)?;
        self.rebuild_cache();
        Ok(())
    }

    fn rebuild_cache(&mut self) {
        let user = self.store.jj_client.user_identity().unwrap_or_default();
        self.cache.next_actions = super::build_next_actions(
            &self.data.problems,
            &self.data.solutions,
            &self.data.critiques,
            &user,
        );
        self.rebuild_tree();
        self.update_selected_detail();
    }
}
