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

        Ok(Self {
            should_quit: false,
            focused_pane: FocusedPane::NextActions,
            milestones,
            problems,
            solutions,
            critiques,
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
}
