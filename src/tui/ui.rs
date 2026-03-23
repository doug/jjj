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
        InputMode::Input {
            prompt,
            buffer,
            cursor_pos,
            ..
        } => {
            draw_input_line(f, prompt, buffer, *cursor_pos, vertical_chunks[1]);
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

pub(super) fn status_color_problem(status: &crate::models::ProblemStatus) -> Color {
    use crate::models::ProblemStatus;
    match status {
        ProblemStatus::Solved => Color::Green,
        ProblemStatus::InProgress => Color::Yellow,
        ProblemStatus::Dissolved => Color::DarkGray,
        ProblemStatus::Open => Color::White,
    }
}

pub(super) fn status_color_solution(status: &crate::models::SolutionStatus) -> Color {
    use crate::models::SolutionStatus;
    match status {
        SolutionStatus::Approved => Color::Green,
        SolutionStatus::Withdrawn => Color::Red,
        SolutionStatus::Submitted => Color::Yellow,
        SolutionStatus::Proposed => Color::Cyan,
    }
}

pub(super) fn status_color_critique(status: &crate::models::CritiqueStatus) -> Color {
    use crate::models::CritiqueStatus;
    match status {
        CritiqueStatus::Addressed | CritiqueStatus::Dismissed => Color::Green,
        CritiqueStatus::Valid => Color::Red,
        CritiqueStatus::Open => Color::Yellow,
    }
}

pub(super) fn status_color_milestone(status: &crate::models::MilestoneStatus) -> Color {
    use crate::models::MilestoneStatus;
    match status {
        MilestoneStatus::Completed => Color::Green,
        MilestoneStatus::Active => Color::Yellow,
        MilestoneStatus::Cancelled => Color::Red,
        MilestoneStatus::Planning => Color::Cyan,
    }
}

pub(super) fn severity_color(severity: &crate::models::CritiqueSeverity) -> Color {
    use crate::models::CritiqueSeverity;
    match severity {
        CritiqueSeverity::Critical => Color::Red,
        CritiqueSeverity::High => Color::Yellow,
        CritiqueSeverity::Medium => Color::White,
        CritiqueSeverity::Low => Color::DarkGray,
    }
}

pub(super) fn priority_prefix(priority: &Priority) -> &'static str {
    match priority {
        Priority::Critical => "🔴 ",
        Priority::High => "🟡 ",
        Priority::Medium | Priority::Low => "",
    }
}

pub(super) fn priority_color(priority: &Priority) -> Color {
    match priority {
        Priority::Critical => Color::Red,
        Priority::High => Color::Yellow,
        Priority::Medium => Color::White,
        Priority::Low => Color::DarkGray,
    }
}

pub(super) fn confidence_color(confidence: &crate::models::Confidence) -> Color {
    use crate::models::Confidence;
    match confidence {
        Confidence::Red => Color::Red,
        Confidence::Amber => Color::Yellow,
        Confidence::Green => Color::Green,
        Confidence::Unknown => Color::DarkGray,
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

    let cursor_id = app
        .cache
        .tree_items
        .get(app.ui.tree_index)
        .map(|i| i.node.id().to_string());

    let items: Vec<ListItem> = display_items
        .iter()
        .map(|item| {
            let is_selected = app.ui.selected_ids.contains(item.node.id());
            let is_cursor = cursor_id.as_deref() == Some(item.node.id());
            let indent = "  ".repeat(item.depth);

            // Action symbol (if any)
            let action_sym = item.action_symbol.as_deref().unwrap_or("");

            let (label, color, dim) = match &item.node {
                TreeNode::ProjectRoot { .. } => (
                    format!("{}Root", indent),
                    Color::White,
                    false,
                ),
                TreeNode::Milestone { title, .. } => (
                    format!("{}{}", indent, title),
                    Color::Magenta,
                    false,
                ),
                TreeNode::Backlog { .. } => (
                    format!("{}Backlog", indent),
                    Color::DarkGray,
                    false,
                ),
                TreeNode::Problem {
                    title,
                    status,
                    assignee,
                    rank,
                    ..
                } => {
                    let rank_prefix = rank.map(|r| format!("#{} ", r)).unwrap_or_default();
                    let assignee_suffix = assignee
                        .as_deref()
                        .map(|a| {
                            // Extract name from "Name <email>" format
                            let name = a.split('<').next().unwrap_or(a).trim();
                            let name = if name.len() > 12 { &name[..12] } else { name };
                            format!(" @{}", name)
                        })
                        .unwrap_or_default();
                    (
                        format!(
                            "{}{}{}{}{}",
                            indent,
                            action_sym,
                            rank_prefix,
                            title,
                            assignee_suffix
                        ),
                        status_color_problem(status),
                        false,
                    )
                }
                TreeNode::Solution {
                    title,
                    status,
                    assignee,
                    ..
                } => {
                    let assignee_suffix = assignee
                        .as_deref()
                        .map(|a| {
                            let name = a.split('<').next().unwrap_or(a).trim();
                            let name = if name.len() > 12 { &name[..12] } else { name };
                            format!(" @{}", name)
                        })
                        .unwrap_or_default();
                    (
                        format!(
                            "{}{}{}{}",
                            indent,
                            action_sym,
                            title,
                            assignee_suffix
                        ),
                        status_color_solution(status),
                        false,
                    )
                }
                TreeNode::Critique {
                    title,
                    status,
                    severity,
                    ..
                } => (
                    format!(
                        "{}{}{} [{}]",
                        indent,
                        action_sym,
                        title,
                        severity
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

            // Add bold modifier for selected items
            let style = if is_selected {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };

            // Gutter: cursor gets "> ", selected gets "> " (persistent), else "  "
            let gutter = if is_cursor || is_selected { "> " } else { "  " };
            let gutter_style = if is_cursor {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let mut spans = vec![
                Span::styled(gutter, gutter_style),
                Span::styled(label, style),
            ];
            // Add vote stars for problems with votes
            if let TreeNode::Problem { votes, .. } = &item.node {
                if *votes > 0 {
                    spans.push(Span::styled(
                        format!(" {}", "★".repeat(*votes as usize)),
                        Style::default().fg(Color::Yellow),
                    ));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

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

    let lines = app.cache.selected_detail.to_styled_lines();
    let border_color = app.cache.selected_detail.border_color();
    let title = app.cache.selected_detail.block_title();

    let text: Vec<Line> = lines
        .into_iter()
        .skip(app.ui.detail_scroll as usize)
        .collect();

    let detail = Paragraph::new(text)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
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

fn draw_input_line(f: &mut Frame, prompt: &str, buffer: &str, cursor_pos: usize, area: Rect) {
    // First line: prompt and input with cursor
    let input_area = Rect::new(area.x, area.y, area.width, 1);

    let prompt_span = Span::styled(prompt, Style::default().fg(Color::Yellow));

    let clamped_pos = cursor_pos.min(buffer.len());
    let before_cursor = &buffer[..clamped_pos];
    let after_cursor = if clamped_pos < buffer.len() {
        &buffer[clamped_pos + 1..]
    } else {
        ""
    };
    let cursor_char = if clamped_pos < buffer.len() {
        &buffer[clamped_pos..clamped_pos + 1]
    } else {
        "█"
    };

    let before_span = Span::styled(
        before_cursor,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let cursor_span = Span::styled(
        cursor_char,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let after_span = Span::styled(
        after_cursor,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let line = Line::from(vec![prompt_span, before_span, cursor_span, after_span]);
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
    let selection_info = if !app.ui.selected_ids.is_empty() {
        format!("[{} selected] ", app.ui.selected_ids.len())
    } else {
        String::new()
    };

    let context_text = if let Some((msg, _)) = &app.ui.flash_message {
        msg.clone()
    } else if let Some(ref filter) = app.ui.search_filter {
        format!("{}[/{}] {}", selection_info, filter, app.context_hints())
    } else {
        format!("{}{}", selection_info, app.context_hints())
    };
    let context_style = if app.ui.flash_message.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let context = Paragraph::new(context_text).style(context_style);
    f.render_widget(context, chunks[0]);

    // Global shortcuts (bottom)
    let global = Paragraph::new(
        "[Space] select | [Tab] next action | [R] related | [j/k] scroll | [?] help | [q] quit",
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(global, chunks[1]);
}

fn draw_help_overlay(f: &mut Frame, app: &App) {
    let area = f.area();

    // Calculate centered popup, clamped to terminal size
    let popup_width = 46u16.min(area.width);
    let popup_height = 30u16.min(area.height);
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
        Line::from("    S+\u{2191}/\u{2193}   Reorder problem"),
        Line::from("    S+\u{2190}/\u{2192}   Tier drill in/out"),
        Line::from("    +/-     Add/remove vote"),
        Line::from("    r       Toggle personal/global"),
        Line::from("    R       Toggle related"),
        Line::from(""),
        Line::from(Span::styled(
            "  Selection",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    Space   Toggle select + move down"),
        Line::from("    Ctrl+A  Select all / deselect all"),
        Line::from("    Esc     Clear selection"),
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
            TreeNode::Milestone { .. } => Some(EntityType::Milestone),
            TreeNode::ProjectRoot { .. } | TreeNode::Backlog { .. } => None,
        });

    match entity_type {
        Some(EntityType::Problem) => {
            lines.push(Line::from("    n       New solution"));
            lines.push(Line::from("    c       Cycle confidence (RAG)"));
            lines.push(Line::from("    s       Mark solved"));
            lines.push(Line::from("    d       Dissolve (with reason)"));
            lines.push(Line::from("    o       Reopen"));
            lines.push(Line::from("    A       Assign to me"));
            lines.push(Line::from("    m       Move to milestone"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    t       Edit tags"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
            lines.push(Line::from("    x       Delete"));
        }
        Some(EntityType::Solution) => {
            lines.push(Line::from("    n       New critique"));
            lines.push(Line::from("    u       Submit for review"));
            lines.push(Line::from("    a       Approve"));
            lines.push(Line::from("    d       Withdraw"));
            lines.push(Line::from("    A       Assign to me"));
            lines.push(Line::from("    g       Go to change"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    t       Edit tags"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
            lines.push(Line::from("    x       Delete"));
        }
        Some(EntityType::Critique) => {
            lines.push(Line::from("    a       Address"));
            lines.push(Line::from("    d       Dismiss"));
            lines.push(Line::from("    v       Validate"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
            lines.push(Line::from("    x       Delete"));
        }
        Some(EntityType::Milestone) => {
            lines.push(Line::from("    n       New problem"));
            lines.push(Line::from("    s       Mark completed"));
            lines.push(Line::from("    d       Cancel"));
            lines.push(Line::from("    o       Activate"));
            lines.push(Line::from("    A       Assign to me"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
            lines.push(Line::from("    x       Delete"));
        }
        None => {
            // ProjectRoot or Backlog
            lines.push(Line::from("    n       New (milestone/problem)"));
        }
    }

    lines
}
