use crate::models::{Critique, Milestone, Problem, Solution};
use pulldown_cmark::{Event, Options as ParseOptions, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Rank and vote metadata for a problem, sourced from milestone rankings.
pub struct ProblemRankInfo {
    /// 1-indexed rank position within the milestone (1 = highest priority).
    pub rank: Option<usize>,
    /// Number of voters who ranked this problem.
    pub votes: u32,
    /// QV budget spent by the current user on this problem.
    pub budget_used: u32,
    /// Total QV budget for the milestone.
    pub budget_total: u32,
}

pub enum DetailContent {
    None,
    Problem(Problem, Option<ProblemRankInfo>),
    Solution(Solution),
    Critique(Critique),
    Milestone(Milestone),
}

impl DetailContent {
    /// Get the status-based border color for the detail block.
    pub fn border_color(&self) -> Color {
        match self {
            DetailContent::None => Color::DarkGray,
            DetailContent::Problem(p, _) => super::ui::status_color_problem(&p.status),
            DetailContent::Solution(s) => super::ui::status_color_solution(&s.status),
            DetailContent::Critique(c) => super::ui::status_color_critique(&c.status),
            DetailContent::Milestone(m) => super::ui::status_color_milestone(&m.status),
        }
    }

    /// Get the block title (entity type name).
    pub fn block_title(&self) -> &'static str {
        match self {
            DetailContent::None => "Detail",
            DetailContent::Problem(..) => " Problem ",
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
            DetailContent::Problem(p, rank_info) => problem_lines(p, rank_info.as_ref()),
            DetailContent::Solution(s) => solution_lines(s),
            DetailContent::Critique(c) => critique_lines(c),
            DetailContent::Milestone(m) => milestone_lines(m),
        }
    }
}

fn problem_lines(p: &Problem, rank_info: Option<&ProblemRankInfo>) -> Vec<Line<'static>> {
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

    lines.push(meta_line(
        "Status",
        &p.status.to_string(),
        Some(status_color),
    ));
    lines.push(meta_line(
        "Priority",
        &p.priority.to_string(),
        Some(super::ui::priority_color(&p.priority)),
    ));
    if !matches!(p.confidence, crate::models::Confidence::Unknown) {
        lines.push(meta_line(
            "Confidence",
            &p.confidence.to_string(),
            Some(super::ui::confidence_color(&p.confidence)),
        ));
    }
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
    if let Some(ri) = rank_info {
        if let Some(rank) = ri.rank {
            lines.push(meta_line(
                "Rank",
                &format!("#{}", rank),
                Some(Color::Yellow),
            ));
        }
        if ri.votes > 0 {
            let vote_str = if ri.budget_total > 0 {
                format!(
                    "{}\u{2605} (budget {}/{})",
                    ri.votes, ri.budget_used, ri.budget_total
                )
            } else {
                format!("{}\u{2605}", ri.votes)
            };
            lines.push(meta_line("Votes", &vote_str, Some(Color::Yellow)));
        }
    }
    lines.push(Line::from(""));
    lines.push(divider_line());

    if !p.description.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Description"));
        lines.extend(render_md_body(&p.description));
    }

    if !p.context.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Context"));
        lines.extend(render_md_body(&p.context));
    }

    lines.push(Line::from(""));
    lines
}

fn solution_lines(s: &Solution) -> Vec<Line<'static>> {
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
        lines.extend(render_md_body(&s.approach));
    }
    if !s.tradeoffs.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Tradeoffs"));
        lines.extend(render_md_body(&s.tradeoffs));
    }

    lines.push(Line::from(""));
    lines
}

fn critique_lines(c: &Critique) -> Vec<Line<'static>> {
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
        lines.extend(render_md_body(&c.argument));
    }
    if !c.evidence.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Evidence"));
        lines.extend(render_md_body(&c.evidence));
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
            lines.extend(render_md_body(&reply.body));
        }
    }

    lines.push(Line::from(""));
    lines
}

fn milestone_lines(m: &Milestone) -> Vec<Line<'static>> {
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
        lines.extend(render_md_body(&m.goals));
    }
    if !m.success_criteria.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("Success Criteria"));
        lines.extend(render_md_body(&m.success_criteria));
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

/// Render a markdown body block into indented, styled lines using pulldown-cmark.
fn render_md_body(text: &str) -> Vec<Line<'static>> {
    let mut opts = ParseOptions::empty();
    opts.insert(ParseOptions::ENABLE_STRIKETHROUGH);
    opts.insert(ParseOptions::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(text, opts);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = Vec::new();
    let mut list_depth: usize = 0;
    let mut list_indices: Vec<Option<u64>> = Vec::new();

    let current_style =
        |stack: &[Style]| -> Style { stack.iter().fold(Style::default(), |acc, s| acc.patch(*s)) };

    let flush_line = |lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>| {
        let mut result = vec![Span::raw("    ".to_string())];
        result.append(spans);
        lines.push(Line::from(result));
    };

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { .. } => {
                    style_stack.push(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    );
                }
                Tag::Emphasis => {
                    style_stack.push(Style::default().add_modifier(Modifier::ITALIC));
                }
                Tag::Strong => {
                    style_stack.push(Style::default().add_modifier(Modifier::BOLD));
                }
                Tag::Strikethrough => {
                    style_stack.push(Style::default().add_modifier(Modifier::CROSSED_OUT));
                }
                Tag::BlockQuote(_) => {
                    style_stack.push(Style::default().fg(Color::DarkGray));
                }
                Tag::CodeBlock(_) => {
                    style_stack.push(Style::default().fg(Color::Cyan).bg(Color::DarkGray));
                }
                Tag::List(start) => {
                    list_depth += 1;
                    list_indices.push(start);
                }
                Tag::Item => {
                    if !current_spans.is_empty() {
                        flush_line(&mut lines, &mut current_spans);
                    }
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    if let Some(idx) = list_indices.last_mut() {
                        match idx {
                            Some(n) => {
                                current_spans.push(Span::styled(
                                    format!("{}{n}. ", indent),
                                    Style::default().fg(Color::DarkGray),
                                ));
                                *n += 1;
                            }
                            None => {
                                current_spans.push(Span::styled(
                                    format!("{}\u{2022} ", indent),
                                    Style::default().fg(Color::DarkGray),
                                ));
                            }
                        }
                    }
                }
                Tag::Link { .. } => {
                    style_stack.push(Style::default().fg(Color::Blue));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Heading(_) => {
                    flush_line(&mut lines, &mut current_spans);
                    if matches!(tag, TagEnd::Heading(_)) {
                        style_stack.pop();
                    }
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                    style_stack.pop();
                }
                TagEnd::BlockQuote(_) => {
                    style_stack.pop();
                }
                TagEnd::CodeBlock => {
                    style_stack.pop();
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    list_indices.pop();
                }
                TagEnd::Item => {
                    if !current_spans.is_empty() {
                        flush_line(&mut lines, &mut current_spans);
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                let style = current_style(&style_stack);
                current_spans.push(Span::styled(text.to_string(), style));
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::Cyan).bg(Color::DarkGray),
                ));
            }
            Event::SoftBreak => {
                current_spans.push(Span::raw(" ".to_string()));
            }
            Event::HardBreak => {
                flush_line(&mut lines, &mut current_spans);
            }
            Event::Rule => {
                flush_line(&mut lines, &mut current_spans);
                lines.push(Line::from(Span::styled(
                    format!("    {}", "\u{2500}".repeat(30)),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "\u{2611} " } else { "\u{2610} " };
                current_spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(if checked {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ));
            }
            _ => {}
        }
    }

    // Flush any remaining spans
    if !current_spans.is_empty() {
        flush_line(&mut lines, &mut current_spans);
    }

    lines
}
