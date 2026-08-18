# Detail Pane Richness Design

## Problem

The TUI detail pane renders entity data as plain `format!()` strings with no color, no visual hierarchy, and no markdown support. The pane is the primary reading surface but looks like raw key-value dumps.

## Design

### Layout Structure

Every entity type follows the same visual template:

1. **Title line** - Bold, status-colored, with priority emoji (problems)
2. **Metadata grid** - Key-value pairs with dimmed labels and colored values, 2-column where space allows
3. **Tags** - Rendered as bracketed chips `[tag1] [tag2]` in cyan
4. **Divider** - Thin `─────` line in dark gray
5. **Body sections** - Section headers in bold white, body text with basic markdown rendering

Per entity type:
- **Problem**: status/priority/assignee/milestone/tags, then description + context
- **Solution**: status/problem/assignee/changes/tags, then approach + tradeoffs
- **Critique**: status/severity/solution, then argument + evidence + location + replies
- **Milestone**: status/target date/assignee, then goals + success criteria

### Markdown Rendering

Line-by-line parser in `src/tui/markdown.rs` with a single public function:

```rust
pub fn markdown_to_line(text: &str) -> Line<'static>
```

Supported syntax:
- `**bold**` / `__bold__` -> BOLD modifier
- `` `inline code` `` -> Cyan on DarkGray background
- `# Header` / `## Subheader` -> BOLD + White
- `- item` / `* item` -> bullet with indented content
- `> blockquote` -> DarkGray + italic
- Blank lines preserved

No nested formatting, no fenced code blocks, no multi-line state.

### Color Theming

- **Block border**: status-colored (reuses existing `status_color_*` functions)
- **Block title**: entity type name
- **Metadata labels**: DarkGray (dimmed)
- **Metadata values**: White, except status/priority which use semantic colors
- **Section headers**: White + BOLD
- **Tags**: Cyan with brackets
- **Divider**: DarkGray `─` characters

### Architecture

- `detail.rs`: Replace `to_lines() -> Vec<String>` with `to_styled_lines() -> Vec<Line<'static>>`
- New `src/tui/markdown.rs`: line-by-line markdown-to-spans parser
- `ui.rs`: `draw_detail()` consumes styled lines directly, block border colored by entity status

## Scope

All four entity types (Problem, Solution, Critique, Milestone) in one pass.
