use super::app::{App, FocusedPane};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(35),
            Constraint::Percentage(45),
        ])
        .split(size);

    let next_actions_style = if app.focused_pane == FocusedPane::NextActions {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let next_actions = Paragraph::new("Next Actions placeholder")
        .block(Block::default()
            .title("Next Actions")
            .borders(Borders::ALL)
            .border_style(next_actions_style));
    f.render_widget(next_actions, main_chunks[0]);

    let tree_style = if app.focused_pane == FocusedPane::ProjectTree {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let tree = Paragraph::new("Project Tree placeholder")
        .block(Block::default()
            .title("Project Tree")
            .borders(Borders::ALL)
            .border_style(tree_style));
    f.render_widget(tree, main_chunks[1]);

    let detail = Paragraph::new("Detail placeholder")
        .block(Block::default()
            .title("Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(detail, main_chunks[2]);
}
