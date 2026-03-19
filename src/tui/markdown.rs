use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Convert a single line of markdown text to a styled Ratatui `Line`.
///
/// Supports headers (`#`, `##`, `###`), list items (`-`, `*`),
/// blockquotes (`>`), and inline formatting (`**bold**`, `__bold__`, `` `code` ``).
pub fn markdown_to_line(text: &str) -> Line<'static> {
    // Empty lines
    if text.is_empty() {
        return Line::from(Span::raw(String::new()));
    }

    // Headers: # Header, ## Sub, ### Subsub
    if let Some(rest) = text.strip_prefix("### ") {
        let style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        return Line::from(Span::styled(rest.to_owned(), style));
    }
    if let Some(rest) = text.strip_prefix("## ") {
        let style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        return Line::from(Span::styled(rest.to_owned(), style));
    }
    if let Some(rest) = text.strip_prefix("# ") {
        let style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        return Line::from(Span::styled(rest.to_owned(), style));
    }

    // List items: - item or * item
    if let Some(rest) = text.strip_prefix("- ") {
        let mut spans = vec![Span::styled(
            "  \u{2022} ".to_owned(),
            Style::default().fg(Color::DarkGray),
        )];
        spans.extend(parse_inline_spans(rest));
        return Line::from(spans);
    }
    if let Some(rest) = text.strip_prefix("* ") {
        let mut spans = vec![Span::styled(
            "  \u{2022} ".to_owned(),
            Style::default().fg(Color::DarkGray),
        )];
        spans.extend(parse_inline_spans(rest));
        return Line::from(spans);
    }

    // Blockquote: > text
    if let Some(rest) = text.strip_prefix("> ") {
        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        return Line::from(Span::styled(format!("  {rest}"), style));
    }

    // Default: parse inline formatting
    Line::from(parse_inline_spans(text))
}

/// Marker types for inline formatting.
#[derive(Debug, Clone, Copy)]
enum InlineMarker {
    DoubleStar,
    DoubleUnderscore,
    Backtick,
}

impl InlineMarker {
    fn pattern(&self) -> &'static str {
        match self {
            InlineMarker::DoubleStar => "**",
            InlineMarker::DoubleUnderscore => "__",
            InlineMarker::Backtick => "`",
        }
    }

    fn len(&self) -> usize {
        self.pattern().len()
    }
}

/// Parse inline markdown formatting (`**bold**`, `__bold__`, `` `code` ``) into
/// a vector of styled Ratatui `Span`s.
fn parse_inline_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest marker in the remaining text
        let candidates = [
            InlineMarker::DoubleStar,
            InlineMarker::DoubleUnderscore,
            InlineMarker::Backtick,
        ];

        let earliest = candidates
            .iter()
            .filter_map(|marker| remaining.find(marker.pattern()).map(|pos| (pos, *marker)))
            .min_by_key(|(pos, _)| *pos);

        match earliest {
            Some((pos, marker)) => {
                // Emit any text before the marker as plain
                if pos > 0 {
                    spans.push(Span::raw(remaining[..pos].to_owned()));
                }

                let after_open = &remaining[pos + marker.len()..];

                // Look for the closing marker
                if let Some(close_pos) = after_open.find(marker.pattern()) {
                    let content = &after_open[..close_pos];
                    let styled_span = match marker {
                        InlineMarker::DoubleStar | InlineMarker::DoubleUnderscore => Span::styled(
                            content.to_owned(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        InlineMarker::Backtick => Span::styled(
                            content.to_owned(),
                            Style::default().fg(Color::Cyan).bg(Color::DarkGray),
                        ),
                    };
                    spans.push(styled_span);
                    remaining = &after_open[close_pos + marker.len()..];
                } else {
                    // No closing marker found — emit the marker as literal text and continue
                    spans.push(Span::raw(remaining[pos..pos + marker.len()].to_owned()));
                    remaining = after_open;
                }
            }
            None => {
                // No markers found — emit rest as plain text
                spans.push(Span::raw(remaining.to_owned()));
                remaining = "";
            }
        }
    }

    // Guarantee at least one span
    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let line = markdown_to_line("Hello world");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "Hello world");
    }

    #[test]
    fn test_bold_double_star() {
        let line = markdown_to_line("**bold**");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "bold");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_bold_double_underscore() {
        let line = markdown_to_line("__bold__");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "bold");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_inline_code() {
        let line = markdown_to_line("`code`");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "code");
        assert_eq!(line.spans[0].style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_header() {
        let line = markdown_to_line("# Header");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "Header");
        let mods = line.spans[0].style.add_modifier;
        assert!(mods.contains(Modifier::BOLD));
        assert!(mods.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_subheader() {
        let line = markdown_to_line("## Sub");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "Sub");
        let mods = line.spans[0].style.add_modifier;
        assert!(mods.contains(Modifier::BOLD));
        assert!(mods.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_list_item_dash() {
        let line = markdown_to_line("- item");
        // First span: bullet, second+: content
        assert!(line.spans.len() >= 2);
        assert!(line.spans[0].content.contains('\u{2022}'));
        assert_eq!(line.spans[1].content.as_ref(), "item");
    }

    #[test]
    fn test_list_item_star() {
        let line = markdown_to_line("* item");
        assert!(line.spans.len() >= 2);
        assert!(line.spans[0].content.contains('\u{2022}'));
        assert_eq!(line.spans[1].content.as_ref(), "item");
    }

    #[test]
    fn test_blockquote() {
        let line = markdown_to_line("> text");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "  text");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_empty_string() {
        let line = markdown_to_line("");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "");
    }

    #[test]
    fn test_multiple_bold() {
        let line = markdown_to_line("**a** and **b**");
        // Should produce: bold("a"), plain(" and "), bold("b")
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[0].content.as_ref(), "a");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(line.spans[1].content.as_ref(), " and ");
        assert_eq!(line.spans[2].content.as_ref(), "b");
        assert!(line.spans[2].style.add_modifier.contains(Modifier::BOLD));
    }
}
