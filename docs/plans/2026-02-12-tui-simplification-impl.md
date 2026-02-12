# TUI Simplification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Simplify TUI from 3 panes to 2 by merging Next Actions into Tree with visual symbols and Tab navigation.

**Architecture:** Remove FocusedPane enum and next_actions_index. Add action_symbol field to FlatTreeItem populated from next_actions logic. Tab cycles through action items (items with symbols) instead of switching panes. Tree rendering adds symbols and priority/status colors. Add filter mode to toggle between full tree and actions-only view.

**Tech Stack:** Rust, ratatui

---

## Task 1: Add action_symbol to FlatTreeItem

**Files:**
- Modify: `src/tui/tree.rs:73-78`
- Modify: `src/tui/tree.rs:97-106` (and similar FlatTreeItem constructions)

**Step 1: Add action_symbol field to FlatTreeItem struct**

In `src/tui/tree.rs`, update the struct:

```rust
#[derive(Debug, Clone)]
pub struct FlatTreeItem {
    pub node: TreeNode,
    pub depth: usize,
    pub has_children: bool,
    pub action_symbol: Option<String>,  // NEW: e.g., "⚡", "🚫", "⏳", "📋", "👀"
}
```

**Step 2: Update all FlatTreeItem constructions to include action_symbol: None**

Update the 4 places where FlatTreeItem is constructed:
- Line ~97 (Milestone)
- Line ~127 (Backlog)
- Line ~164 (Problem)
- Line ~183 (Solution)
- Line ~196 (Critique)

Each should add `action_symbol: None`:

```rust
items.push(FlatTreeItem {
    node: TreeNode::Milestone { ... },
    depth: 0,
    has_children: !milestone_problems.is_empty(),
    action_symbol: None,  // ADD THIS
});
```

**Step 3: Run tests to verify compilation**

Run: `cargo test --lib`
Expected: PASS (no behavior change yet)

**Step 4: Commit**

```bash
git add src/tui/tree.rs
git commit -m "refactor(tui): add action_symbol field to FlatTreeItem"
```

---

## Task 2: Add priority field to TreeNode::Problem

**Files:**
- Modify: `src/tui/tree.rs:17-21`
- Modify: `src/tui/tree.rs:163-173`

**Step 1: Add priority field to TreeNode::Problem variant**

```rust
TreeNode::Problem {
    id: String,
    title: String,
    status: ProblemStatus,
    priority: crate::models::Priority,  // ADD THIS
    expanded: bool,
}
```

**Step 2: Update TreeNode::Problem construction in add_problems**

```rust
items.push(FlatTreeItem {
    node: TreeNode::Problem {
        id: problem.id.clone(),
        title: problem.title.clone(),
        status: problem.status.clone(),
        priority: problem.priority.clone(),  // ADD THIS
        expanded,
    },
    depth,
    has_children: !problem_solutions.is_empty(),
    action_symbol: None,
});
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/tree.rs
git commit -m "refactor(tui): add priority field to TreeNode::Problem"
```

---

## Task 3: Create annotate_tree_with_actions function

**Files:**
- Modify: `src/tui/tree.rs` (add new function at end)

**Step 1: Add import for Category at top of tree.rs**

```rust
use super::next_actions::{Category, NextAction};
```

**Step 2: Add annotate_tree_with_actions function**

```rust
/// Annotates tree items with action symbols based on next_actions list
pub fn annotate_tree_with_actions(items: &mut [FlatTreeItem], next_actions: &[NextAction]) {
    use std::collections::HashMap;

    // Build lookup from entity_id -> category
    let action_map: HashMap<&str, Category> = next_actions
        .iter()
        .map(|a| (a.entity_id.as_str(), a.category))
        .collect();

    for item in items.iter_mut() {
        let id = item.node.id();
        if let Some(&category) = action_map.get(id) {
            item.action_symbol = Some(category_to_symbol(category).to_string());
        }
    }
}

fn category_to_symbol(category: Category) -> &'static str {
    match category {
        Category::Ready => "⚡",
        Category::Blocked => "🚫",
        Category::Waiting => "⏳",
        Category::Todo => "📋",
        Category::Review => "👀",
    }
}
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/tree.rs
git commit -m "feat(tui): add annotate_tree_with_actions function"
```

---

## Task 4: Export and call annotate_tree_with_actions from mod.rs

**Files:**
- Modify: `src/tui/mod.rs`

**Step 1: Find current exports in mod.rs**

Read `src/tui/mod.rs` to understand current structure.

**Step 2: Add re-export**

Add to the pub use section:
```rust
pub use tree::annotate_tree_with_actions;
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/mod.rs
git commit -m "feat(tui): export annotate_tree_with_actions"
```

---

## Task 5: Call annotate_tree_with_actions in App::rebuild_cache

**Files:**
- Modify: `src/tui/app.rs:1130-1140`

**Step 1: Update rebuild_cache to annotate tree items**

```rust
fn rebuild_cache(&mut self) {
    let user = self.store.jj_client.user_identity().unwrap_or_default();
    self.cache.next_actions = super::build_next_actions(
        &self.data.problems,
        &self.data.solutions,
        &self.data.critiques,
        &user,
    );
    self.rebuild_tree();
    // NEW: Annotate tree with action symbols
    super::annotate_tree_with_actions(&mut self.cache.tree_items, &self.cache.next_actions);
    self.update_selected_detail();
}
```

**Step 2: Also update App::new where cache is first built (~line 154)**

After `let cache = RenderCache { ... }` and before `let mut app = Self { ... }`:

```rust
let mut cache = RenderCache {
    next_actions,
    tree_items,
    selected_detail: super::DetailContent::None,
};
super::annotate_tree_with_actions(&mut cache.tree_items, &cache.next_actions);
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): annotate tree items with action symbols"
```

---

## Task 6: Update draw_project_tree to show symbols and colors

**Files:**
- Modify: `src/tui/ui.rs:137-214`

**Step 1: Import Priority**

Add at top of file:
```rust
use crate::models::Priority;
```

**Step 2: Add priority_prefix helper function**

```rust
fn priority_prefix(priority: &Priority) -> &'static str {
    match priority {
        Priority::Critical => "🔴 ",
        Priority::High => "🟡 ",
        Priority::Medium | Priority::Low => "",
    }
}
```

**Step 3: Update draw_project_tree to include action symbols and priority**

Replace the item mapping in draw_project_tree:

```rust
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
            "  "  // Changed from "○ " to align better
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
                id, title, status, priority, ..
            } => {
                let priority_sym = priority_prefix(priority);
                let dim = matches!(priority, Priority::Low);
                (
                    format!("{}{}{}{}{}: {}", indent, expand_char, priority_sym, action_sym, id, title),
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
                format!("{}{}{}{}: {} [{}]", indent, expand_char, action_sym, id, title, severity),
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
```

**Step 4: Run TUI manually to verify**

Run: `cargo run -- tui`
Expected: Tree items should show action symbols (⚡🚫⏳📋👀) and priority symbols (🔴🟡)

**Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): show action symbols and priority in tree"
```

---

## Task 7: Remove FocusedPane and next_actions_index from UiState

**Files:**
- Modify: `src/tui/app.rs:48-52` (remove FocusedPane enum)
- Modify: `src/tui/app.rs:74-85` (remove from UiState)
- Modify: `src/tui/app.rs:94-109` (remove from UiState::new)

**Step 1: Remove FocusedPane enum**

Delete lines 48-52:
```rust
// DELETE THIS:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    NextActions,
    ProjectTree,
}
```

**Step 2: Remove focused_pane and next_actions_index from UiState**

Update UiState struct:
```rust
pub struct UiState {
    // REMOVE: pub focused_pane: FocusedPane,
    // REMOVE: pub next_actions_index: usize,
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
    pub show_related: bool,
    pub related_items: Vec<SimilarityResult>,
    pub related_selected: usize,
    pub input_mode: InputMode,
}
```

**Step 3: Update UiState::new**

```rust
impl UiState {
    pub fn new() -> Self {
        let mut expanded_nodes = HashSet::new();
        expanded_nodes.insert("backlog".to_string());
        Self {
            // REMOVE: focused_pane: FocusedPane::NextActions,
            // REMOVE: next_actions_index: 0,
            tree_index: 0,
            expanded_nodes,
            detail_scroll: 0,
            flash_message: None,
            show_related: true,
            related_items: Vec::new(),
            related_selected: 0,
            input_mode: InputMode::Normal,
        }
    }
}
```

**Step 4: Fix all compilation errors (many places reference focused_pane)**

This will cause many errors. Fix each by:
- Remove all `match self.ui.focused_pane { FocusedPane::NextActions => ..., FocusedPane::ProjectTree => ... }`
- Keep only the ProjectTree branch logic (tree_index based)

Key methods to update:
- `toggle_focus` - will become `jump_to_next_action` (Task 8)
- `navigate_up` - remove NextActions branch
- `navigate_down` - remove NextActions branch
- `sync_tree_to_selection` - delete entirely (no longer needed)
- `get_selected_entity_info` - remove NextActions branch
- `get_selected_entity` - remove NextActions branch
- `update_selected_detail` - remove NextActions branch
- `context_hints` - remove NextActions branch
- `start_new_item` - remove NextActions branch
- `start_edit_title` - remove NextActions branch
- `open_in_editor` - remove NextActions branch

**Step 5: Run to find remaining errors**

Run: `cargo build`
Expected: Fix all errors until it compiles

**Step 6: Commit**

```bash
git add src/tui/app.rs
git commit -m "refactor(tui): remove FocusedPane and next_actions_index"
```

---

## Task 8: Implement Tab jumping to action items

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Replace toggle_focus with jump_to_next_action**

```rust
fn jump_to_next_action(&mut self, reverse: bool) {
    if self.cache.tree_items.is_empty() {
        return;
    }

    // Find indices of items with action symbols
    let action_indices: Vec<usize> = self.cache.tree_items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.action_symbol.is_some())
        .map(|(i, _)| i)
        .collect();

    if action_indices.is_empty() {
        return;
    }

    // Find next action item
    let current = self.ui.tree_index;
    let next_index = if reverse {
        // Find previous action item (or wrap to last)
        action_indices
            .iter()
            .rev()
            .find(|&&i| i < current)
            .or_else(|| action_indices.last())
            .copied()
    } else {
        // Find next action item (or wrap to first)
        action_indices
            .iter()
            .find(|&&i| i > current)
            .or_else(|| action_indices.first())
            .copied()
    };

    if let Some(idx) = next_index {
        self.ui.tree_index = idx;
        self.update_selected_detail();
    }
}
```

**Step 2: Update handle_normal_key for Tab and BackTab**

```rust
KeyCode::Tab => self.jump_to_next_action(false),
KeyCode::BackTab => self.jump_to_next_action(true),
```

Note: `KeyCode::BackTab` handles Shift+Tab.

**Step 3: Run TUI to test Tab navigation**

Run: `cargo run -- tui`
Expected: Tab jumps to next item with action symbol, Shift+Tab jumps backward

**Step 4: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): Tab jumps to action items"
```

---

## Task 9: Update layout from 3 panes to 2

**Files:**
- Modify: `src/tui/ui.rs:11-36`

**Step 1: Update draw function to use 2-pane layout**

```rust
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

    // Main layout: two columns (was three)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Project Tree (was 35%)
            Constraint::Percentage(60), // Detail Pane (was 45%)
        ])
        .split(vertical_chunks[0]);

    // REMOVED: draw_next_actions(f, app, main_chunks[0]);
    draw_project_tree(f, app, main_chunks[0]);  // Now index 0
    draw_detail(f, app, main_chunks[1]);        // Now index 1

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
```

**Step 2: Run TUI to verify layout**

Run: `cargo run -- tui`
Expected: Two-pane layout (40% tree, 60% detail)

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): change to 2-pane layout (40/60)"
```

---

## Task 10: Remove draw_next_actions function

**Files:**
- Modify: `src/tui/ui.rs:63-106`

**Step 1: Delete draw_next_actions function entirely**

Remove the entire function (lines ~63-106).

**Step 2: Remove unused import FocusedPane**

At the top of ui.rs, change:
```rust
use super::app::{App, FocusedPane, InputMode};
```
to:
```rust
use super::app::{App, InputMode};
```

**Step 3: Run to verify**

Run: `cargo build`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "refactor(tui): remove draw_next_actions function"
```

---

## Task 11: Update draw_project_tree to always show focused border

**Files:**
- Modify: `src/tui/ui.rs` (draw_project_tree function)

**Step 1: Remove focus-dependent border styling**

Since there's only one pane now, it's always "focused". Update draw_project_tree:

```rust
fn draw_project_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use super::tree::TreeNode;

    // Always show as focused (single pane)
    let border_style = Style::default().fg(Color::Cyan);

    // ... rest unchanged
```

**Step 2: Run TUI**

Run: `cargo run -- tui`
Expected: Tree border always cyan

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "style(tui): tree always shows focused border"
```

---

## Task 12: Update footer hint text

**Files:**
- Modify: `src/tui/ui.rs` (draw_footer function, ~line 332)

**Step 1: Update footer text**

Change:
```rust
let global = Paragraph::new("[Tab] pane | [R] related | [j/k] scroll | [?] help | [q] quit")
```
to:
```rust
let global = Paragraph::new("[Tab] next action | [R] related | [j/k] scroll | [?] help | [q] quit")
```

**Step 2: Run TUI**

Run: `cargo run -- tui`
Expected: Footer shows updated hint

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "docs(tui): update footer hint for Tab"
```

---

## Task 13: Update help overlay

**Files:**
- Modify: `src/tui/ui.rs` (draw_help_overlay and get_context_actions)

**Step 1: Update navigation section in draw_help_overlay**

Change:
```rust
Line::from("    Tab     Switch pane"),
```
to:
```rust
Line::from("    Tab     Jump to next action"),
Line::from("    S-Tab   Jump to prev action"),
```

**Step 2: Update get_context_actions to remove FocusedPane references**

Since FocusedPane is gone, simplify get_context_actions to only use tree selection:

```rust
fn get_context_actions(app: &App) -> Vec<Line<'static>> {
    use super::tree::TreeNode;

    let mut lines = vec![
        Line::from(Span::styled("  Actions", Style::default().add_modifier(Modifier::BOLD))),
    ];

    // Determine what's selected from tree
    let entity_type = app.cache.tree_items.get(app.ui.tree_index)
        .and_then(|item| match &item.node {
            TreeNode::Problem { .. } => Some(super::next_actions::EntityType::Problem),
            TreeNode::Solution { .. } => Some(super::next_actions::EntityType::Solution),
            TreeNode::Critique { .. } => Some(super::next_actions::EntityType::Critique),
            TreeNode::Milestone { .. } | TreeNode::Backlog { .. } => None,
        });

    match entity_type {
        Some(super::next_actions::EntityType::Problem) => {
            lines.push(Line::from("    n       New solution"));
            lines.push(Line::from("    s       Mark solved"));
            lines.push(Line::from("    o       Reopen"));
            lines.push(Line::from("    d       Dissolve"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
        Some(super::next_actions::EntityType::Solution) => {
            lines.push(Line::from("    n       New critique"));
            lines.push(Line::from("    a       Accept"));
            lines.push(Line::from("    r       Refute"));
            lines.push(Line::from("    e       Edit title"));
            lines.push(Line::from("    E       Edit in $EDITOR"));
        }
        Some(super::next_actions::EntityType::Critique) => {
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
```

**Step 3: Run TUI and test help**

Run: `cargo run -- tui`, then press `?`
Expected: Help shows updated navigation and context-sensitive actions

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "docs(tui): update help overlay for new layout"
```

---

## Task 14: Add filter_actions_only to UiState

**Files:**
- Modify: `src/tui/app.rs:74-85` (UiState struct)
- Modify: `src/tui/app.rs:94-109` (UiState::new)

**Step 1: Add filter_actions_only field to UiState**

```rust
pub struct UiState {
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
    pub show_related: bool,
    pub related_items: Vec<SimilarityResult>,
    pub related_selected: usize,
    pub input_mode: InputMode,
    pub filter_actions_only: bool,  // NEW
}
```

**Step 2: Initialize to false in UiState::new**

```rust
Self {
    tree_index: 0,
    expanded_nodes,
    detail_scroll: 0,
    flash_message: None,
    show_related: true,
    related_items: Vec::new(),
    related_selected: 0,
    input_mode: InputMode::Normal,
    filter_actions_only: false,  // NEW
}
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/app.rs
git commit -m "refactor(tui): add filter_actions_only to UiState"
```

---

## Task 15: Create filter_tree_to_actions function

**Files:**
- Modify: `src/tui/tree.rs` (add new function)

**Step 1: Add filter_tree_to_actions function**

```rust
/// Filters tree to only show action items and their ancestors
pub fn filter_tree_to_actions(items: &[FlatTreeItem]) -> Vec<FlatTreeItem> {
    use std::collections::HashSet;

    // First pass: collect IDs of items with action symbols
    let action_ids: HashSet<&str> = items
        .iter()
        .filter(|item| item.action_symbol.is_some())
        .map(|item| item.node.id())
        .collect();

    if action_ids.is_empty() {
        return Vec::new();
    }

    // Second pass: for each action item, mark all ancestors as needed
    let mut needed_ids: HashSet<String> = HashSet::new();
    for item in items.iter().filter(|i| i.action_symbol.is_some()) {
        needed_ids.insert(item.node.id().to_string());

        // Walk backwards to find ancestors
        let item_depth = item.depth;
        let item_idx = items.iter().position(|i| i.node.id() == item.node.id()).unwrap();

        let mut current_depth = item_depth;
        for ancestor in items[..item_idx].iter().rev() {
            if ancestor.depth < current_depth {
                needed_ids.insert(ancestor.node.id().to_string());
                current_depth = ancestor.depth;
                if current_depth == 0 {
                    break;
                }
            }
        }
    }

    // Third pass: keep only needed items
    items
        .iter()
        .filter(|item| needed_ids.contains(item.node.id()))
        .cloned()
        .collect()
}
```

**Step 2: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 3: Commit**

```bash
git add src/tui/tree.rs
git commit -m "feat(tui): add filter_tree_to_actions function"
```

---

## Task 16: Export filter_tree_to_actions

**Files:**
- Modify: `src/tui/mod.rs`

**Step 1: Add to exports**

```rust
pub use tree::{annotate_tree_with_actions, build_flat_tree, filter_tree_to_actions, FlatTreeItem, TreeNode};
```

**Step 2: Commit**

```bash
git add src/tui/mod.rs
git commit -m "feat(tui): export filter_tree_to_actions"
```

---

## Task 17: Add toggle_filter and filtered tree rendering

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add toggle_filter method**

```rust
fn toggle_filter(&mut self) {
    self.ui.filter_actions_only = !self.ui.filter_actions_only;
    let mode = if self.ui.filter_actions_only { "Actions only" } else { "Full tree" };
    self.show_flash(mode);
}
```

**Step 2: Add 'f' key binding in handle_normal_key**

```rust
KeyCode::Char('f') => self.toggle_filter(),
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): add f key to toggle filter mode"
```

---

## Task 18: Update draw_project_tree to use filter

**Files:**
- Modify: `src/tui/ui.rs` (draw_project_tree function)

**Step 1: Apply filter when rendering**

At the start of draw_project_tree, get the items to display:

```rust
fn draw_project_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use super::tree::TreeNode;

    // Apply filter if enabled
    let display_items: Vec<_> = if app.ui.filter_actions_only {
        super::filter_tree_to_actions(&app.cache.tree_items)
    } else {
        app.cache.tree_items.clone()
    };

    // Always show as focused (single pane)
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
            // ... rest of mapping logic unchanged
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)  // Use dynamic title
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        // ... rest unchanged
```

**Step 2: Fix selection index when filtered**

The tree_index refers to the full tree. When filtered, we need to map it:

```rust
    // Find selection in display items
    let selected_id = app.cache.tree_items
        .get(app.ui.tree_index)
        .map(|i| i.node.id());

    let display_index = selected_id
        .and_then(|id| display_items.iter().position(|i| i.node.id() == id));

    let mut state = ListState::default();
    if let Some(idx) = display_index {
        state.select(Some(idx));
    } else if !display_items.is_empty() {
        state.select(Some(0));
    }
```

**Step 3: Run TUI to test**

Run: `cargo run -- tui`, press `f` to toggle
Expected: Tree filters to show only action items and ancestors

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): filter tree display when filter enabled"
```

---

## Task 19: Update help overlay with filter key

**Files:**
- Modify: `src/tui/ui.rs` (draw_help_overlay)

**Step 1: Add filter key to navigation section**

```rust
Line::from("    f       Toggle filter (full/actions)"),
```

**Step 2: Commit**

```bash
git add src/tui/ui.rs
git commit -m "docs(tui): add filter key to help"
```

---

## Task 20: Run full test suite

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings (or only minor ones)

**Step 3: Run fmt**

Run: `cargo fmt`

**Step 4: Final commit if any formatting changes**

```bash
git add -A
git commit -m "style: apply rustfmt"
```

---

## Task 21: Manual integration testing

**Step 1: Start TUI**

Run: `cargo run -- tui`

**Step 2: Verify features**

Test checklist:
- [ ] Tree shows action symbols (⚡🚫⏳📋👀)
- [ ] Tree shows priority symbols for P0/P1 problems (🔴🟡)
- [ ] P3 (Low) problems are dimmed
- [ ] Tab jumps to next action item
- [ ] Shift+Tab jumps to previous action item
- [ ] Tab wraps around at end
- [ ] Arrow keys navigate normally
- [ ] Left/Right collapse/expand work
- [ ] n/e/E/s/o/a/r/d/v work on appropriate items
- [ ] Help (?) shows correct info
- [ ] Layout is 40/60 split
- [ ] Detail pane shows correct content
- [ ] `f` toggles filter mode
- [ ] Filter mode shows only action items + ancestors
- [ ] Title shows "[Actions]" when filtered
- [ ] Selection preserved when toggling filter

**Step 3: If issues found, fix and commit**

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add action_symbol to FlatTreeItem | tree.rs |
| 2 | Add priority to TreeNode::Problem | tree.rs |
| 3 | Create annotate_tree_with_actions | tree.rs |
| 4 | Export annotate_tree_with_actions | mod.rs |
| 5 | Call annotate in rebuild_cache | app.rs |
| 6 | Show symbols/colors in tree | ui.rs |
| 7 | Remove FocusedPane | app.rs |
| 8 | Implement Tab jumping | app.rs |
| 9 | Update to 2-pane layout | ui.rs |
| 10 | Remove draw_next_actions | ui.rs |
| 11 | Always-focused border | ui.rs |
| 12 | Update footer hint | ui.rs |
| 13 | Update help overlay | ui.rs |
| 14 | Add filter_actions_only to UiState | app.rs |
| 15 | Create filter_tree_to_actions | tree.rs |
| 16 | Export filter_tree_to_actions | mod.rs |
| 17 | Add toggle_filter and f key | app.rs |
| 18 | Apply filter in draw_project_tree | ui.rs |
| 19 | Add filter key to help | ui.rs |
| 20 | Run test suite | - |
| 21 | Manual testing | - |
