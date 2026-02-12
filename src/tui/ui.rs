use super::app::{App, FocusedPane, InputMode};
use super::next_actions::Category;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
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

    // Draw footer or input line
    match &app.ui.input_mode {
        InputMode::Input { prompt, buffer, .. } => {
            draw_input_line(f, prompt, buffer, vertical_chunks[1]);
        }
        _ => {
            draw_footer(f, app, vertical_chunks[1]);
        }
    }

    // Draw overlays last (on top)
    if matches!(app.ui.input_mode, InputMode::Help) {
        draw_help_overlay(f, app);
    }
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
    let is_focused = app.ui.focused_pane == FocusedPane::NextActions;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .cache
        .next_actions
        .iter()
        .map(|action| {
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
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Next Actions")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.cache.next_actions.is_empty() {
        state.select(Some(app.ui.next_actions_index));
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

    let is_focused = app.ui.focused_pane == FocusedPane::ProjectTree;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .cache
        .tree_items
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.depth);
            let expand_char = if item.has_children {
                if item.node.is_expanded() {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "○ "
            };

            let (label, color) = match &item.node {
                TreeNode::Milestone { title, .. } => (
                    format!("{}{}{}", indent, expand_char, title),
                    Color::Magenta,
                ),
                TreeNode::Backlog { .. } => {
                    (format!("{}{}Backlog", indent, expand_char), Color::DarkGray)
                }
                TreeNode::Problem {
                    id, title, status, ..
                } => (
                    format!("{}{}{}: {}", indent, expand_char, id, title),
                    status_color_problem(status),
                ),
                TreeNode::Solution {
                    id, title, status, ..
                } => (
                    format!("{}{}{}: {}", indent, expand_char, id, title),
                    status_color_solution(status),
                ),
                TreeNode::Critique {
                    id,
                    title,
                    status,
                    severity,
                } => (
                    format!("{}○ {}: {} [{}]", indent, id, title, severity),
                    status_color_critique(status),
                ),
            };

            ListItem::new(Line::from(Span::styled(label, Style::default().fg(color))))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Project Tree")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.cache.tree_items.is_empty() && app.ui.tree_index < app.cache.tree_items.len() {
        state.select(Some(app.ui.tree_index));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Decide if we should show the related panel
    let show_related = app.ui.show_related && !app.ui.related_items.is_empty();

    // Split area if showing related panel
    let (detail_area, related_area) = if show_related {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Detail content
                Constraint::Length(7), // Related panel (5 items + 2 for border)
            ])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let lines = app.cache.selected_detail.to_lines();
    let text: Vec<Line> = lines
        .iter()
        .skip(app.ui.detail_scroll as usize)
        .map(|s| Line::from(s.as_str()))
        .collect();

    let detail = Paragraph::new(text)
        .block(
            Block::default()
                .title("Detail")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(detail, detail_area);

    // Draw related panel if enabled
    if let Some(related_area) = related_area {
        draw_related_panel(f, app, related_area);
    }
}

fn draw_related_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .ui
        .related_items
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == app.ui.related_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            let short_id = &r.entity_id[..6.min(r.entity_id.len())];
            let type_char = r.entity_type.chars().next().unwrap_or('?');
            ListItem::new(Line::from(Span::styled(
                format!(
                    "{}/{}  [{:.2}]  {}",
                    type_char, short_id, r.similarity, r.title
                ),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Related [R to toggle]")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(list, area);
}

fn draw_input_line(f: &mut Frame, prompt: &str, buffer: &str, area: Rect) {
    // Single line input at bottom
    let input_area = Rect::new(area.x, area.y, area.width, 2);

    let prompt_span = Span::styled(prompt, Style::default().fg(Color::DarkGray));
    let buffer_span = Span::styled(buffer, Style::default().fg(Color::White));
    let cursor_span = Span::styled("█", Style::default().fg(Color::Cyan));

    let line = Line::from(vec![prompt_span, buffer_span, cursor_span]);

    let input = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(input, input_area);

    // Show hint below
    let hint = Paragraph::new("[Enter] submit | [Esc] cancel")
        .style(Style::default().fg(Color::DarkGray));
    let hint_area = Rect::new(area.x, area.y + 1, area.width, 1);
    f.render_widget(hint, hint_area);
}

fn draw_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Context line (top) - or flash message if present
    let context_text = if let Some((msg, _)) = &app.ui.flash_message {
        msg.clone()
    } else {
        app.context_hints()
    };
    let context_style = if app.ui.flash_message.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let context = Paragraph::new(context_text).style(context_style);
    f.render_widget(context, chunks[0]);

    // Global shortcuts (bottom)
    let global = Paragraph::new("[Tab] pane | [R] related | [j/k] scroll | [?] help | [q] quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(global, chunks[1]);
}

fn draw_help_overlay(f: &mut Frame, app: &App) {
    let area = f.area();

    // Calculate centered popup (40 wide, 18 tall)
    let popup_width = 40u16;
    let popup_height = 18u16;
    let popup_x = area.width.saturating_sub(popup_width) / 2;
    let popup_y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Build help text based on context
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("    ↑/↓     Move selection"),
        Line::from("    ←/→     Collapse/Expand"),
        Line::from("    Tab     Switch pane"),
        Line::from("    j/k     Scroll detail"),
        Line::from("    R       Toggle related"),
        Line::from(""),
    ];

    // Context-sensitive actions
    let action_lines = get_context_actions(app);
    lines.extend(action_lines);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press any key to close",
        Style::default().fg(Color::DarkGray),
    )));

    // Clear the area and draw popup
    f.render_widget(Clear, popup_area);

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(help, popup_area);
}

fn get_context_actions(app: &App) -> Vec<Line<'static>> {
    use super::next_actions::EntityType;
    use super::tree::TreeNode;

    let mut lines = vec![
        Line::from(Span::styled("  Actions", Style::default().add_modifier(Modifier::BOLD))),
    ];

    // Determine what's selected
    let entity_type = match app.ui.focused_pane {
        FocusedPane::NextActions => {
            app.cache.next_actions.get(app.ui.next_actions_index)
                .map(|a| a.entity_type)
        }
        FocusedPane::ProjectTree => {
            app.cache.tree_items.get(app.ui.tree_index)
                .and_then(|item| match &item.node {
                    TreeNode::Problem { .. } => Some(EntityType::Problem),
                    TreeNode::Solution { .. } => Some(EntityType::Solution),
                    TreeNode::Critique { .. } => Some(EntityType::Critique),
                    TreeNode::Milestone { .. } | TreeNode::Backlog { .. } => None,
                })
        }
    };

    match entity_type {
        Some(EntityType::Problem) => {
            lines.push(Line::from("    n       New solution"));
            lines.push(Line::from("    s       Mark solved"));
            lines.push(Line::from("    d       Dissolve"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
        Some(EntityType::Solution) => {
            lines.push(Line::from("    n       New critique"));
            lines.push(Line::from("    a       Accept"));
            lines.push(Line::from("    r       Refute"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
        Some(EntityType::Critique) => {
            lines.push(Line::from("    a       Address"));
            lines.push(Line::from("    d       Dismiss"));
            lines.push(Line::from("    v       Validate"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
        None => {
            // Milestone or Backlog
            lines.push(Line::from("    n       New problem"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
    }

    lines
}
