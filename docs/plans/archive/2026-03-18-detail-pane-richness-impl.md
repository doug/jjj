# Detail Pane Richness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Replace the plain-text detail pane with styled, color-themed rendering including markdown support for all four entity types.

**Architecture:** `detail.rs` switches from `Vec<String>` to `Vec<Line<'static>>` via a new `to_styled_lines()` method that builds Ratatui spans with colors/modifiers. A new `src/tui/markdown.rs` module provides line-by-line markdown-to-spans parsing. `ui.rs` consumes styled lines directly and colors the detail block border by entity status.

**Tech Stack:** Ratatui (Line, Span, Style, Color, Modifier), existing model types (Problem, Solution, Critique, Milestone)

---

### Task 1: Create markdown parser module

**Files:**
- Create: `src/tui/markdown.rs`
- Modify: `src/tui/mod.rs` (add `mod markdown;`)

**Step 1: Write unit tests for markdown parser**

Create `src/tui/markdown.rs` with tests at the bottom:

```rust
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Convert a single line of text to a styled Ratatui Line.
///
/// Supports: `**bold**`, `__bold__`, `` `code` ``, `# headers`,
/// `- list items`, `> blockquotes`. No nested formatting.
pub fn markdown_to_line(text: &str) -> Line<'static> {
    // stub — will implement in step 3
    Line::from(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let line = markdown_to_line("hello world");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "hello world");
    }

    #[test]
    fn test_bold_double_star() {
        let line = markdown_to_line("before **bold** after");
        assert!(line.spans.len() >= 3);
        assert_eq!(line.spans[1].content.as_ref(), "bold");
        assert!(line.spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_bold_double_underscore() {
        let line = markdown_to_line("before __bold__ after");
        assert!(line.spans.len() >= 3);
        assert_eq!(line.spans[1].content.as_ref(), "bold");
        assert!(line.spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_inline_code() {
        let line = markdown_to_line("run `cargo test` now");
        assert!(line.spans.len() >= 3);
        assert_eq!(line.spans[1].content.as_ref(), "cargo test");
        assert_eq!(line.spans[1].style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_header() {
        let line = markdown_to_line("# My Header");
        assert_eq!(line.spans[0].content.as_ref(), "My Header");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_subheader() {
        let line = markdown_to_line("## Sub Header");
        assert_eq!(line.spans[0].content.as_ref(), "Sub Header");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_list_item_dash() {
        let line = markdown_to_line("- list item");
        let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(full_text.contains("list item"));
    }

    #[test]
    fn test_list_item_star() {
        let line = markdown_to_line("* list item");
        let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(full_text.contains("list item"));
    }

    #[test]
    fn test_blockquote() {
        let line = markdown_to_line("> quoted text");
        assert!(line.spans[0]
            .style
            .add_modifier
            .contains(Modifier::ITALIC));
    }

    #[test]
    fn test_empty_string() {
        let line = markdown_to_line("");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "");
    }

    #[test]
    fn test_multiple_bold_segments() {
        let line = markdown_to_line("**a** and **b**");
        assert!(line.spans.len() >= 4);
        assert_eq!(line.spans[0].content.as_ref(), "a");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test markdown --lib`
Expected: Most tests FAIL (stub returns unstyled single span)

**Step 3: Implement markdown_to_line**

Replace the stub with the real implementation:

```rust
pub fn markdown_to_line(text: &str) -> Line<'static> {
    // Handle empty lines
    if text.is_empty() {
        return Line::from(String::new());
    }

    // Headers: "# " or "## " or "### " at start of line
    if let Some(rest) = text.strip_prefix("# ").or_else(|| text.strip_prefix("## ")).or_else(|| text.strip_prefix("### ")) {
        return Line::from(Span::styled(
            rest.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ));
    }

    // Blockquote: "> " at start
    if let Some(rest) = text.strip_prefix("> ") {
        return Line::from(Span::styled(
            format!("  {}", rest),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
    }

    // List items: "- " or "* " at start
    if let Some(rest) = text.strip_prefix("- ").or_else(|| text.strip_prefix("* ")) {
        let mut spans = vec![Span::styled(
            "  \u{2022} ".to_string(),
            Style::default().fg(Color::DarkGray),
        )];
        spans.extend(parse_inline_spans(rest));
        return Line::from(spans);
    }

    // Regular line: parse inline formatting
    Line::from(parse_inline_spans(text))
}

/// Parse inline bold (`**...**`, `__...__`) and code (`` `...` ``) markers.
fn parse_inline_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest inline marker
        let bold_star = remaining.find("**");
        let bold_under = remaining.find("__");
        let code = remaining.find('`');

        // Pick the earliest marker
        let earliest = [bold_star, bold_under, code.map(|i| {
            // Only count backtick if not escaped or part of ``
            i
        })]
            .into_iter()
            .flatten()
            .min();

        match earliest {
            None => {
                // No more markers — emit rest as plain text
                if !remaining.is_empty() {
                    spans.push(Span::raw(remaining.to_string()));
                }
                break;
            }
            Some(pos) => {
                // Emit text before the marker
                if pos > 0 {
                    spans.push(Span::raw(remaining[..pos].to_string()));
                }

                let after = &remaining[pos..];

                if after.starts_with("**") {
                    if let Some(end) = after[2..].find("**") {
                        let content = &after[2..2 + end];
                        spans.push(Span::styled(
                            content.to_string(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ));
                        remaining = &after[2 + end + 2..];
                    } else {
                        spans.push(Span::raw(after[..2].to_string()));
                        remaining = &after[2..];
                    }
                } else if after.starts_with("__") {
                    if let Some(end) = after[2..].find("__") {
                        let content = &after[2..2 + end];
                        spans.push(Span::styled(
                            content.to_string(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ));
                        remaining = &after[2 + end + 2..];
                    } else {
                        spans.push(Span::raw(after[..2].to_string()));
                        remaining = &after[2..];
                    }
                } else if after.starts_with('`') {
                    if let Some(end) = after[1..].find('`') {
                        let content = &after[1..1 + end];
                        spans.push(Span::styled(
                            content.to_string(),
                            Style::default().fg(Color::Cyan).bg(Color::DarkGray),
                        ));
                        remaining = &after[1 + end + 1..];
                    } else {
                        spans.push(Span::raw(after[..1].to_string()));
                        remaining = &after[1..];
                    }
                } else {
                    // Shouldn't happen, but safety fallback
                    spans.push(Span::raw(after[..1].to_string()));
                    remaining = &after[1..];
                }
            }
        }
    }

    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }
    spans
}
```

**Step 4: Register the module**

In `src/tui/mod.rs`, add after `mod detail;`:

```rust
mod markdown;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test markdown --lib`
Expected: All PASS

**Step 6: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: Clean

**Step 7: Commit**

```bash
git add src/tui/markdown.rs src/tui/mod.rs
git commit -m "feat(tui): add line-by-line markdown parser for detail pane"
```

---

### Task 2: Add status_color_milestone and make color functions public

**Files:**
- Modify: `src/tui/ui.rs:62-97` (change `fn` to `pub(super) fn`, add milestone)

**Step 1: Add milestone color function and make existing ones accessible**

In `src/tui/ui.rs`, change:
- `fn status_color_problem` → `pub(super) fn status_color_problem`
- `fn status_color_solution` → `pub(super) fn status_color_solution`
- `fn status_color_critique` → `pub(super) fn status_color_critique`
- `fn priority_prefix` → `pub(super) fn priority_prefix`

Add new function after `status_color_critique`:

```rust
pub(super) fn status_color_milestone(status: &crate::models::MilestoneStatus) -> Color {
    use crate::models::MilestoneStatus;
    match status {
        MilestoneStatus::Completed => Color::Green,
        MilestoneStatus::Active => Color::Yellow,
        MilestoneStatus::Cancelled => Color::Red,
        MilestoneStatus::Planning => Color::Cyan,
    }
}
```

Also add a severity color function:

```rust
pub(super) fn severity_color(severity: &crate::models::CritiqueSeverity) -> Color {
    use crate::models::CritiqueSeverity;
    match severity {
        CritiqueSeverity::Critical => Color::Red,
        CritiqueSeverity::High => Color::Yellow,
        CritiqueSeverity::Medium => Color::White,
        CritiqueSeverity::Low => Color::DarkGray,
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Success

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): add milestone/severity color functions, make color helpers pub(super)"
```

---

### Task 3: Replace to_lines() with to_styled_lines() in detail.rs

**Files:**
- Modify: `src/tui/detail.rs` (rewrite entire impl block)

**Step 1: Rewrite DetailContent impl**

Replace the entire `impl DetailContent` block. The new method returns `Vec<Line<'static>>` with styled spans. It also needs a new method `status_color()` that returns the block border color.

```rust
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
            DetailContent::Problem(p) => self.problem_lines(p),
            DetailContent::Solution(s) => self.solution_lines(s),
            DetailContent::Critique(c) => self.critique_lines(c),
            DetailContent::Milestone(m) => self.milestone_lines(m),
        }
    }

    fn problem_lines(&self, p: &Problem) -> Vec<Line<'static>> {
        use super::markdown::markdown_to_line;

        let status_color = super::ui::status_color_problem(&p.status);
        let priority_sym = super::ui::priority_prefix(&p.priority);

        let mut lines = vec![
            Line::from(""),
            // Title
            Line::from(Span::styled(
                format!("  {}{}", priority_sym, p.title),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Metadata
        lines.push(meta_line("  Status", &p.status.to_string(), Some(status_color)));
        lines.push(meta_line(
            "  Priority",
            &p.priority.to_string(),
            Some(super::ui::priority_color(&p.priority)),
        ));
        if let Some(assignee) = &p.assignee {
            let name = assignee.split('<').next().unwrap_or(assignee).trim();
            lines.push(meta_line("  Assignee", name, None));
        }
        if let Some(milestone) = &p.milestone_id {
            lines.push(meta_line("  Milestone", &milestone[..8.min(milestone.len())], None));
        }
        if !p.tags.is_empty() {
            lines.push(tags_line(&p.tags));
        }
        lines.push(Line::from(""));

        // Divider
        lines.push(divider_line());

        // Description
        if !p.description.is_empty() {
            lines.push(Line::from(""));
            lines.push(section_header("Description"));
            for text_line in p.description.lines() {
                lines.push(indent_md(markdown_to_line(text_line)));
            }
        }

        // Context
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

    fn solution_lines(&self, s: &Solution) -> Vec<Line<'static>> {
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

        lines.push(meta_line("  Status", &s.status.to_string(), Some(status_color)));
        lines.push(meta_line("  Problem", &s.problem_id[..8.min(s.problem_id.len())], None));
        if let Some(assignee) = &s.assignee {
            let name = assignee.split('<').next().unwrap_or(assignee).trim();
            lines.push(meta_line("  Assignee", name, None));
        }
        if !s.change_ids.is_empty() {
            lines.push(meta_line("  Changes", &s.change_ids.join(", "), None));
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

    fn critique_lines(&self, c: &Critique) -> Vec<Line<'static>> {
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

        lines.push(meta_line("  Status", &c.status.to_string(), Some(status_color)));
        lines.push(meta_line("  Severity", &c.severity.to_string(), Some(sev_color)));
        lines.push(meta_line("  Solution", &c.solution_id[..8.min(c.solution_id.len())], None));
        if let Some(file) = &c.file_path {
            let loc = format!("{}:{}", file, c.line_start.unwrap_or(0));
            lines.push(meta_line("  Location", &loc, None));
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
                        format!("  {} ", reply.author),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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

    fn milestone_lines(&self, m: &Milestone) -> Vec<Line<'static>> {
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

        lines.push(meta_line("  Status", &m.status.to_string(), Some(status_color)));
        if let Some(date) = &m.target_date {
            lines.push(meta_line("  Target", &date.format("%Y-%m-%d").to_string(), None));
        }
        if let Some(assignee) = &m.assignee {
            let name = assignee.split('<').next().unwrap_or(assignee).trim();
            lines.push(meta_line("  Assignee", name, None));
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
}

/// Render a metadata key-value pair as a styled Line.
fn meta_line(label: &str, value: &str, value_color: Option<Color>) -> Line<'static> {
    let val_style = Style::default().fg(value_color.unwrap_or(Color::White));
    Line::from(vec![
        Span::styled(
            format!("{:<12}", label),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), val_style),
    ])
}

/// Render tags as bracketed cyan chips.
fn tags_line(tags: &[String]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        "  Tags      ".to_string(),
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
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
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

/// Prepend 4 spaces of indentation to a markdown-parsed line.
fn indent_md(line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw("    ".to_string())];
    spans.extend(line.spans);
    Line::from(spans)
}
```

Note: This references `super::ui::priority_color` which doesn't exist yet. Add it in ui.rs alongside the other color helpers:

```rust
pub(super) fn priority_color(priority: &Priority) -> Color {
    match priority {
        Priority::Critical => Color::Red,
        Priority::High => Color::Yellow,
        Priority::Medium => Color::White,
        Priority::Low => Color::DarkGray,
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Success (may need to fix import paths)

**Step 3: Commit**

```bash
git add src/tui/detail.rs src/tui/ui.rs
git commit -m "feat(tui): styled detail lines with metadata grid, dividers, and markdown"
```

---

### Task 4: Update draw_detail() in ui.rs to use styled lines

**Files:**
- Modify: `src/tui/ui.rs:278-318` (draw_detail function)

**Step 1: Update draw_detail to consume styled lines**

Replace the `draw_detail` function body. Key changes:
1. Call `to_styled_lines()` instead of `to_lines()`
2. Use `border_color()` and `block_title()` for the detail block
3. Remove the string-to-Line conversion (lines are already styled)

```rust
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
```

**Step 2: Remove the old to_lines() method from detail.rs**

Delete the `to_lines()` method entirely (it's now replaced by `to_styled_lines()`).

**Step 3: Check for any remaining callers of to_lines()**

Run: `cargo build`
If there are compilation errors from other files calling `to_lines()`, update them to use `to_styled_lines()`.

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 5: Run clippy and fmt**

Run: `cargo fmt && cargo clippy -- -D warnings`
Expected: Clean

**Step 6: Commit**

```bash
git add src/tui/ui.rs src/tui/detail.rs
git commit -m "feat(tui): wire styled detail lines into draw_detail with colored borders"
```

---

### Task 5: Final cleanup and verification

**Files:**
- Possibly modify: any files with compilation warnings

**Step 1: Full build**

Run: `cargo build`
Expected: Clean

**Step 2: Full test suite**

Run: `cargo test`
Expected: All pass

**Step 3: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: Clean

**Step 4: Manual test**

Run: `cargo run -- tui`
Verify:
- Select a problem → see colored title, metadata grid, divider, markdown-styled description
- Select a solution → see approach section with markdown rendering
- Select a critique → see severity in color, argument/evidence sections
- Select a milestone → see goals and success criteria sections
- Block border color matches entity status (green for solved, yellow for active, etc.)
- Tags show as `[tag1] [tag2]` in cyan
- Scrolling with j/k still works correctly

**Step 5: Commit if any cleanup was needed**

```bash
git add -A
git commit -m "chore: cleanup detail pane richness implementation"
```
