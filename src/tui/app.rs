use crate::db::search::SimilarityResult;
use crate::db::Database;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, Milestone, Problem, Solution};
use crate::storage::MetadataStore;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use super::next_actions::EntityType;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Help,
    Input {
        prompt: String,
        buffer: String,
        action: InputAction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    NewProblem {
        milestone_id: Option<String>,
    },
    NewSolution {
        problem_id: String,
    },
    NewCritique {
        solution_id: String,
    },
    EditTitle {
        entity_type: EntityType,
        entity_id: String,
    },
    Search,
}

#[derive(Debug, Clone)]
pub struct EditorRequest {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub temp_path: std::path::PathBuf,
    pub original_content: String,
    pub editor: String,
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
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
    pub show_related: bool,
    pub related_items: Vec<SimilarityResult>,
    pub related_selected: usize,
    pub input_mode: InputMode,
    pub filter_actions_only: bool,
    /// Text search filter for the tree (set via '/' key)
    pub search_filter: Option<String>,
    /// Debounce timer for related items loading: (entity_type, entity_id, requested_at)
    pub related_pending_load: Option<(String, String, Instant)>,
    /// Cache of related items by (entity_type, entity_id)
    pub related_cache: HashMap<(String, String), Vec<SimilarityResult>>,
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
            tree_index: 0,
            expanded_nodes,
            detail_scroll: 0,
            flash_message: None,
            show_related: true,
            related_items: Vec::new(),
            related_selected: 0,
            input_mode: InputMode::Normal,
            filter_actions_only: false,
            search_filter: None,
            related_pending_load: None,
            related_cache: HashMap::new(),
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
    pub editor_request: Option<EditorRequest>,
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

        let mut cache = RenderCache {
            next_actions,
            tree_items,
            selected_detail: super::DetailContent::None,
        };
        super::annotate_tree_with_actions(&mut cache.tree_items, &cache.next_actions);

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
            editor_request: None,
        };
        app.update_selected_detail();
        app.load_related_for_selected();
        Ok(app)
    }

    pub fn run<B: Backend + std::io::Write>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
            // Check for editor request
            if let Some(request) = self.editor_request.take() {
                self.run_editor(terminal, request)?;
                continue;
            }

            self.clear_expired_flash();
            self.check_pending_related_load();
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
        match &self.ui.input_mode {
            InputMode::Help => {
                // Any key exits help
                self.ui.input_mode = InputMode::Normal;
            }
            InputMode::Input { .. } => {
                self.handle_input_key(key)?;
            }
            InputMode::Normal => {
                self.handle_normal_key(key)?;
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.jump_to_next_action(false),
            KeyCode::BackTab => self.jump_to_next_action(true),
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
            KeyCode::Char('n') => self.start_new_item()?,
            KeyCode::Char('e') => self.start_edit_title()?,
            KeyCode::Char('s') => self.handle_action_s()?,
            KeyCode::Char('o') => self.handle_action_o()?,
            KeyCode::Char('v') => self.handle_action_v()?,
            KeyCode::Char('f') => self.toggle_filter(),
            KeyCode::Char('/') => self.start_search(),
            KeyCode::Char('R') => self.toggle_related_panel(),
            KeyCode::Char('E') => self.open_in_editor()?,
            KeyCode::Char('?') => self.toggle_help(),
            _ => {}
        }
        Ok(())
    }

    fn handle_input_key(&mut self, key: KeyCode) -> Result<()> {
        // Extract current input state
        let (prompt, buffer, action) = match &self.ui.input_mode {
            InputMode::Input {
                prompt,
                buffer,
                action,
            } => (prompt.clone(), buffer.clone(), action.clone()),
            _ => return Ok(()),
        };

        // Search mode has special live-filtering behavior
        if action == InputAction::Search {
            match key {
                KeyCode::Esc => {
                    self.ui.search_filter = None;
                    self.ui.input_mode = InputMode::Normal;
                    self.apply_search_filter();
                }
                KeyCode::Enter => {
                    // Keep filter active, just exit input mode
                    self.ui.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    let mut new_buffer = buffer;
                    new_buffer.pop();
                    self.ui.search_filter = if new_buffer.is_empty() {
                        None
                    } else {
                        Some(new_buffer.clone())
                    };
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer: new_buffer,
                        action,
                    };
                    self.apply_search_filter();
                }
                KeyCode::Char(c) => {
                    let mut new_buffer = buffer;
                    new_buffer.push(c);
                    self.ui.search_filter = Some(new_buffer.clone());
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer: new_buffer,
                        action,
                    };
                    self.apply_search_filter();
                }
                _ => {}
            }
            return Ok(());
        }

        match key {
            KeyCode::Esc => {
                self.ui.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                if !buffer.is_empty() {
                    self.execute_input_action(&action, &buffer)?;
                }
                self.ui.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                let mut new_buffer = buffer;
                new_buffer.pop();
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer: new_buffer,
                    action,
                };
            }
            KeyCode::Char(c) => {
                let mut new_buffer = buffer;
                new_buffer.push(c);
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer: new_buffer,
                    action,
                };
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_input_action(&mut self, action: &InputAction, title: &str) -> Result<()> {
        match action {
            InputAction::NewProblem { milestone_id } => {
                self.create_problem(title, milestone_id.clone())?;
            }
            InputAction::NewSolution { problem_id } => {
                self.create_solution(title, problem_id)?;
            }
            InputAction::NewCritique { solution_id } => {
                self.create_critique(title, solution_id)?;
            }
            InputAction::EditTitle {
                entity_type,
                entity_id,
            } => {
                self.update_title(entity_type, entity_id, title)?;
            }
            InputAction::Search => {
                // Search is handled directly in handle_input_key
            }
        }
        Ok(())
    }

    fn create_problem(&mut self, title: &str, milestone_id: Option<String>) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Problem;

        let id = generate_id();
        let mut problem = Problem::new(id.clone(), title.to_string());
        problem.milestone_id = milestone_id;

        self.store
            .with_metadata(&format!("Create problem: {}", title), || {
                self.store.save_problem(&problem)
            })?;

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    fn create_solution(&mut self, title: &str, problem_id: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Solution;

        let id = generate_id();
        let solution = Solution::new(id.clone(), title.to_string(), problem_id.to_string());

        self.store
            .with_metadata(&format!("Create solution: {}", title), || {
                self.store.save_solution(&solution)
            })?;

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    fn create_critique(&mut self, title: &str, solution_id: &str) -> Result<()> {
        use crate::id::generate_id;
        use crate::models::Critique;

        let id = generate_id();
        let critique = Critique::new(id.clone(), title.to_string(), solution_id.to_string());

        self.store
            .with_metadata(&format!("Create critique: {}", title), || {
                self.store.save_critique(&critique)
            })?;

        self.show_flash(&format!("Created {}", id));
        self.refresh_data()?;
        Ok(())
    }

    fn update_title(
        &mut self,
        entity_type: &EntityType,
        entity_id: &str,
        new_title: &str,
    ) -> Result<()> {
        match entity_type {
            EntityType::Problem => {
                self.store.with_metadata(
                    &format!("Update problem title: {}", new_title),
                    || {
                        let mut problem = self.store.load_problem(entity_id)?;
                        problem.title = new_title.to_string();
                        self.store.save_problem(&problem)
                    },
                )?;
            }
            EntityType::Solution => {
                self.store.with_metadata(
                    &format!("Update solution title: {}", new_title),
                    || {
                        let mut solution = self.store.load_solution(entity_id)?;
                        solution.title = new_title.to_string();
                        self.store.save_solution(&solution)
                    },
                )?;
            }
            EntityType::Critique => {
                self.store.with_metadata(
                    &format!("Update critique title: {}", new_title),
                    || {
                        let mut critique = self.store.load_critique(entity_id)?;
                        critique.title = new_title.to_string();
                        self.store.save_critique(&critique)
                    },
                )?;
            }
        }

        self.show_flash(&format!("Updated {}", entity_id));
        self.refresh_data()?;
        Ok(())
    }

    fn navigate_up(&mut self) {
        if self.ui.tree_index > 0 {
            self.ui.tree_index -= 1;
        }
        self.update_selected_detail();
    }

    fn navigate_down(&mut self) {
        if self.ui.tree_index < self.cache.tree_items.len().saturating_sub(1) {
            self.ui.tree_index += 1;
        }
        self.update_selected_detail();
    }

    fn jump_to_next_action(&mut self, reverse: bool) {
        if self.cache.tree_items.is_empty() {
            return;
        }

        // Find indices of items with action symbols
        let action_indices: Vec<usize> = self
            .cache
            .tree_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.action_symbol.is_some())
            .map(|(i, _)| i)
            .collect();

        if action_indices.is_empty() {
            return;
        }

        // Find next action item
        let current = self.ui.tree_index;
        let next_index = if reverse {
            // Find previous action item (or wrap to last)
            action_indices
                .iter()
                .rev()
                .find(|&&i| i < current)
                .or_else(|| action_indices.last())
                .copied()
        } else {
            // Find next action item (or wrap to first)
            action_indices
                .iter()
                .find(|&&i| i > current)
                .or_else(|| action_indices.first())
                .copied()
        };

        if let Some(idx) = next_index {
            // Expand ancestors to reveal the item
            let target_id = self.cache.tree_items[idx].node.id().to_string();
            self.expand_to_reveal(&target_id);
            self.rebuild_tree();
            // Re-find the item after tree rebuild
            for (i, item) in self.cache.tree_items.iter().enumerate() {
                if item.node.id() == target_id {
                    self.ui.tree_index = i;
                    break;
                }
            }
            self.update_selected_detail();
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

    fn toggle_filter(&mut self) {
        self.ui.filter_actions_only = !self.ui.filter_actions_only;
        let mode = if self.ui.filter_actions_only {
            "Actions only"
        } else {
            "Full tree"
        };
        self.show_flash(mode);
    }

    fn start_search(&mut self) {
        self.ui.input_mode = InputMode::Input {
            prompt: "/".to_string(),
            buffer: self.ui.search_filter.clone().unwrap_or_default(),
            action: InputAction::Search,
        };
    }

    fn toggle_help(&mut self) {
        self.ui.input_mode = match &self.ui.input_mode {
            InputMode::Help => InputMode::Normal,
            _ => InputMode::Help,
        };
    }

    /// Schedule a debounced load of related items for the currently selected entity
    pub fn load_related_for_selected(&mut self) {
        self.ui.related_selected = 0;

        // Get current selected entity info
        let (entity_type, entity_id) = match self.get_selected_entity_info() {
            Some(info) => info,
            None => {
                self.ui.related_items.clear();
                self.ui.related_pending_load = None;
                return;
            }
        };

        // Check cache first
        let cache_key = (entity_type.clone(), entity_id.clone());
        if let Some(cached) = self.ui.related_cache.get(&cache_key) {
            self.ui.related_items = cached.clone();
            self.ui.related_pending_load = None;
            return;
        }

        // Schedule debounced load
        self.ui.related_pending_load = Some((entity_type, entity_id, Instant::now()));
    }

    /// Check if a pending related items load is ready (debounce expired)
    fn check_pending_related_load(&mut self) {
        use crate::db::search::find_similar;

        let (entity_type, entity_id) = match &self.ui.related_pending_load {
            Some((et, eid, requested_at))
                if requested_at.elapsed() >= Duration::from_millis(300) =>
            {
                (et.clone(), eid.clone())
            }
            _ => return,
        };

        self.ui.related_pending_load = None;

        // Verify selection hasn't changed
        if let Some((current_type, current_id)) = self.get_selected_entity_info() {
            if current_type != entity_type || current_id != entity_id {
                return;
            }
        } else {
            return;
        }

        // Load from database
        let mut items = Vec::new();
        if let Some(ref db) = self.db {
            if let Ok(results) = find_similar(db.conn(), &entity_type, &entity_id, None, 5) {
                items = results.into_iter().filter(|r| r.similarity > 0.5).collect();
            }
        }

        // Update cache and display
        let cache_key = (entity_type, entity_id);
        self.ui.related_cache.insert(cache_key, items.clone());
        self.ui.related_items = items;
    }

    fn get_selected_entity_info(&self) -> Option<(String, String)> {
        use super::tree::TreeNode;

        self.cache
            .tree_items
            .get(self.ui.tree_index)
            .and_then(|item| match &item.node {
                TreeNode::Problem { id, .. } => Some(("problem".to_string(), id.clone())),
                TreeNode::Solution { id, .. } => Some(("solution".to_string(), id.clone())),
                TreeNode::Critique { id, .. } => Some(("critique".to_string(), id.clone())),
                TreeNode::Milestone { id, .. } => Some(("milestone".to_string(), id.clone())),
                TreeNode::Backlog { .. } => None,
            })
    }

    pub fn rebuild_tree(&mut self) {
        self.cache.tree_items = super::build_flat_tree(
            &self.data.milestones,
            &self.data.problems,
            &self.data.solutions,
            &self.data.critiques,
            &self.ui.expanded_nodes,
        );
        // Re-apply search filter if active
        if self.ui.search_filter.is_some() {
            self.apply_search_filter_to_tree();
        }
    }

    fn apply_search_filter(&mut self) {
        self.rebuild_tree();
        super::annotate_tree_with_actions(&mut self.cache.tree_items, &self.cache.next_actions);
        // Clamp tree_index
        let max_index = self.cache.tree_items.len().saturating_sub(1);
        if self.ui.tree_index > max_index {
            self.ui.tree_index = max_index;
        }
        self.update_selected_detail();
    }

    fn apply_search_filter_to_tree(&mut self) {
        if let Some(ref query) = self.ui.search_filter {
            let query_lower = query.to_lowercase();
            self.cache.tree_items.retain(|item| {
                let title = item.node.title().to_lowercase();
                let id = item.node.id().to_lowercase();
                title.contains(&query_lower) || id.contains(&query_lower)
            });
        }
    }

    pub fn context_hints(&self) -> String {
        use super::tree::TreeNode;

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

    pub fn update_selected_detail(&mut self) {
        use super::tree::TreeNode;

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
        self.ui.detail_scroll = 0; // Reset scroll on new selection
        self.load_related_for_selected(); // Load related items for new selection
    }

    fn get_selected_entity(&self) -> Option<(String, super::next_actions::EntityType)> {
        use super::tree::TreeNode;

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

    fn start_new_item(&mut self) -> Result<()> {
        use super::tree::TreeNode;

        let (prompt, action) = if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
            match &item.node {
                TreeNode::Milestone { id, .. } => (
                    "New problem title: ".to_string(),
                    InputAction::NewProblem {
                        milestone_id: Some(id.clone()),
                    },
                ),
                TreeNode::Backlog { .. } => (
                    "New problem title: ".to_string(),
                    InputAction::NewProblem { milestone_id: None },
                ),
                TreeNode::Problem { id, .. } => (
                    "New solution title: ".to_string(),
                    InputAction::NewSolution {
                        problem_id: id.clone(),
                    },
                ),
                TreeNode::Solution { id, .. } => (
                    "New critique title: ".to_string(),
                    InputAction::NewCritique {
                        solution_id: id.clone(),
                    },
                ),
                TreeNode::Critique { .. } => return Ok(()),
            }
        } else {
            return Ok(());
        };

        self.ui.input_mode = InputMode::Input {
            prompt,
            buffer: String::new(),
            action,
        };
        Ok(())
    }

    fn start_edit_title(&mut self) -> Result<()> {
        use super::tree::TreeNode;

        let (prompt, action, current_title) =
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Problem,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Solution { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Solution,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Critique { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Critique,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    _ => return Ok(()),
                }
            } else {
                return Ok(());
            };

        self.ui.input_mode = InputMode::Input {
            prompt,
            buffer: current_title,
            action,
        };
        Ok(())
    }

    fn handle_action_s(&mut self) -> Result<()> {
        use crate::models::ProblemStatus;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Problem {
                let id_clone = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Solve problem {}", id), || {
                        let mut problem = self.store.load_problem(&id)?;
                        problem.set_status(ProblemStatus::Solved);
                        self.store.save_problem(&problem)
                    }) {
                    Ok(_) => {
                        self.show_flash(&format!("{} solved", id_clone));
                        self.refresh_data()?;
                    }
                    Err(e) => {
                        self.show_flash(&format!("Error: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_action_o(&mut self) -> Result<()> {
        use crate::models::ProblemStatus;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Problem {
                let id_clone = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Reopen problem {}", id), || {
                        let mut problem = self.store.load_problem(&id)?;
                        problem.set_status(ProblemStatus::Open);
                        self.store.save_problem(&problem)
                    }) {
                    Ok(_) => {
                        self.show_flash(&format!("{} reopened", id_clone));
                        self.refresh_data()?;
                    }
                    Err(e) => {
                        self.show_flash(&format!("Error: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_action_v(&mut self) -> Result<()> {
        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Critique {
                let id_clone = id.clone();
                match self
                    .store
                    .with_metadata(&format!("Validate critique {}", id), || {
                        let mut critique = self.store.load_critique(&id)?;
                        critique.validate();
                        self.store.save_critique(&critique)
                    }) {
                    Ok(_) => {
                        self.show_flash(&format!("{} validated", id_clone));
                        self.refresh_data()?;
                    }
                    Err(e) => {
                        self.show_flash(&format!("Error: {}", e));
                    }
                }
            }
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
        self.ui.related_cache.clear();
        self.rebuild_cache();
        // Clamp tree_index to valid range after data change
        let max_index = self.cache.tree_items.len().saturating_sub(1);
        if self.ui.tree_index > max_index {
            self.ui.tree_index = max_index;
        }
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
        // Annotate tree with action symbols
        super::annotate_tree_with_actions(&mut self.cache.tree_items, &self.cache.next_actions);
        self.update_selected_detail();
    }

    fn open_in_editor(&mut self) -> Result<()> {
        use super::tree::TreeNode;

        // Get selected entity
        let (entity_type, entity_id) =
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, .. } => (EntityType::Problem, id.clone()),
                    TreeNode::Solution { id, .. } => (EntityType::Solution, id.clone()),
                    TreeNode::Critique { id, .. } => (EntityType::Critique, id.clone()),
                    _ => {
                        self.show_flash("Cannot edit this item type");
                        return Ok(());
                    }
                }
            } else {
                self.show_flash("No item selected");
                return Ok(());
            };

        // Serialize entity to temp file
        let temp_path = std::env::temp_dir().join(format!(
            "jjj-edit-{}.md",
            &entity_id[..8.min(entity_id.len())]
        ));
        let original_content = match self.serialize_entity_for_edit(&entity_type, &entity_id) {
            Ok(content) => content,
            Err(e) => {
                self.show_flash(&format!("Load error: {}", e));
                return Ok(());
            }
        };

        if let Err(e) = std::fs::write(&temp_path, &original_content) {
            self.show_flash(&format!("Write error: {}", e));
            return Ok(());
        }

        // Get editor
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());

        // Signal that we need to suspend
        self.editor_request = Some(EditorRequest {
            entity_type,
            entity_id,
            temp_path,
            original_content,
            editor,
        });

        Ok(())
    }

    fn serialize_entity_for_edit(
        &self,
        entity_type: &EntityType,
        entity_id: &str,
    ) -> Result<String> {
        match entity_type {
            EntityType::Problem => {
                let problem = self.store.load_problem(entity_id)?;
                Ok(format!(
                    "---\ntitle: {}\nstatus: {:?}\npriority: {}\n---\n\n## Description\n\n{}\n",
                    problem.title,
                    problem.status,
                    problem.priority,
                    if problem.description.is_empty() {
                        ""
                    } else {
                        &problem.description
                    }
                ))
            }
            EntityType::Solution => {
                let solution = self.store.load_solution(entity_id)?;
                Ok(format!(
                    "---\ntitle: {}\nstatus: {:?}\n---\n\n## Description\n\n{}\n",
                    solution.title,
                    solution.status,
                    if solution.approach.is_empty() {
                        ""
                    } else {
                        &solution.approach
                    }
                ))
            }
            EntityType::Critique => {
                let critique = self.store.load_critique(entity_id)?;
                Ok(format!(
                    "---\ntitle: {}\nstatus: {:?}\nseverity: {}\n---\n\n## Description\n\n{}\n",
                    critique.title,
                    critique.status,
                    critique.severity,
                    if critique.argument.is_empty() {
                        ""
                    } else {
                        &critique.argument
                    }
                ))
            }
        }
    }

    fn run_editor<B: Backend + std::io::Write>(
        &mut self,
        terminal: &mut Terminal<B>,
        request: EditorRequest,
    ) -> Result<()> {
        use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
        use crossterm::execute;
        use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
        use std::process::Command;

        // Leave alternate screen
        crossterm::terminal::disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;

        // Run editor
        let status = Command::new(&request.editor)
            .arg(&request.temp_path)
            .status();

        // Re-enter alternate screen
        crossterm::terminal::enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        terminal.clear()?;

        // Process result
        match status {
            Ok(exit_status) if exit_status.success() => {
                let new_content = std::fs::read_to_string(&request.temp_path)?;
                if new_content == request.original_content {
                    self.show_flash("No changes");
                } else {
                    self.apply_edited_content(
                        &request.entity_type,
                        &request.entity_id,
                        &new_content,
                    )?;
                    self.show_flash(&format!("Updated {}", request.entity_id));
                }
            }
            Ok(_) => {
                self.show_flash("Edit cancelled");
            }
            Err(e) => {
                self.show_flash(&format!("Editor error: {}", e));
            }
        }

        // Clean up temp file
        let _ = std::fs::remove_file(&request.temp_path);

        Ok(())
    }

    fn apply_edited_content(
        &mut self,
        entity_type: &EntityType,
        entity_id: &str,
        content: &str,
    ) -> Result<()> {
        // Simple parsing: extract title from frontmatter, description from body
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(crate::error::JjjError::Validation(
                "Invalid format".to_string(),
            ));
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim();

        // Extract title from frontmatter
        let title = frontmatter
            .lines()
            .find(|l| l.starts_with("title:"))
            .map(|l| l.trim_start_matches("title:").trim().to_string())
            .unwrap_or_default();

        // Extract description from body (after ## Description header)
        let description = body
            .strip_prefix("## Description")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        match entity_type {
            EntityType::Problem => {
                self.store
                    .with_metadata(&format!("Edit problem {}", entity_id), || {
                        let mut problem = self.store.load_problem(entity_id)?;
                        problem.title = title.clone();
                        problem.description = description.clone();
                        self.store.save_problem(&problem)
                    })?;
            }
            EntityType::Solution => {
                self.store
                    .with_metadata(&format!("Edit solution {}", entity_id), || {
                        let mut solution = self.store.load_solution(entity_id)?;
                        solution.title = title.clone();
                        solution.approach = description.clone();
                        self.store.save_solution(&solution)
                    })?;
            }
            EntityType::Critique => {
                self.store
                    .with_metadata(&format!("Edit critique {}", entity_id), || {
                        let mut critique = self.store.load_critique(entity_id)?;
                        critique.title = title.clone();
                        critique.argument = description.clone();
                        self.store.save_critique(&critique)
                    })?;
            }
        }

        self.refresh_data()?;
        Ok(())
    }
}
