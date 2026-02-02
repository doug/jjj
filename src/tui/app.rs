use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, Milestone, Problem, Solution};
use crate::storage::MetadataStore;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::time::Duration;

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
    pub tree_index: usize,
    pub detail_scroll: u16,
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

        Ok(Self {
            should_quit: false,
            focused_pane: FocusedPane::NextActions,
            milestones,
            problems,
            solutions,
            critiques,
            next_actions,
            next_actions_index: 0,
            tree_index: 0,
            detail_scroll: 0,
            store,
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
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

    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Char('j') => self.scroll_detail_down(),
            KeyCode::Char('k') => self.scroll_detail_up(),
            KeyCode::Char(' ') => self.page_detail_down(),
            _ => {}
        }
        Ok(())
    }

    fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::NextActions => FocusedPane::ProjectTree,
            FocusedPane::ProjectTree => FocusedPane::NextActions,
        };
    }

    fn navigate_up(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index > 0 {
                    self.next_actions_index -= 1;
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index > 0 {
                    self.tree_index -= 1;
                }
            }
        }
    }

    fn navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index < self.next_actions.len().saturating_sub(1) {
                    self.next_actions_index += 1;
                }
            }
            FocusedPane::ProjectTree => {
                // Will implement with tree
                self.tree_index += 1;
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
}
