mod app;
mod detail;
mod next_actions;
mod tree;
mod ui;

pub use app::App;
pub use detail::{DetailContent, ProblemRankInfo};
pub use next_actions::{build_next_actions, EntityType, NextAction};
pub use tree::{
    annotate_tree_with_actions, build_flat_tree, filter_tree_to_actions, FlatTreeItem, TreeNode,
};

use crate::error::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Launch the interactive TUI
pub fn launch() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    // The TUI now owns the terminal. Suppress direct stdout/stderr writes from
    // shared code paths (automation, storage warnings, domain hooks) that would
    // otherwise scroll the alt-screen and visibly stack the footer/status bar.
    crate::output::set_tui_active(true);
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    // Real terminal restored: direct writes are safe again.
    crate::output::set_tui_active(false);
    terminal.show_cursor()?;

    res
}
