use crate::db::search::SimilarityResult;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, Milestone, Problem, Solution};
use crate::ranking::borda;
use crate::ranking::ordering;
use crate::storage::MetadataStore;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use super::next_actions::EntityType;

mod actions;
mod editor;
mod navigation;
mod related;

/// Which pane currently has keyboard focus.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    #[default]
    Tree,
    Detail,
}

/// Controls how keyboard input is interpreted by the event loop.
///
/// - `Normal` — standard navigation and action keys.
/// - `Help` — help overlay is displayed; any key returns to `Normal`.
/// - `Input` — the user is typing a value (new entity title, search query, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Help,
    Input {
        prompt: String,
        buffer: String,
        action: InputAction,
        cursor_pos: usize,
    },
}

/// The action to execute when the user confirms text entry in `InputMode::Input`.
///
/// Each variant carries the context needed to perform the action (e.g., the
/// `problem_id` to attach a new solution to). `Search` is handled specially —
/// the tree is filtered live as the user types rather than on confirmation.
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
    EditTags {
        entity_type: EntityType,
        entity_id: String,
    },
    Search,
    DissolveP {
        problem_id: String,
    },
    ConfirmDelete {
        entity_type: String,
        entity_id: String,
    },
    NewMilestone,
    MoveProblemToMilestone {
        problem_id: String,
    },
    BatchConfirmDelete {
        entities: Vec<(String, String)>, // Vec<(entity_type, entity_id)>
    },
    MoveProblemsToMilestone {
        problem_ids: Vec<String>,
    },
}

/// A pending request to suspend the TUI and open an entity in an external editor.
///
/// Created by `open_in_editor()` and consumed by `run_editor()` in the main
/// event loop. Carries the serialized original content so that unchanged saves
/// can be detected and skipped.
#[derive(Debug, Clone)]
pub struct EditorRequest {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub temp_path: std::path::PathBuf,
    pub original_content: String,
    pub editor: String,
}

/// Raw data loaded from storage — the single source of truth for the TUI.
///
/// All rendering derives from this snapshot. Refreshed wholesale by
/// [`App::refresh_data`] after any mutation. Kept separate from [`UiState`]
/// so navigation state (selection, scroll, expansion) survives data reloads.
pub struct ProjectData {
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,
    /// milestone_id -> problem_id -> (rank_position, voter_count_str)
    pub rankings: HashMap<String, HashMap<String, (usize, String)>>,
}

impl ProjectData {
    pub fn load(store: &MetadataStore) -> Result<Self> {
        let milestones = store.list_milestones()?;
        let problems = store.list_problems()?;
        let solutions = store.list_solutions()?;
        let critiques = store.list_critiques()?;
        let rankings = Self::compute_rankings(store, &milestones);
        Ok(Self {
            milestones,
            problems,
            solutions,
            critiques,
            rankings,
        })
    }

    /// Compute aggregated rankings per milestone using harmonic rank + squared votes.
    /// Returns milestone_id -> problem_id -> (rank_position, voter_count_str).
    fn compute_rankings(
        store: &MetadataStore,
        milestones: &[Milestone],
    ) -> HashMap<String, HashMap<String, (usize, String)>> {
        let mut result = HashMap::new();
        let base = store.meta_path();

        for milestone in milestones {
            let orderings = match ordering::load_all_orderings(base, &milestone.id) {
                Ok(o) => o,
                Err(_) => continue,
            };
            if orderings.is_empty() {
                continue;
            }

            let problem_count = orderings.values().map(|o| o.order.len()).max().unwrap_or(0);

            let aggregated = borda::aggregate_rankings(&orderings, problem_count);

            let mut milestone_rankings = HashMap::new();
            for (problem_id, rank) in &aggregated {
                milestone_rankings.insert(
                    problem_id.clone(),
                    (rank.position, format!("{}", rank.voter_count)),
                );
            }
            result.insert(milestone.id.clone(), milestone_rankings);
        }

        result
    }
}

/// All transient UI state: selection, scroll position, expansion, input mode.
///
/// Distinct from [`ProjectData`] (authoritative storage snapshot) and
/// [`RenderCache`] (derived display data). `UiState` persists across
/// `refresh_data()` calls so the user's navigation position is preserved after
/// mutations.
///
/// The related-items pipeline uses a three-stage debounce/background-load model:
/// 1. Selection change → `related_pending_load` is set with a timestamp.
/// 2. After 300 ms, a background thread is spawned and `related_rx` is set.
/// 3. Each tick, `check_pending_related_load()` polls the receiver and populates
///    `related_items` and `related_cache` when the result arrives.
pub struct UiState {
    pub focused_pane: FocusedPane,
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
    pub show_related: bool,
    pub related_items: Vec<SimilarityResult>,
    pub related_selected: usize,
    pub input_mode: InputMode,
    pub filter_actions_only: bool,
    /// IDs of multi-selected entities (UUID7, globally unique across types).
    pub selected_ids: HashSet<String>,
    /// Text search filter for the tree (set via '/' key)
    pub search_filter: Option<String>,
    /// Pending debounce request: `(entity_type, entity_id, requested_at)`.
    /// Cleared when the background thread is spawned or the selection changes.
    pub related_pending_load: Option<(String, String, Instant)>,
    /// LRU-style cache keyed by `(entity_type, entity_id)`. Cleared on
    /// `refresh_data()` so stale embeddings are not shown after mutations.
    pub related_cache: HashMap<(String, String), Vec<SimilarityResult>>,
    /// In-flight background load: `(entity_type, entity_id, receiver)`.
    /// Dropped (cancelling the load) when the selection changes.
    pub related_rx: Option<(
        String,
        String,
        std::sync::mpsc::Receiver<Vec<SimilarityResult>>,
    )>,
    /// Whether to show personal ordering (true) or global aggregated view (false).
    pub show_personal_ordering: bool,
    /// The current user's personal orderings per milestone (loaded on startup).
    /// milestone_id -> UserOrdering
    pub personal_orderings: HashMap<String, crate::ranking::ordering::UserOrdering>,
    /// Tier drilling state: stack of (milestone_id, start_index, end_index).
    /// Empty = showing all items. Each entry narrows to a third of the parent range.
    pub tier_drill: Vec<(String, usize, usize)>,
    /// Undo stack for ordering operations: (milestone_id, previous_ordering).
    pub ordering_undo: std::collections::VecDeque<(String, crate::ranking::ordering::UserOrdering)>,
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
            focused_pane: FocusedPane::Tree,
            tree_index: 0,
            expanded_nodes,
            detail_scroll: 0,
            flash_message: None,
            show_related: true,
            related_items: Vec::new(),
            related_selected: 0,
            input_mode: InputMode::Normal,
            filter_actions_only: false,
            selected_ids: HashSet::new(),
            search_filter: None,
            related_pending_load: None,
            related_cache: HashMap::new(),
            related_rx: None,
            show_personal_ordering: true,
            personal_orderings: HashMap::new(),
            tier_drill: Vec::new(),
            ordering_undo: std::collections::VecDeque::new(),
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
    /// Initialize the TUI application, loading all project data from storage.
    ///
    /// Discovers the jj repository, constructs the [`MetadataStore`], loads
    /// [`ProjectData`], and builds the initial render cache. Expands the backlog
    /// node and the first milestone by default. Kicks off the initial related-items
    /// load for whatever entity is selected first.
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        let data = ProjectData::load(&store)?;
        let mut ui = UiState::new();

        // Expand root and first milestone by default
        ui.expanded_nodes.insert("project-root".to_string());
        if let Some(m) = data.milestones.first() {
            ui.expanded_nodes.insert(m.id.clone());
        }

        let user = store.jj_client.user_identity().unwrap_or_default();

        // Load personal orderings for the current user, syncing with current problem list
        let mut personal_orderings = HashMap::new();
        for milestone in &data.milestones {
            if let Ok(Some(mut ord)) =
                ordering::load_user_ordering(store.meta_path(), &milestone.id, &user)
            {
                // Sync: append new problems, remove stale ones
                let current_ids: Vec<String> = data
                    .problems
                    .iter()
                    .filter(|p| p.milestone_id.as_deref() == Some(&milestone.id))
                    .map(|p| p.id.clone())
                    .collect();
                let existing: std::collections::HashSet<String> =
                    ord.order.iter().cloned().collect();
                for id in &current_ids {
                    if !existing.contains(id) {
                        ord.order.push(id.clone());
                    }
                }
                let current_set: std::collections::HashSet<&str> =
                    current_ids.iter().map(|s| s.as_str()).collect();
                ord.order.retain(|id| current_set.contains(id.as_str()));
                personal_orderings.insert(milestone.id.clone(), ord);
            }
        }
        ui.personal_orderings = personal_orderings;
        let next_actions = super::next_actions::build_next_actions(
            &data.problems,
            &data.solutions,
            &data.critiques,
            &user,
        );
        let tree_ctx = super::tree::TreeBuildContext {
            solutions: &data.solutions,
            critiques: &data.critiques,
            expanded_nodes: &ui.expanded_nodes,
            personal_orderings: &ui.personal_orderings,
        };
        let tree_items = super::tree::build_flat_tree_ranked(
            &data.milestones,
            &data.problems,
            &tree_ctx,
            ui.show_personal_ordering,
            &ui.tier_drill,
        );

        let mut cache = RenderCache {
            next_actions,
            tree_items,
            selected_detail: super::DetailContent::None,
        };
        super::annotate_tree_with_actions(&mut cache.tree_items, &cache.next_actions);

        // Store the db path for background queries (no connection held open)
        let db_path = store.jj_client.repo_root().join(".jj").join("jjj.db");
        let db_path = if db_path.exists() {
            Some(db_path)
        } else {
            None
        };

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

    /// Run the main TUI event loop until the user quits.
    ///
    /// Each iteration: checks for a pending editor request (suspends terminal if
    /// present), expires flash messages, polls for completed background loads,
    /// renders a frame, then waits up to 100 ms for a key press. Exits when
    /// `should_quit` is set.
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
                        self.handle_key(key)?;
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

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Ctrl+C always exits, regardless of mode
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Ok(());
        }

        match &self.ui.input_mode {
            InputMode::Help => {
                // Any key exits help
                self.ui.input_mode = InputMode::Normal;
            }
            InputMode::Input { .. } => {
                self.handle_input_key(key.code)?;
            }
            InputMode::Normal => {
                self.handle_normal_key(key)?;
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.ui.focused_pane {
            FocusedPane::Tree => self.handle_tree_key(key)?,
            FocusedPane::Detail => self.handle_detail_key(key)?,
        }
        Ok(())
    }

    fn handle_tree_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => {
                self.ui.focused_pane = FocusedPane::Detail;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.bubble_up()?;
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.bubble_up()?;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.bubble_down()?;
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.bubble_down()?;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.assign_top_tier()?;
            }
            KeyCode::Char('K') => {
                self.assign_top_tier()?;
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.assign_bottom_tier()?;
            }
            KeyCode::Char('J') => {
                self.assign_bottom_tier()?;
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.tier_drill_out();
            }
            KeyCode::Char('H') => {
                self.tier_drill_out();
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.tier_drill_in()?;
            }
            KeyCode::Char('L') => {
                self.tier_drill_in()?;
            }
            KeyCode::Up | KeyCode::Char('k') => self.navigate_up(),
            KeyCode::Down | KeyCode::Char('j') => self.navigate_down(),
            KeyCode::Left | KeyCode::Char('h') => self.collapse_or_parent(),
            KeyCode::Right | KeyCode::Char('l') => self.expand_or_child(),
            KeyCode::Char(' ') => self.toggle_selection(),
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.undo_ordering()?;
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.select_all_visible();
            }
            KeyCode::Char('a') => self.handle_action_a()?,
            KeyCode::Char('d') => self.handle_action_d()?,
            KeyCode::Char('n') => self.start_new_item()?,
            KeyCode::Char('e') => self.start_edit_title()?,
            KeyCode::Char('t') => self.start_edit_tags()?,
            KeyCode::Char('s') => self.handle_action_s()?,
            KeyCode::Char('o') => self.handle_action_o()?,
            KeyCode::Char('A') => self.handle_action_shift_a()?,
            KeyCode::Char('u') => self.handle_action_u()?,
            KeyCode::Char('v') => self.handle_action_v()?,
            KeyCode::Char('f') => self.toggle_filter(),
            KeyCode::Char('/') => self.start_search(),
            KeyCode::Char('R') => self.toggle_related_panel(),
            KeyCode::Char('g') => self.goto_change()?,
            KeyCode::Char('G') => self.jump_to_next_action(false),
            KeyCode::Char('E') => self.open_in_editor()?,
            KeyCode::Char('x') => self.start_delete()?,
            KeyCode::Char('c') => self.cycle_confidence()?,
            KeyCode::Char('r') => self.toggle_ordering_view(),
            KeyCode::Char('m') => self.start_move_to_milestone()?,
            KeyCode::Char('b') => self.page_detail_up(),
            KeyCode::Char('?') => self.toggle_help(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.add_vote()?,
            KeyCode::Char('-') => self.remove_vote()?,
            KeyCode::Esc => self.clear_selection(),
            _ => {}
        }
        Ok(())
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab | KeyCode::Esc => {
                self.ui.focused_pane = FocusedPane::Tree;
            }
            KeyCode::Down | KeyCode::Char('j') => self.scroll_detail_down(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_detail_up(),
            KeyCode::Char('b') => self.page_detail_up(),
            KeyCode::Char(' ') => self.page_detail_down(),
            KeyCode::Char('g') => self.detail_scroll_to_top(),
            KeyCode::Char('G') => self.detail_scroll_to_bottom(),
            KeyCode::Char('?') => self.toggle_help(),
            _ => {}
        }
        Ok(())
    }

    fn handle_input_key(&mut self, key: KeyCode) -> Result<()> {
        // Extract current input state
        let (prompt, buffer, action, cursor_pos) = match &self.ui.input_mode {
            InputMode::Input {
                prompt,
                buffer,
                action,
                cursor_pos,
            } => (prompt.clone(), buffer.clone(), action.clone(), *cursor_pos),
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
                KeyCode::Left => {
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer,
                        action,
                        cursor_pos: cursor_pos.saturating_sub(1),
                    };
                }
                KeyCode::Right => {
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer: buffer.clone(),
                        action,
                        cursor_pos: (cursor_pos + 1).min(buffer.len()),
                    };
                }
                KeyCode::Home => {
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer,
                        action,
                        cursor_pos: 0,
                    };
                }
                KeyCode::End => {
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer: buffer.clone(),
                        action,
                        cursor_pos: buffer.len(),
                    };
                }
                KeyCode::Backspace => {
                    if cursor_pos > 0 {
                        let mut new_buffer = buffer;
                        new_buffer.remove(cursor_pos - 1);
                        let new_cursor = cursor_pos - 1;
                        self.ui.search_filter = if new_buffer.is_empty() {
                            None
                        } else {
                            Some(new_buffer.clone())
                        };
                        self.ui.input_mode = InputMode::Input {
                            prompt,
                            buffer: new_buffer,
                            action,
                            cursor_pos: new_cursor,
                        };
                        self.apply_search_filter();
                    }
                }
                KeyCode::Char(c) => {
                    let mut new_buffer = buffer;
                    new_buffer.insert(cursor_pos, c);
                    self.ui.search_filter = Some(new_buffer.clone());
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer: new_buffer,
                        action,
                        cursor_pos: cursor_pos + 1,
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
                if !buffer.is_empty()
                    || matches!(
                        action,
                        InputAction::EditTags { .. }
                            | InputAction::MoveProblemToMilestone { .. }
                            | InputAction::MoveProblemsToMilestone { .. }
                    )
                {
                    self.execute_input_action(&action, &buffer)?;
                }
                self.ui.input_mode = InputMode::Normal;
            }
            KeyCode::Left => {
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer,
                    action,
                    cursor_pos: cursor_pos.saturating_sub(1),
                };
            }
            KeyCode::Right => {
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer: buffer.clone(),
                    action,
                    cursor_pos: (cursor_pos + 1).min(buffer.len()),
                };
            }
            KeyCode::Home => {
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer,
                    action,
                    cursor_pos: 0,
                };
            }
            KeyCode::End => {
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer: buffer.clone(),
                    action,
                    cursor_pos: buffer.len(),
                };
            }
            KeyCode::Backspace => {
                if cursor_pos > 0 {
                    let mut new_buffer = buffer;
                    new_buffer.remove(cursor_pos - 1);
                    self.ui.input_mode = InputMode::Input {
                        prompt,
                        buffer: new_buffer,
                        action,
                        cursor_pos: cursor_pos - 1,
                    };
                }
            }
            KeyCode::Char(c) => {
                let mut new_buffer = buffer;
                new_buffer.insert(cursor_pos, c);
                self.ui.input_mode = InputMode::Input {
                    prompt,
                    buffer: new_buffer,
                    action,
                    cursor_pos: cursor_pos + 1,
                };
            }
            _ => {}
        }
        self.update_milestone_prompt_hint();
        Ok(())
    }

    /// When typing a milestone name, update the prompt to show the live fuzzy match.
    fn update_milestone_prompt_hint(&mut self) {
        if let InputMode::Input {
            prompt,
            buffer,
            action,
            cursor_pos,
        } = &self.ui.input_mode
        {
            if !matches!(
                action,
                InputAction::MoveProblemToMilestone { .. }
                    | InputAction::MoveProblemsToMilestone { .. }
            ) {
                return;
            }
            let hint = if buffer.is_empty() {
                "→ backlog".to_string()
            } else {
                let input_lower = buffer.to_lowercase();
                self.data
                    .milestones
                    .iter()
                    .find(|m| m.title.to_lowercase().contains(&input_lower))
                    .map(|m| format!("→ {}", m.title))
                    .unwrap_or_else(|| "→ (no match)".to_string())
            };
            let new_prompt = format!("Milestone [{}]: ", hint);
            // Only update if prompt actually changed to avoid unnecessary clones
            if *prompt != new_prompt {
                let buffer = buffer.clone();
                let action = action.clone();
                let cursor_pos = *cursor_pos;
                self.ui.input_mode = InputMode::Input {
                    prompt: new_prompt,
                    buffer,
                    action,
                    cursor_pos,
                };
            }
        }
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
            InputAction::EditTags {
                entity_type,
                entity_id,
            } => {
                self.update_tags(entity_type, entity_id, title)?;
            }
            InputAction::DissolveP { problem_id } => {
                self.dissolve_problem(problem_id, title)?;
            }
            InputAction::ConfirmDelete {
                entity_type,
                entity_id,
            } => {
                if title.trim() == "y" {
                    self.delete_entity(entity_type, entity_id)?;
                } else {
                    self.show_flash("Delete cancelled");
                }
            }
            InputAction::NewMilestone => {
                self.create_milestone(title)?;
            }
            InputAction::MoveProblemToMilestone { problem_id } => {
                self.move_problem_to_milestone(problem_id, title)?;
            }
            InputAction::MoveProblemsToMilestone { problem_ids } => {
                self.batch_move_to_milestone(problem_ids, title)?;
            }
            InputAction::BatchConfirmDelete { entities } => {
                if title.trim() == "y" {
                    self.batch_delete(entities)?;
                } else {
                    self.show_flash("Delete cancelled");
                }
            }
            InputAction::Search => {
                // Search is handled directly in handle_input_key
            }
        }
        Ok(())
    }
}
