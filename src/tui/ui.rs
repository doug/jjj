use super::app::{App, FocusedPane};
use super::next_actions::Category;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main layout: three columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Next Actions
            Constraint::Percentage(35), // Project Tree
            Constraint::Percentage(45), // Detail Pane
        ])
        .split(size);

    draw_next_actions(f, app, main_chunks[0]);
    draw_project_tree(f, app, main_chunks[1]);
    draw_detail(f, app, main_chunks[2]);
}

fn category_color(cat: Category) -> Color {
    match cat {
        Category::Blocked => Color::Red,
        Category::Ready => Color::Green,
        Category::Review => Color::Cyan,
        Category::Waiting => Color::Yellow,
        Category::Todo => Color::White,
    }
}

fn draw_next_actions(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let is_focused = app.focused_pane == FocusedPane::NextActions;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.next_actions.iter().map(|action| {
        let cat_span = Span::styled(
            format!("[{}] ", action.category.label()),
            Style::default().fg(category_color(action.category)),
        );
        let id_span = Span::styled(
            format!("{}: ", action.entity_id),
            Style::default().fg(Color::DarkGray),
        );
        let title_span = Span::raw(&action.title);

        ListItem::new(Line::from(vec![cat_span, id_span, title_span]))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title("Next Actions")
            .borders(Borders::ALL)
            .border_style(border_style))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.next_actions.is_empty() {
        state.select(Some(app.next_actions_index));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_project_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let is_focused = app.focused_pane == FocusedPane::ProjectTree;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tree = Paragraph::new("Project Tree placeholder")
        .block(Block::default()
            .title("Project Tree")
            .borders(Borders::ALL)
            .border_style(border_style));
    f.render_widget(tree, area);
}

fn draw_detail(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let detail = Paragraph::new("Select an item to see details")
        .block(Block::default()
            .title("Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(detail, area);
}
