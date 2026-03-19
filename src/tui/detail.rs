use crate::models::{Critique, Milestone, Problem, Solution};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub enum DetailContent {
    None,
    Problem(Problem),
    Solution(Solution),
    Critique(Critique),
    Milestone(Milestone),
}

impl DetailContent {
    /// Get the status-based border color for the detail block.
    pub fn border_color(&self) -> Color {
        match self {
            DetailContent::None => Color::DarkGray,
            DetailContent::Problem(p) => super::ui::status_color_problem(&p.status),
            DetailContent::Solution(s) => super::ui::status_color_solution(&s.status),
            DetailContent::Critique(c) => super::ui::status_color_critique(&c.status),
            DetailContent::Milestone(m) => super::ui::status_color_milestone(&m.status),
        }
    }

    /// Get the block title (entity type name).
    pub fn block_title(&self) -> &'static str {
        match self {
            DetailContent::None => "Detail",
            DetailContent::Problem(_) => " Problem ",
            DetailContent::Solution(_) => " Solution ",
            DetailContent::Critique(_) => " Critique ",
            DetailContent::Milestone(_) => " Milestone ",
        }
    }

    pub fn to_styled_lines(&self) -> Vec<Line<'static>> {
        match self {
            DetailContent::None => vec![Line::from(Span::styled(
                "Select an item to see details",
                Style::default().fg(Color::DarkGray),
            ))],
            DetailContent::Problem(p) => problem_lines(p),
            DetailContent::Solution(s) => solution_lines(s),
            DetailContent::Critique(c) => critique_lines(c),
            DetailContent::Milestone(m) => milestone_lines(m),
        }
    }
}

fn problem_lines(p: &Problem) -> Vec<Line<'static>> {
    use super::markdown::markdown_to_line;

    let status_color = super::ui::status_color_problem(&p.status);
    let priority_sym = super::ui::priority_prefix(&p.priority);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}{}", priority_sym, p.title),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    lines.push(meta_line("Status", &p.status.to_string(), Some(status_color)));
    lines.push(meta_line(
        "Priority",
        &p.priority.to_string(),
        Some(super::ui::priority_color(&p.priority)),
    ));
    if let Some(assignee) = &p.assignee {
        let name = assignee.split('<').next().unwrap_or(assignee).trim();
        lines.push(meta_line("Assignee", name, None));
    }
    if let Some(milestone) = &p.milestone_id {
        lines.push(meta_line(
            "Milestone",
            &milestone[..8.min(milestone.len())],
            None,
        ));
    }
    if !p.tags.is_empty() {
        lines.push(tags_line(&p.tags));
    }
    lines.push(Line::from(""));
    lines.push(divider_line());

    if !p.description.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Description"));
        for text_line in p.description.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }

    if !p.context.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Context"));
        for text_line in p.context.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }

    lines.push(Line::from(""));
    lines
}

fn solution_lines(s: &Solution) -> Vec<Line<'static>> {
    use super::markdown::markdown_to_line;

    let status_color = super::ui::status_color_solution(&s.status);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", s.title),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    lines.push(meta_line(
        "Status",
        &s.status.to_string(),
        Some(status_color),
    ));
    lines.push(meta_line(
        "Problem",
        &s.problem_id[..8.min(s.problem_id.len())],
        None,
    ));
    if let Some(assignee) = &s.assignee {
        let name = assignee.split('<').next().unwrap_or(assignee).trim();
        lines.push(meta_line("Assignee", name, None));
    }
    if !s.change_ids.is_empty() {
        lines.push(meta_line("Changes", &s.change_ids.join(", "), None));
    }
    if !s.tags.is_empty() {
        lines.push(tags_line(&s.tags));
    }
    lines.push(Line::from(""));
    lines.push(divider_line());

    if !s.approach.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Approach"));
        for text_line in s.approach.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }
    if !s.tradeoffs.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Tradeoffs"));
        for text_line in s.tradeoffs.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }

    lines.push(Line::from(""));
    lines
}

fn critique_lines(c: &Critique) -> Vec<Line<'static>> {
    use super::markdown::markdown_to_line;

    let status_color = super::ui::status_color_critique(&c.status);
    let sev_color = super::ui::severity_color(&c.severity);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", c.title),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    lines.push(meta_line(
        "Status",
        &c.status.to_string(),
        Some(status_color),
    ));
    lines.push(meta_line(
        "Severity",
        &c.severity.to_string(),
        Some(sev_color),
    ));
    lines.push(meta_line(
        "Solution",
        &c.solution_id[..8.min(c.solution_id.len())],
        None,
    ));
    if let Some(file) = &c.file_path {
        let loc = format!("{}:{}", file, c.line_start.unwrap_or(0));
        lines.push(meta_line("Location", &loc, None));
    }
    lines.push(Line::from(""));
    lines.push(divider_line());

    if !c.argument.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Argument"));
        for text_line in c.argument.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }
    if !c.evidence.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Evidence"));
        for text_line in c.evidence.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }
    if !c.replies.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Replies"));
        for reply in &c.replies {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {} ", reply.author),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("({})", reply.created_at),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            for reply_line in reply.body.lines() {
                lines.push(indent_md(markdown_to_line(reply_line)));
            }
        }
    }

    lines.push(Line::from(""));
    lines
}

fn milestone_lines(m: &Milestone) -> Vec<Line<'static>> {
    use super::markdown::markdown_to_line;

    let status_color = super::ui::status_color_milestone(&m.status);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", m.title),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    lines.push(meta_line(
        "Status",
        &m.status.to_string(),
        Some(status_color),
    ));
    if let Some(date) = &m.target_date {
        lines.push(meta_line(
            "Target",
            &date.format("%Y-%m-%d").to_string(),
            None,
        ));
    }
    if let Some(assignee) = &m.assignee {
        let name = assignee.split('<').next().unwrap_or(assignee).trim();
        lines.push(meta_line("Assignee", name, None));
    }
    lines.push(Line::from(""));
    lines.push(divider_line());

    if !m.goals.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Goals"));
        for text_line in m.goals.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }
    if !m.success_criteria.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Success Criteria"));
        for text_line in m.success_criteria.lines() {
            lines.push(indent_md(markdown_to_line(text_line)));
        }
    }

    lines.push(Line::from(""));
    lines
}

/// Render a metadata key-value pair as a styled Line.
fn meta_line(label: &str, value: &str, value_color: Option<Color>) -> Line<'static> {
    let val_style = Style::default().fg(value_color.unwrap_or(Color::White));
    Line::from(vec![
        Span::styled(
            format!("  {:<12}", label),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), val_style),
    ])
}

/// Render tags as bracketed cyan chips.
fn tags_line(tags: &[String]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        "  Tags        ".to_string(),
        Style::default().fg(Color::DarkGray),
    )];
    for (i, tag) in tags.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("[{}]", tag),
            Style::default().fg(Color::Cyan),
        ));
    }
    Line::from(spans)
}

/// A thin horizontal divider line.
fn divider_line() -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", "\u{2500}".repeat(36)),
        Style::default().fg(Color::DarkGray),
    ))
}

/// A bold section header (e.g., "Description:").
fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}:", title),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Prepend indentation to a markdown-parsed line.
fn indent_md(line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw("    ".to_string())];
    spans.extend(line.spans);
    Line::from(spans)
}
