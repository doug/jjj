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

    // Vertical split: main content and footer
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Main content
            Constraint::Length(2), // Footer (2 lines)
        ])
        .split(size);

    // Main layout: three columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Next Actions
            Constraint::Percentage(35), // Project Tree
            Constraint::Percentage(45), // Detail Pane
        ])
        .split(vertical_chunks[0]);

    draw_next_actions(f, app, main_chunks[0]);
    draw_project_tree(f, app, main_chunks[1]);
    draw_detail(f, app, main_chunks[2]);
    draw_footer(f, app, vertical_chunks[1]);
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

fn status_color_problem(status: &crate::models::ProblemStatus) -> Color {
    use crate::models::ProblemStatus;
    match status {
        ProblemStatus::Solved => Color::Green,
        ProblemStatus::InProgress => Color::Yellow,
        ProblemStatus::Dissolved => Color::DarkGray,
        ProblemStatus::Open => Color::White,
    }
}

fn status_color_solution(status: &crate::models::SolutionStatus) -> Color {
    use crate::models::SolutionStatus;
    match status {
        SolutionStatus::Accepted => Color::Green,
        SolutionStatus::Refuted => Color::Red,
        SolutionStatus::Testing => Color::Yellow,
        SolutionStatus::Proposed => Color::Cyan,
    }
}

fn status_color_critique(status: &crate::models::CritiqueStatus) -> Color {
    use crate::models::CritiqueStatus;
    match status {
        CritiqueStatus::Addressed | CritiqueStatus::Dismissed => Color::Green,
        CritiqueStatus::Valid => Color::Red,
        CritiqueStatus::Open => Color::Yellow,
    }
}

fn draw_project_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use super::tree::TreeNode;

    let is_focused = app.focused_pane == FocusedPane::ProjectTree;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.tree_items.iter().map(|item| {
        let indent = "  ".repeat(item.depth);
        let expand_char = if item.has_children {
            if item.node.is_expanded() { "▼ " } else { "▶ " }
        } else {
            "○ "
        };

        let (label, color) = match &item.node {
            TreeNode::Milestone { title, .. } => {
                (format!("{}{}{}", indent, expand_char, title), Color::Magenta)
            }
            TreeNode::Backlog { .. } => {
                (format!("{}{}Backlog", indent, expand_char), Color::DarkGray)
            }
            TreeNode::Problem { id, title, status, .. } => {
                (format!("{}{}{}: {}", indent, expand_char, id, title), status_color_problem(status))
            }
            TreeNode::Solution { id, title, status, .. } => {
                (format!("{}{}{}: {}", indent, expand_char, id, title), status_color_solution(status))
            }
            TreeNode::Critique { id, title, status, severity } => {
                (format!("{}○ {}: {} [{}]", indent, id, title, severity), status_color_critique(status))
            }
        };

        ListItem::new(Line::from(Span::styled(label, Style::default().fg(color))))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .title("Project Tree")
            .borders(Borders::ALL)
            .border_style(border_style))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.tree_items.is_empty() && app.tree_index < app.tree_items.len() {
        state.select(Some(app.tree_index));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let lines = app.selected_detail.to_lines();
    let text: Vec<Line> = lines.iter()
        .skip(app.detail_scroll as usize)
        .map(|s| Line::from(s.as_str()))
        .collect();

    let detail = Paragraph::new(text)
        .block(Block::default()
            .title("Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)))
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(detail, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Context line (top)
    let context = Paragraph::new(app.context_hints())
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(context, chunks[0]);

    // Global shortcuts (bottom)
    let global = Paragraph::new("[Tab] pane | [/] commands | [j/k] scroll | [?] help | [q] quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(global, chunks[1]);
}
