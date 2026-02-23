use crate::db::search::SimilarityResult;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, Milestone, Problem, Solution};
use crate::storage::MetadataStore;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use super::next_actions::EntityType;

mod actions;
mod editor;
mod navigation;
mod related;

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
    /// In-flight background related-items load: (entity_type, entity_id, receiver)
    pub related_rx: Option<(String, String, std::sync::mpsc::Receiver<Vec<SimilarityResult>>)>,
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
            related_rx: None,
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
    pub(super) store: MetadataStore,
    pub(super) db_path: Option<std::path::PathBuf>,
    pub(super) user: String,
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
        let next_actions = super::build_next_actions(
            &data.problems,
            &data.solutions,
            &data.critiques,
            &user,
        );
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

        // Store the db path for background queries (no connection held open)
        let db_path = store.jj_client.repo_root().join(".jj").join("jjj.db");
        let db_path = if db_path.exists() { Some(db_path) } else { None };

        let mut app = Self {
            should_quit: false,
            data,
            ui,
            cache,
            store,
            db_path,
            user,
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

    pub(super) fn show_flash(&mut self, message: &str) {
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
            KeyCode::Char('g') => self.goto_change()?,
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
}
