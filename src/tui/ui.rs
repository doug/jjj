use super::app::{App, InputMode};
use crate::models::Priority;
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

    // Main layout: two columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Project Tree
            Constraint::Percentage(60), // Detail Pane
        ])
        .split(vertical_chunks[0]);

    draw_project_tree(f, app, main_chunks[0]);
    draw_detail(f, app, main_chunks[1]);

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
        SolutionStatus::Review => Color::Yellow,
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

fn priority_prefix(priority: &Priority) -> &'static str {
    match priority {
        Priority::Critical => "🔴 ",
        Priority::High => "🟡 ",
        Priority::Medium | Priority::Low => "",
    }
}

fn draw_project_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use super::tree::TreeNode;

    // Apply filter if enabled
    let display_items: Vec<_> = if app.ui.filter_actions_only {
        super::filter_tree_to_actions(&app.cache.tree_items)
    } else {
        app.cache.tree_items.clone()
    };

    // Tree is always focused now (single-pane navigation)
    let border_style = Style::default().fg(Color::Cyan);

    // Update title based on filter mode
    let title = if app.ui.filter_actions_only {
        "Project Tree [Actions]"
    } else {
        "Project Tree"
    };

    let items: Vec<ListItem> = display_items
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
                "  " // Changed from "○ " to align better
            };

            // Action symbol (if any)
            let action_sym = item.action_symbol.as_deref().unwrap_or("");

            let (label, color, dim) = match &item.node {
                TreeNode::Milestone { title, .. } => (
                    format!("{}{}{}", indent, expand_char, title),
                    Color::Magenta,
                    false,
                ),
                TreeNode::Backlog { .. } => (
                    format!("{}{}Backlog", indent, expand_char),
                    Color::DarkGray,
                    false,
                ),
                TreeNode::Problem {
                    id,
                    title,
                    status,
                    priority,
                    ..
                } => {
                    let priority_sym = priority_prefix(priority);
                    let dim = matches!(priority, Priority::Low);
                    (
                        format!(
                            "{}{}{}{}{}: {}",
                            indent, expand_char, priority_sym, action_sym, id, title
                        ),
                        status_color_problem(status),
                        dim,
                    )
                }
                TreeNode::Solution {
                    id, title, status, ..
                } => (
                    format!("{}{}{}{}: {}", indent, expand_char, action_sym, id, title),
                    status_color_solution(status),
                    false,
                ),
                TreeNode::Critique {
                    id,
                    title,
                    status,
                    severity,
                } => (
                    format!(
                        "{}{}{}{}: {} [{}]",
                        indent, expand_char, action_sym, id, title, severity
                    ),
                    status_color_critique(status),
                    false,
                ),
            };

            let style = if dim {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(color)
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    // Find selection in display items by matching ID
    let selected_id = app
        .cache
        .tree_items
        .get(app.ui.tree_index)
        .map(|i| i.node.id());

    let display_index =
        selected_id.and_then(|id| display_items.iter().position(|i| i.node.id() == id));

    let mut state = ListState::default();
    if let Some(idx) = display_index {
        state.select(Some(idx));
    } else if !display_items.is_empty() {
        state.select(Some(0));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Show related panel when there are results or a load is in-flight
    let show_related =
        app.ui.show_related && (!app.ui.related_items.is_empty() || app.ui.related_rx.is_some());

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
    let is_loading = app.ui.related_rx.is_some();

    let items: Vec<ListItem> = if is_loading && app.ui.related_items.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.ui
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
            .collect()
    };

    let title = if is_loading {
        "Related [loading...] [R to toggle]"
    } else {
        "Related [R to toggle]"
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(list, area);
}

fn draw_input_line(f: &mut Frame, prompt: &str, buffer: &str, area: Rect) {
    // First line: prompt and input
    let input_area = Rect::new(area.x, area.y, area.width, 1);

    let prompt_span = Span::styled(prompt, Style::default().fg(Color::Yellow));
    let buffer_span = Span::styled(
        buffer,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let cursor_span = Span::styled("█", Style::default().fg(Color::Cyan));

    let line = Line::from(vec![prompt_span, buffer_span, cursor_span]);
    let input = Paragraph::new(line);
    f.render_widget(input, input_area);

    // Second line: hint
    let hint =
        Paragraph::new("[Enter] submit | [Esc] cancel").style(Style::default().fg(Color::DarkGray));
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
    } else if let Some(ref filter) = app.ui.search_filter {
        format!("[/{}] {}", filter, app.context_hints())
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
    let global =
        Paragraph::new("[Tab] next action | [R] related | [j/k] scroll | [?] help | [q] quit")
            .style(Style::default().fg(Color::DarkGray));
    f.render_widget(global, chunks[1]);
}

fn draw_help_overlay(f: &mut Frame, app: &App) {
    let area = f.area();

    // Calculate centered popup (40 wide, 19 tall)
    let popup_width = 40u16;
    let popup_height = 19u16;
    let popup_x = area.width.saturating_sub(popup_width) / 2;
    let popup_y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Build help text based on context
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    ↑/↓     Move selection"),
        Line::from("    ←/→     Collapse/Expand"),
        Line::from("    Tab     Jump to next action"),
        Line::from("    S-Tab   Jump to prev action"),
        Line::from("    /       Search/filter tree"),
        Line::from("    f       Toggle filter (full/actions)"),
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

    let help = Paragraph::new(lines).block(
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

    let mut lines = vec![Line::from(Span::styled(
        "  Actions",
        Style::default().add_modifier(Modifier::BOLD),
    ))];

    // Determine what's selected from tree
    let entity_type = app
        .cache
        .tree_items
        .get(app.ui.tree_index)
        .and_then(|item| match &item.node {
            TreeNode::Problem { .. } => Some(EntityType::Problem),
            TreeNode::Solution { .. } => Some(EntityType::Solution),
            TreeNode::Critique { .. } => Some(EntityType::Critique),
            TreeNode::Milestone { .. } | TreeNode::Backlog { .. } => None,
        });

    match entity_type {
        Some(EntityType::Problem) => {
            lines.push(Line::from("    n       New solution"));
            lines.push(Line::from("    s       Mark solved"));
            lines.push(Line::from("    o       Reopen"));
            lines.push(Line::from("    d       Dissolve"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
        Some(EntityType::Solution) => {
            lines.push(Line::from("    n       New critique"));
            lines.push(Line::from("    a       Accept"));
            lines.push(Line::from("    r       Refute"));
            lines.push(Line::from("    g       Go to change"));
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
