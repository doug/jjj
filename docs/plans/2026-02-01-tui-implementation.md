# TUI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build `jjj ui` - a three-column interactive TUI with Next Actions, Project Tree, and Detail Pane.

**Architecture:** Replace existing `tui.rs` with a modular TUI built on ratatui. Uses the existing `status.rs` logic for Next Actions data. Project Tree renders the milestone/problem/solution/critique hierarchy. All actions route through existing CLI command handlers.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28 (already in Cargo.toml)

---

## Task 1: Add `Ui` Command to CLI

**Files:**
- Modify: `src/cli.rs:11-113` (add Ui variant to Commands enum)
- Modify: `src/commands/mod.rs:1-42` (add ui module and execute routing)
- Create: `src/commands/ui.rs`

**Step 1: Add Ui command variant to cli.rs**

In `src/cli.rs`, add after line 22 (after Board command):

```rust
    /// Launch interactive TUI
    Ui,
```

**Step 2: Run `cargo check` to verify syntax**

Run: `cargo check`
Expected: Compilation error about non-exhaustive match in commands/mod.rs

**Step 3: Add ui module to commands/mod.rs**

At top of `src/commands/mod.rs`, add after line 1:

```rust
pub mod ui;
```

**Step 4: Add match arm for Ui command**

In `src/commands/mod.rs` function `execute`, add after line 20 (after Board match):

```rust
        Commands::Ui => ui::execute(),
```

**Step 5: Create minimal ui.rs placeholder**

Create `src/commands/ui.rs`:

```rust
use crate::error::Result;

pub fn execute() -> Result<()> {
    println!("TUI coming soon!");
    Ok(())
}
```

**Step 6: Run `cargo check` to verify**

Run: `cargo check`
Expected: Compiles successfully

**Step 7: Test the command exists**

Run: `cargo run -- ui`
Expected: Output "TUI coming soon!"

**Step 8: Commit**

```bash
git add src/cli.rs src/commands/mod.rs src/commands/ui.rs
git commit -m "feat: add jjj ui command placeholder"
```

---

## Task 2: Remove Board Command

**Files:**
- Modify: `src/cli.rs:17-22` (remove Board variant)
- Modify: `src/commands/mod.rs` (remove board module and routing)
- Delete: `src/commands/board.rs`

**Step 1: Remove Board from Commands enum in cli.rs**

Delete lines 17-22 in `src/cli.rs`:

```rust
    /// Display the board (solutions by status)
    Board {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
```

**Step 2: Remove board module declaration**

In `src/commands/mod.rs`, delete line 1:

```rust
pub mod board;
```

**Step 3: Remove board match arm**

In `src/commands/mod.rs`, delete the Board match arm:

```rust
        Commands::Board { json } => board::execute(json),
```

**Step 4: Delete board.rs file**

Run: `rm src/commands/board.rs`

**Step 5: Run `cargo check` to verify**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: remove jjj board command (replaced by jjj ui)"
```

---

## Task 3: Create TUI Module Structure

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Create: `src/tui/ui.rs`
- Modify: `src/lib.rs` (update module reference)
- Delete: `src/tui.rs` (old file)

**Step 1: Create tui directory**

Run: `mkdir -p src/tui`

**Step 2: Create mod.rs**

Create `src/tui/mod.rs`:

```rust
mod app;
mod ui;

pub use app::App;

use crate::error::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Launch the interactive TUI
pub fn launch() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new()?;
    let res = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}
```

**Step 3: Create app.rs with basic structure**

Create `src/tui/app.rs`:

```rust
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, Milestone, Problem, Solution};
use crate::storage::MetadataStore;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::time::Duration;

/// Which pane has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    NextActions,
    ProjectTree,
}

/// Application state
pub struct App {
    pub should_quit: bool,
    pub focused_pane: FocusedPane,

    // Data
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,

    // Selection state
    pub next_actions_index: usize,
    pub tree_index: usize,
    pub detail_scroll: u16,

    // Store for actions
    store: MetadataStore,
}

impl App {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;

        let milestones = store.list_milestones()?;
        let problems = store.list_problems()?;
        let solutions = store.list_solutions()?;
        let critiques = store.list_critiques()?;

        Ok(Self {
            should_quit: false,
            focused_pane: FocusedPane::NextActions,
            milestones,
            problems,
            solutions,
            critiques,
            next_actions_index: 0,
            tree_index: 0,
            detail_scroll: 0,
            store,
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| super::ui::draw(f, self))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.toggle_focus(),
            _ => {}
        }
        Ok(())
    }

    fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::NextActions => FocusedPane::ProjectTree,
            FocusedPane::ProjectTree => FocusedPane::NextActions,
        };
    }
}
```

**Step 4: Create ui.rs with basic layout**

Create `src/tui/ui.rs`:

```rust
use super::app::{App, FocusedPane};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main layout: three columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Next Actions
            Constraint::Percentage(35), // Project Tree
            Constraint::Percentage(45), // Detail Pane
        ])
        .split(size);

    // Next Actions pane
    let next_actions_style = if app.focused_pane == FocusedPane::NextActions {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let next_actions = Paragraph::new("Next Actions placeholder")
        .block(Block::default()
            .title("Next Actions")
            .borders(Borders::ALL)
            .border_style(next_actions_style));
    f.render_widget(next_actions, main_chunks[0]);

    // Project Tree pane
    let tree_style = if app.focused_pane == FocusedPane::ProjectTree {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let tree = Paragraph::new("Project Tree placeholder")
        .block(Block::default()
            .title("Project Tree")
            .borders(Borders::ALL)
            .border_style(tree_style));
    f.render_widget(tree, main_chunks[1]);

    // Detail pane
    let detail = Paragraph::new("Detail placeholder")
        .block(Block::default()
            .title("Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(detail, main_chunks[2]);
}
```

**Step 5: Update lib.rs to use new tui module**

In `src/lib.rs`, change:

```rust
pub mod tui;
```

(Keep as-is, but it will now point to the directory)

**Step 6: Delete old tui.rs**

Run: `rm src/tui.rs`

**Step 7: Update commands/ui.rs to call tui::launch**

Replace `src/commands/ui.rs`:

```rust
use crate::error::Result;
use crate::tui;

pub fn execute() -> Result<()> {
    tui::launch()
}
```

**Step 8: Run `cargo check` to verify**

Run: `cargo check`
Expected: Compiles successfully

**Step 9: Test the TUI launches**

Run: `cargo run -- ui`
Expected: TUI appears with three columns, Tab switches focus, q quits

**Step 10: Commit**

```bash
git add -A
git commit -m "refactor: restructure TUI as module with three-column layout"
```

---

## Task 4: Implement Next Actions Pane

**Files:**
- Create: `src/tui/next_actions.rs`
- Modify: `src/tui/mod.rs` (add module)
- Modify: `src/tui/app.rs` (add next actions data)
- Modify: `src/tui/ui.rs` (render next actions)

**Step 1: Create next_actions.rs with data structures**

Create `src/tui/next_actions.rs`:

```rust
use crate::models::{Critique, CritiqueStatus, Priority, Problem, Solution, SolutionStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub category: Category,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub title: String,
    pub summary: String,
    pub priority: Priority,
    pub details: Vec<ActionDetail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Blocked,
    Ready,
    Review,
    Waiting,
    Todo,
}

impl Category {
    pub fn sort_order(&self) -> i32 {
        match self {
            Category::Blocked => 0,
            Category::Ready => 1,
            Category::Review => 2,
            Category::Waiting => 3,
            Category::Todo => 4,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Category::Blocked => "BLOCKED",
            Category::Ready => "READY",
            Category::Review => "REVIEW",
            Category::Waiting => "WAITING",
            Category::Todo => "TODO",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Problem,
    Solution,
    Critique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDetail {
    pub id: String,
    pub text: String,
    pub severity: Option<String>,
}

pub fn build_next_actions(
    problems: &[Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    user: &str,
) -> Vec<NextAction> {
    let mut items = Vec::new();

    // 1. BLOCKED: Solutions with open critiques
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let open_critiques: Vec<&Critique> = critiques
            .iter()
            .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
            .collect();

        if !open_critiques.is_empty() {
            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| p.priority.clone()).unwrap_or_default();

            items.push(NextAction {
                category: Category::Blocked,
                entity_type: EntityType::Solution,
                entity_id: solution.id.clone(),
                title: solution.title.clone(),
                summary: format!("{} open critique(s)", open_critiques.len()),
                priority,
                details: open_critiques.iter().map(|c| ActionDetail {
                    id: c.id.clone(),
                    text: c.title.clone(),
                    severity: Some(format!("{}", c.severity)),
                }).collect(),
            });
        }
    }

    // 2. READY: Solutions ready to accept
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let has_open = critiques.iter()
            .any(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open);

        if !has_open && !solution.critique_ids.is_empty() {
            let problem = problems.iter().find(|p| p.id == solution.problem_id);
            let priority = problem.map(|p| p.priority.clone()).unwrap_or_default();

            items.push(NextAction {
                category: Category::Ready,
                entity_type: EntityType::Solution,
                entity_id: solution.id.clone(),
                title: solution.title.clone(),
                summary: "All critiques resolved".to_string(),
                priority,
                details: vec![],
            });
        }
    }

    // 3. REVIEW: Critiques assigned to user
    for critique in critiques.iter().filter(|c| c.status == CritiqueStatus::Open) {
        if let Some(reviewer) = &critique.reviewer {
            if user.contains(reviewer) || reviewer.contains(user) {
                let solution = solutions.iter().find(|s| s.id == critique.solution_id);
                let problem = solution.and_then(|s| problems.iter().find(|p| p.id == s.problem_id));
                let priority = problem.map(|p| p.priority.clone()).unwrap_or_default();

                items.push(NextAction {
                    category: Category::Review,
                    entity_type: EntityType::Critique,
                    entity_id: critique.id.clone(),
                    title: critique.title.clone(),
                    summary: format!("Review on {}", critique.solution_id),
                    priority,
                    details: vec![],
                });
            }
        }
    }

    // 4. TODO: Open problems with no active solutions
    for problem in problems.iter().filter(|p| p.is_open()) {
        let has_active = solutions.iter()
            .any(|s| s.problem_id == problem.id && s.is_active());

        if !has_active {
            items.push(NextAction {
                category: Category::Todo,
                entity_type: EntityType::Problem,
                entity_id: problem.id.clone(),
                title: problem.title.clone(),
                summary: "No solutions proposed".to_string(),
                priority: problem.priority.clone(),
                details: vec![],
            });
        }
    }

    // Sort by category then priority
    items.sort_by(|a, b| {
        let cat_cmp = a.category.sort_order().cmp(&b.category.sort_order());
        if cat_cmp != std::cmp::Ordering::Equal {
            return cat_cmp;
        }
        b.priority.cmp(&a.priority)
    });

    items
}
```

**Step 2: Add module to mod.rs**

In `src/tui/mod.rs`, add after `mod ui;`:

```rust
mod next_actions;

pub use next_actions::{build_next_actions, NextAction};
```

**Step 3: Update app.rs to include next_actions**

In `src/tui/app.rs`, add to struct App:

```rust
    pub next_actions: Vec<super::NextAction>,
```

And in `App::new()`, after loading critiques:

```rust
        let user = store.jj_client.user_identity().unwrap_or_default();
        let next_actions = super::build_next_actions(&problems, &solutions, &critiques, &user);
```

And add to the Self block:

```rust
            next_actions,
```

**Step 4: Update ui.rs to render next actions list**

Replace the Next Actions section in `src/tui/ui.rs`:

```rust
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

    // Main layout: three columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Next Actions
            Constraint::Percentage(35), // Project Tree
            Constraint::Percentage(45), // Detail Pane
        ])
        .split(size);

    draw_next_actions(f, app, main_chunks[0]);
    draw_project_tree(f, app, main_chunks[1]);
    draw_detail(f, app, main_chunks[2]);
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

fn draw_project_tree(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let is_focused = app.focused_pane == FocusedPane::ProjectTree;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tree = Paragraph::new("Project Tree placeholder")
        .block(Block::default()
            .title("Project Tree")
            .borders(Borders::ALL)
            .border_style(border_style));
    f.render_widget(tree, area);
}

fn draw_detail(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let detail = Paragraph::new("Select an item to see details")
        .block(Block::default()
            .title("Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(detail, area);
}
```

**Step 5: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Test TUI with next actions**

Run: `cargo run -- ui`
Expected: Next Actions pane shows items (or "All caught up" if empty)

**Step 7: Commit**

```bash
git add -A
git commit -m "feat(tui): implement Next Actions pane with categories"
```

---

## Task 5: Add Navigation in Next Actions

**Files:**
- Modify: `src/tui/app.rs` (add navigation handling)

**Step 1: Add navigation key handling**

In `src/tui/app.rs`, update `handle_key`:

```rust
    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Char('j') => self.scroll_detail_down(),
            KeyCode::Char('k') => self.scroll_detail_up(),
            KeyCode::Char(' ') => self.page_detail_down(),
            _ => {}
        }
        Ok(())
    }

    fn navigate_up(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index > 0 {
                    self.next_actions_index -= 1;
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index > 0 {
                    self.tree_index -= 1;
                }
            }
        }
    }

    fn navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index < self.next_actions.len().saturating_sub(1) {
                    self.next_actions_index += 1;
                }
            }
            FocusedPane::ProjectTree => {
                // Will implement with tree
                self.tree_index += 1;
            }
        }
    }

    fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(1);
    }

    fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    fn page_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(10);
    }
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Test navigation**

Run: `cargo run -- ui`
Expected: Up/Down arrows move selection in Next Actions, j/k scroll detail

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(tui): add navigation keys for Next Actions"
```

---

## Task 6: Implement Project Tree Data Structure

**Files:**
- Create: `src/tui/tree.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/app.rs`

**Step 1: Create tree.rs with tree node structure**

Create `src/tui/tree.rs`:

```rust
use crate::models::{Critique, CritiqueStatus, Milestone, MilestoneStatus, Problem, ProblemStatus, Solution, SolutionStatus};

#[derive(Debug, Clone)]
pub enum TreeNode {
    Milestone { id: String, title: String, status: MilestoneStatus, expanded: bool },
    Backlog { expanded: bool },
    Problem { id: String, title: String, status: ProblemStatus, expanded: bool },
    Solution { id: String, title: String, status: SolutionStatus, expanded: bool },
    Critique { id: String, title: String, status: CritiqueStatus, severity: String },
}

impl TreeNode {
    pub fn id(&self) -> &str {
        match self {
            TreeNode::Milestone { id, .. } => id,
            TreeNode::Backlog { .. } => "backlog",
            TreeNode::Problem { id, .. } => id,
            TreeNode::Solution { id, .. } => id,
            TreeNode::Critique { id, .. } => id,
        }
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            TreeNode::Milestone { expanded, .. } => *expanded,
            TreeNode::Backlog { expanded } => *expanded,
            TreeNode::Problem { expanded, .. } => *expanded,
            TreeNode::Solution { expanded, .. } => *expanded,
            TreeNode::Critique { .. } => false, // Critiques don't expand
        }
    }

    pub fn set_expanded(&mut self, value: bool) {
        match self {
            TreeNode::Milestone { expanded, .. } => *expanded = value,
            TreeNode::Backlog { expanded } => *expanded = value,
            TreeNode::Problem { expanded, .. } => *expanded = value,
            TreeNode::Solution { expanded, .. } => *expanded = value,
            TreeNode::Critique { .. } => {}
        }
    }

    pub fn can_expand(&self) -> bool {
        !matches!(self, TreeNode::Critique { .. })
    }
}

#[derive(Debug, Clone)]
pub struct FlatTreeItem {
    pub node: TreeNode,
    pub depth: usize,
    pub has_children: bool,
}

pub fn build_flat_tree(
    milestones: &[Milestone],
    problems: &[Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    expanded_nodes: &std::collections::HashSet<String>,
) -> Vec<FlatTreeItem> {
    let mut items = Vec::new();

    // Add milestones
    for milestone in milestones {
        let milestone_problems: Vec<_> = problems.iter()
            .filter(|p| p.milestone_id.as_ref() == Some(&milestone.id))
            .collect();

        let expanded = expanded_nodes.contains(&milestone.id);
        items.push(FlatTreeItem {
            node: TreeNode::Milestone {
                id: milestone.id.clone(),
                title: milestone.title.clone(),
                status: milestone.status.clone(),
                expanded,
            },
            depth: 0,
            has_children: !milestone_problems.is_empty(),
        });

        if expanded {
            add_problems(&mut items, &milestone_problems, solutions, critiques, expanded_nodes, 1);
        }
    }

    // Add backlog (problems without milestone)
    let backlog_problems: Vec<_> = problems.iter()
        .filter(|p| p.milestone_id.is_none())
        .collect();

    let backlog_expanded = expanded_nodes.contains("backlog");
    items.push(FlatTreeItem {
        node: TreeNode::Backlog { expanded: backlog_expanded },
        depth: 0,
        has_children: !backlog_problems.is_empty(),
    });

    if backlog_expanded {
        add_problems(&mut items, &backlog_problems, solutions, critiques, expanded_nodes, 1);
    }

    items
}

fn add_problems(
    items: &mut Vec<FlatTreeItem>,
    problems: &[&Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    expanded_nodes: &std::collections::HashSet<String>,
    depth: usize,
) {
    for problem in problems {
        let problem_solutions: Vec<_> = solutions.iter()
            .filter(|s| s.problem_id == problem.id)
            .collect();

        let expanded = expanded_nodes.contains(&problem.id);
        items.push(FlatTreeItem {
            node: TreeNode::Problem {
                id: problem.id.clone(),
                title: problem.title.clone(),
                status: problem.status.clone(),
                expanded,
            },
            depth,
            has_children: !problem_solutions.is_empty(),
        });

        if expanded {
            for solution in problem_solutions {
                let solution_critiques: Vec<_> = critiques.iter()
                    .filter(|c| c.solution_id == solution.id)
                    .collect();

                let sol_expanded = expanded_nodes.contains(&solution.id);
                items.push(FlatTreeItem {
                    node: TreeNode::Solution {
                        id: solution.id.clone(),
                        title: solution.title.clone(),
                        status: solution.status.clone(),
                        expanded: sol_expanded,
                    },
                    depth: depth + 1,
                    has_children: !solution_critiques.is_empty(),
                });

                if sol_expanded {
                    for critique in solution_critiques {
                        items.push(FlatTreeItem {
                            node: TreeNode::Critique {
                                id: critique.id.clone(),
                                title: critique.title.clone(),
                                status: critique.status.clone(),
                                severity: format!("{}", critique.severity),
                            },
                            depth: depth + 2,
                            has_children: false,
                        });
                    }
                }
            }
        }
    }
}
```

**Step 2: Add module to mod.rs**

In `src/tui/mod.rs`, add:

```rust
mod tree;

pub use tree::{build_flat_tree, FlatTreeItem, TreeNode};
```

**Step 3: Update app.rs with tree state**

Add to App struct:

```rust
    pub tree_items: Vec<super::FlatTreeItem>,
    pub expanded_nodes: std::collections::HashSet<String>,
```

Update `App::new()`:

```rust
        let mut expanded_nodes = std::collections::HashSet::new();
        // Expand first milestone and backlog by default
        if let Some(m) = milestones.first() {
            expanded_nodes.insert(m.id.clone());
        }
        expanded_nodes.insert("backlog".to_string());

        let tree_items = super::build_flat_tree(
            &milestones, &problems, &solutions, &critiques, &expanded_nodes
        );
```

Add to Self:

```rust
            tree_items,
            expanded_nodes,
```

**Step 4: Add rebuild_tree method**

Add to App impl:

```rust
    pub fn rebuild_tree(&mut self) {
        self.tree_items = super::build_flat_tree(
            &self.milestones,
            &self.problems,
            &self.solutions,
            &self.critiques,
            &self.expanded_nodes,
        );
    }
```

**Step 5: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(tui): add project tree data structure"
```

---

## Task 7: Render Project Tree

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update draw_project_tree function**

Replace `draw_project_tree` in `src/tui/ui.rs`:

```rust
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
            TreeNode::Milestone { title, status, .. } => {
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
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Test tree rendering**

Run: `cargo run -- ui`
Expected: Project Tree shows milestones, problems, solutions, critiques with proper indentation

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(tui): render project tree with indentation and colors"
```

---

## Task 8: Add Tree Navigation and Expand/Collapse

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Update handle_key for tree navigation**

Update `handle_key` in app.rs:

```rust
    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Left => self.collapse_or_parent(),
            KeyCode::Right => self.expand_or_child(),
            KeyCode::Char('j') => self.scroll_detail_down(),
            KeyCode::Char('k') => self.scroll_detail_up(),
            KeyCode::Char(' ') => self.page_detail_down(),
            _ => {}
        }
        Ok(())
    }
```

**Step 2: Add expand/collapse methods**

Add to App impl:

```rust
    fn collapse_or_parent(&mut self) {
        if self.focused_pane != FocusedPane::ProjectTree {
            return;
        }

        if let Some(item) = self.tree_items.get(self.tree_index) {
            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Collapse current node
                self.expanded_nodes.remove(&node_id);
                self.rebuild_tree();
            } else if item.depth > 0 {
                // Move to parent
                for i in (0..self.tree_index).rev() {
                    if self.tree_items[i].depth < item.depth {
                        self.tree_index = i;
                        break;
                    }
                }
            }
        }
    }

    fn expand_or_child(&mut self) {
        if self.focused_pane != FocusedPane::ProjectTree {
            return;
        }

        if let Some(item) = self.tree_items.get(self.tree_index) {
            if !item.has_children {
                return;
            }

            let node_id = item.node.id().to_string();

            if item.node.is_expanded() {
                // Move to first child
                if self.tree_index + 1 < self.tree_items.len() {
                    self.tree_index += 1;
                }
            } else {
                // Expand
                self.expanded_nodes.insert(node_id);
                self.rebuild_tree();
            }
        }
    }
```

**Step 3: Update navigate_down to respect tree bounds**

Update `navigate_down`:

```rust
    fn navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index < self.next_actions.len().saturating_sub(1) {
                    self.next_actions_index += 1;
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index < self.tree_items.len().saturating_sub(1) {
                    self.tree_index += 1;
                }
            }
        }
    }
```

**Step 4: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Test expand/collapse**

Run: `cargo run -- ui`
Expected: Tab to tree, arrows navigate, Left/Right expand/collapse

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(tui): add tree expand/collapse with Left/Right arrows"
```

---

## Task 9: Implement Detail Pane

**Files:**
- Create: `src/tui/detail.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/app.rs`
- Modify: `src/tui/ui.rs`

**Step 1: Create detail.rs**

Create `src/tui/detail.rs`:

```rust
use crate::models::{Critique, Milestone, Problem, Solution};

pub enum DetailContent {
    None,
    Problem(Problem),
    Solution(Solution),
    Critique(Critique),
    Milestone(Milestone),
}

impl DetailContent {
    pub fn to_lines(&self) -> Vec<String> {
        match self {
            DetailContent::None => vec!["Select an item to see details".to_string()],
            DetailContent::Problem(p) => {
                let mut lines = vec![
                    format!("Problem: {}", p.id),
                    format!("Title: {}", p.title),
                    format!("Status: {}", p.status),
                    format!("Priority: {}", p.priority),
                    String::new(),
                ];
                if !p.description.is_empty() {
                    lines.push("Description:".to_string());
                    lines.extend(p.description.lines().map(String::from));
                    lines.push(String::new());
                }
                if !p.context.is_empty() {
                    lines.push("Context:".to_string());
                    lines.extend(p.context.lines().map(String::from));
                }
                if let Some(assignee) = &p.assignee {
                    lines.push(format!("Assignee: {}", assignee));
                }
                if let Some(milestone) = &p.milestone_id {
                    lines.push(format!("Milestone: {}", milestone));
                }
                lines
            }
            DetailContent::Solution(s) => {
                let mut lines = vec![
                    format!("Solution: {}", s.id),
                    format!("Title: {}", s.title),
                    format!("Problem: {}", s.problem_id),
                    format!("Status: {}", s.status),
                    String::new(),
                ];
                if !s.approach.is_empty() {
                    lines.push("Approach:".to_string());
                    lines.extend(s.approach.lines().map(String::from));
                    lines.push(String::new());
                }
                if !s.tradeoffs.is_empty() {
                    lines.push("Tradeoffs:".to_string());
                    lines.extend(s.tradeoffs.lines().map(String::from));
                }
                if !s.change_ids.is_empty() {
                    lines.push(String::new());
                    lines.push(format!("Changes: {}", s.change_ids.join(", ")));
                }
                lines
            }
            DetailContent::Critique(c) => {
                let mut lines = vec![
                    format!("Critique: {}", c.id),
                    format!("Title: {}", c.title),
                    format!("Solution: {}", c.solution_id),
                    format!("Status: {}", c.status),
                    format!("Severity: {}", c.severity),
                    String::new(),
                ];
                if !c.argument.is_empty() {
                    lines.push("Argument:".to_string());
                    lines.extend(c.argument.lines().map(String::from));
                    lines.push(String::new());
                }
                if !c.evidence.is_empty() {
                    lines.push("Evidence:".to_string());
                    lines.extend(c.evidence.lines().map(String::from));
                }
                if let Some(file) = &c.file_path {
                    lines.push(String::new());
                    lines.push(format!("Location: {}:{}", file, c.line_start.unwrap_or(0)));
                }
                if !c.replies.is_empty() {
                    lines.push(String::new());
                    lines.push("Replies:".to_string());
                    for reply in &c.replies {
                        lines.push(format!("  {} ({}): {}", reply.author, reply.created_at, reply.body));
                    }
                }
                lines
            }
            DetailContent::Milestone(m) => {
                let mut lines = vec![
                    format!("Milestone: {}", m.id),
                    format!("Title: {}", m.title),
                    format!("Status: {}", m.status),
                ];
                if let Some(date) = &m.target_date {
                    lines.push(format!("Target: {}", date));
                }
                lines.push(String::new());
                if !m.goals.is_empty() {
                    lines.push("Goals:".to_string());
                    lines.extend(m.goals.lines().map(String::from));
                    lines.push(String::new());
                }
                if !m.success_criteria.is_empty() {
                    lines.push("Success Criteria:".to_string());
                    lines.extend(m.success_criteria.lines().map(String::from));
                }
                lines
            }
        }
    }
}
```

**Step 2: Add module to mod.rs**

In `src/tui/mod.rs`:

```rust
mod detail;

pub use detail::DetailContent;
```

**Step 3: Add selected_detail to app.rs**

Add to App struct:

```rust
    pub selected_detail: super::DetailContent,
```

Initialize in new():

```rust
            selected_detail: super::DetailContent::None,
```

**Step 4: Add method to update detail based on selection**

Add to App impl:

```rust
    pub fn update_selected_detail(&mut self) {
        use super::tree::TreeNode;

        // Check focused pane and get relevant selection
        match self.focused_pane {
            FocusedPane::NextActions => {
                if let Some(action) = self.next_actions.get(self.next_actions_index) {
                    self.selected_detail = match action.entity_type {
                        super::next_actions::EntityType::Problem => {
                            self.problems.iter()
                                .find(|p| p.id == action.entity_id)
                                .cloned()
                                .map(super::DetailContent::Problem)
                                .unwrap_or(super::DetailContent::None)
                        }
                        super::next_actions::EntityType::Solution => {
                            self.solutions.iter()
                                .find(|s| s.id == action.entity_id)
                                .cloned()
                                .map(super::DetailContent::Solution)
                                .unwrap_or(super::DetailContent::None)
                        }
                        super::next_actions::EntityType::Critique => {
                            self.critiques.iter()
                                .find(|c| c.id == action.entity_id)
                                .cloned()
                                .map(super::DetailContent::Critique)
                                .unwrap_or(super::DetailContent::None)
                        }
                    };
                }
            }
            FocusedPane::ProjectTree => {
                if let Some(item) = self.tree_items.get(self.tree_index) {
                    self.selected_detail = match &item.node {
                        TreeNode::Milestone { id, .. } => {
                            self.milestones.iter()
                                .find(|m| m.id == *id)
                                .cloned()
                                .map(super::DetailContent::Milestone)
                                .unwrap_or(super::DetailContent::None)
                        }
                        TreeNode::Backlog { .. } => super::DetailContent::None,
                        TreeNode::Problem { id, .. } => {
                            self.problems.iter()
                                .find(|p| p.id == *id)
                                .cloned()
                                .map(super::DetailContent::Problem)
                                .unwrap_or(super::DetailContent::None)
                        }
                        TreeNode::Solution { id, .. } => {
                            self.solutions.iter()
                                .find(|s| s.id == *id)
                                .cloned()
                                .map(super::DetailContent::Solution)
                                .unwrap_or(super::DetailContent::None)
                        }
                        TreeNode::Critique { id, .. } => {
                            self.critiques.iter()
                                .find(|c| c.id == *id)
                                .cloned()
                                .map(super::DetailContent::Critique)
                                .unwrap_or(super::DetailContent::None)
                        }
                    };
                }
            }
        }
        self.detail_scroll = 0; // Reset scroll on new selection
    }
```

**Step 5: Call update_selected_detail after navigation**

Update navigation methods to call update_selected_detail:

```rust
    fn navigate_up(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index > 0 {
                    self.next_actions_index -= 1;
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index > 0 {
                    self.tree_index -= 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index < self.next_actions.len().saturating_sub(1) {
                    self.next_actions_index += 1;
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index < self.tree_items.len().saturating_sub(1) {
                    self.tree_index += 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::NextActions => FocusedPane::ProjectTree,
            FocusedPane::ProjectTree => FocusedPane::NextActions,
        };
        self.update_selected_detail();
    }
```

Also call it in `new()` after building everything:

```rust
        let mut app = Self { ... };
        app.update_selected_detail();
        Ok(app)
```

**Step 6: Update draw_detail in ui.rs**

Replace `draw_detail`:

```rust
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
```

**Step 7: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 8: Test detail pane**

Run: `cargo run -- ui`
Expected: Selecting items shows their details in the right pane

**Step 9: Commit**

```bash
git add -A
git commit -m "feat(tui): implement detail pane with scrolling"
```

---

## Task 10: Add Two-Line Footer

**Files:**
- Modify: `src/tui/ui.rs`
- Modify: `src/tui/app.rs`

**Step 1: Update layout to include footer**

In `src/tui/ui.rs`, update `draw`:

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
```

**Step 2: Add context_hints method to App**

In `src/tui/app.rs`, add:

```rust
    pub fn context_hints(&self) -> String {
        use super::tree::TreeNode;

        match self.focused_pane {
            FocusedPane::NextActions => {
                if let Some(action) = self.next_actions.get(self.next_actions_index) {
                    match action.entity_type {
                        super::next_actions::EntityType::Problem => {
                            format!("{}: [n]ew solution [s]olve [d]issolve [e]dit", action.entity_id)
                        }
                        super::next_actions::EntityType::Solution => {
                            format!("{}: [a]ccept [r]efute [n]ew critique [e]dit", action.entity_id)
                        }
                        super::next_actions::EntityType::Critique => {
                            format!("{}: [a]ddress [d]ismiss [e]dit", action.entity_id)
                        }
                    }
                } else {
                    "No selection".to_string()
                }
            }
            FocusedPane::ProjectTree => {
                if let Some(item) = self.tree_items.get(self.tree_index) {
                    match &item.node {
                        TreeNode::Milestone { id, .. } => {
                            format!("{}: [e]dit", id)
                        }
                        TreeNode::Backlog { .. } => {
                            "[p]roblem new".to_string()
                        }
                        TreeNode::Problem { id, .. } => {
                            format!("{}: [n]ew solution [s]olve [d]issolve [e]dit", id)
                        }
                        TreeNode::Solution { id, .. } => {
                            format!("{}: [a]ccept [r]efute [n]ew critique [e]dit", id)
                        }
                        TreeNode::Critique { id, .. } => {
                            format!("{}: [a]ddress [d]ismiss [e]dit", id)
                        }
                    }
                } else {
                    "No selection".to_string()
                }
            }
        }
    }
```

**Step 3: Add draw_footer function**

In `src/tui/ui.rs`:

```rust
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
```

**Step 4: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Test footer**

Run: `cargo run -- ui`
Expected: Two-line footer shows context-sensitive hints and global shortcuts

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(tui): add two-line footer with context hints"
```

---

## Task 11: Implement One-Way Sync (Next Actions -> Tree)

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add sync_tree_to_selection method**

Add to App impl:

```rust
    fn sync_tree_to_selection(&mut self) {
        if self.focused_pane != FocusedPane::NextActions {
            return;
        }

        if let Some(action) = self.next_actions.get(self.next_actions_index) {
            let target_id = &action.entity_id;

            // Find which nodes need to be expanded to show this item
            // and which index it will be at
            self.expand_to_reveal(target_id);
            self.rebuild_tree();

            // Find the item in the tree
            for (i, item) in self.tree_items.iter().enumerate() {
                if item.node.id() == target_id {
                    self.tree_index = i;
                    break;
                }
            }
        }
    }

    fn expand_to_reveal(&mut self, target_id: &str) {
        // For a solution, we need its problem expanded, and that problem's milestone expanded
        if let Some(solution) = self.solutions.iter().find(|s| s.id == target_id) {
            self.expanded_nodes.insert(solution.problem_id.clone());

            if let Some(problem) = self.problems.iter().find(|p| p.id == solution.problem_id) {
                if let Some(milestone_id) = &problem.milestone_id {
                    self.expanded_nodes.insert(milestone_id.clone());
                } else {
                    self.expanded_nodes.insert("backlog".to_string());
                }
            }
        }

        // For a problem, we need its milestone expanded
        if let Some(problem) = self.problems.iter().find(|p| p.id == target_id) {
            if let Some(milestone_id) = &problem.milestone_id {
                self.expanded_nodes.insert(milestone_id.clone());
            } else {
                self.expanded_nodes.insert("backlog".to_string());
            }
        }

        // For a critique, we need its solution and problem expanded
        if let Some(critique) = self.critiques.iter().find(|c| c.id == target_id) {
            self.expanded_nodes.insert(critique.solution_id.clone());

            if let Some(solution) = self.solutions.iter().find(|s| s.id == critique.solution_id) {
                self.expanded_nodes.insert(solution.problem_id.clone());

                if let Some(problem) = self.problems.iter().find(|p| p.id == solution.problem_id) {
                    if let Some(milestone_id) = &problem.milestone_id {
                        self.expanded_nodes.insert(milestone_id.clone());
                    } else {
                        self.expanded_nodes.insert("backlog".to_string());
                    }
                }
            }
        }
    }
```

**Step 2: Call sync in navigate methods when in NextActions**

Update `navigate_up` and `navigate_down` to call sync after navigation in NextActions:

```rust
    fn navigate_up(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index > 0 {
                    self.next_actions_index -= 1;
                    self.sync_tree_to_selection();
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index > 0 {
                    self.tree_index -= 1;
                }
            }
        }
        self.update_selected_detail();
    }

    fn navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::NextActions => {
                if self.next_actions_index < self.next_actions.len().saturating_sub(1) {
                    self.next_actions_index += 1;
                    self.sync_tree_to_selection();
                }
            }
            FocusedPane::ProjectTree => {
                if self.tree_index < self.tree_items.len().saturating_sub(1) {
                    self.tree_index += 1;
                }
            }
        }
        self.update_selected_detail();
    }
```

**Step 3: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Test sync behavior**

Run: `cargo run -- ui`
Expected: Navigating Next Actions auto-expands and highlights in Project Tree

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(tui): implement one-way sync from Next Actions to Project Tree"
```

---

## Task 12: Add Basic Actions (Accept, Refute, Address, Dismiss)

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add action methods**

Add to App impl:

```rust
    fn accept_solution(&mut self, solution_id: &str) -> Result<()> {
        self.store.accept_solution(solution_id, false)?;
        self.refresh_data()?;
        Ok(())
    }

    fn refute_solution(&mut self, solution_id: &str) -> Result<()> {
        self.store.refute_solution(solution_id)?;
        self.refresh_data()?;
        Ok(())
    }

    fn address_critique(&mut self, critique_id: &str) -> Result<()> {
        self.store.address_critique(critique_id)?;
        self.refresh_data()?;
        Ok(())
    }

    fn dismiss_critique(&mut self, critique_id: &str) -> Result<()> {
        self.store.dismiss_critique(critique_id)?;
        self.refresh_data()?;
        Ok(())
    }

    fn refresh_data(&mut self) -> Result<()> {
        self.milestones = self.store.list_milestones()?;
        self.problems = self.store.list_problems()?;
        self.solutions = self.store.list_solutions()?;
        self.critiques = self.store.list_critiques()?;

        let user = self.store.jj_client.user_identity().unwrap_or_default();
        self.next_actions = super::build_next_actions(&self.problems, &self.solutions, &self.critiques, &user);
        self.rebuild_tree();
        self.update_selected_detail();

        Ok(())
    }
```

**Step 2: Add action key handling**

Update `handle_key`:

```rust
    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Left => self.collapse_or_parent(),
            KeyCode::Right => self.expand_or_child(),
            KeyCode::Char('j') => self.scroll_detail_down(),
            KeyCode::Char('k') => self.scroll_detail_up(),
            KeyCode::Char(' ') => self.page_detail_down(),
            KeyCode::Char('a') => self.handle_action_a()?,
            KeyCode::Char('r') => self.handle_action_r()?,
            KeyCode::Char('d') => self.handle_action_d()?,
            _ => {}
        }
        Ok(())
    }

    fn get_selected_entity(&self) -> Option<(String, super::next_actions::EntityType)> {
        use super::tree::TreeNode;

        match self.focused_pane {
            FocusedPane::NextActions => {
                self.next_actions.get(self.next_actions_index)
                    .map(|a| (a.entity_id.clone(), a.entity_type))
            }
            FocusedPane::ProjectTree => {
                self.tree_items.get(self.tree_index).and_then(|item| {
                    match &item.node {
                        TreeNode::Problem { id, .. } => Some((id.clone(), super::next_actions::EntityType::Problem)),
                        TreeNode::Solution { id, .. } => Some((id.clone(), super::next_actions::EntityType::Solution)),
                        TreeNode::Critique { id, .. } => Some((id.clone(), super::next_actions::EntityType::Critique)),
                        _ => None,
                    }
                })
            }
        }
    }

    fn handle_action_a(&mut self) -> Result<()> {
        use super::next_actions::EntityType;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            match entity_type {
                EntityType::Solution => self.accept_solution(&id)?,
                EntityType::Critique => self.address_critique(&id)?,
                EntityType::Problem => {} // No 'a' action for problems
            }
        }
        Ok(())
    }

    fn handle_action_r(&mut self) -> Result<()> {
        use super::next_actions::EntityType;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Solution {
                self.refute_solution(&id)?;
            }
        }
        Ok(())
    }

    fn handle_action_d(&mut self) -> Result<()> {
        use super::next_actions::EntityType;

        if let Some((id, entity_type)) = self.get_selected_entity() {
            if entity_type == EntityType::Critique {
                self.dismiss_critique(&id)?;
            }
            // For problems, 'd' would be dissolve - add later with input
        }
        Ok(())
    }
```

**Step 3: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Test actions**

Run: `cargo run -- ui`
Expected: Can accept solutions with 'a', refute with 'r', address/dismiss critiques

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(tui): add basic action hotkeys (accept, refute, address, dismiss)"
```

---

## Task 13: Add Flash Message for Feedback

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/ui.rs`

**Step 1: Add flash message state to App**

Add to App struct:

```rust
    pub flash_message: Option<(String, std::time::Instant)>,
```

Initialize in new():

```rust
            flash_message: None,
```

**Step 2: Add show_flash method**

Add to App impl:

```rust
    fn show_flash(&mut self, message: &str) {
        self.flash_message = Some((message.to_string(), std::time::Instant::now()));
    }

    fn clear_expired_flash(&mut self) {
        if let Some((_, time)) = &self.flash_message {
            if time.elapsed() > std::time::Duration::from_secs(2) {
                self.flash_message = None;
            }
        }
    }
```

**Step 3: Call clear_expired_flash in run loop**

In `run`, add before terminal.draw:

```rust
            self.clear_expired_flash();
```

**Step 4: Update action methods to show flash**

Update action methods:

```rust
    fn accept_solution(&mut self, solution_id: &str) -> Result<()> {
        match self.store.accept_solution(solution_id, false) {
            Ok(_) => {
                self.show_flash(&format!("{} accepted", solution_id));
                self.refresh_data()?;
            }
            Err(e) => {
                self.show_flash(&format!("Error: {}", e));
            }
        }
        Ok(())
    }
```

(Similar for other action methods)

**Step 5: Update footer to show flash**

In ui.rs, update `draw_footer`:

```rust
fn draw_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Context line (top) - or flash message if present
    let context_text = if let Some((msg, _)) = &app.flash_message {
        msg.clone()
    } else {
        app.context_hints()
    };
    let context_style = if app.flash_message.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let context = Paragraph::new(context_text).style(context_style);
    f.render_widget(context, chunks[0]);

    // Global shortcuts (bottom)
    let global = Paragraph::new("[Tab] pane | [/] commands | [j/k] scroll | [?] help | [q] quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(global, chunks[1]);
}
```

**Step 6: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 7: Test flash messages**

Run: `cargo run -- ui`
Expected: Actions show brief feedback in footer

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(tui): add flash message feedback for actions"
```

---

## Task 14: Final Integration Test and Cleanup

**Files:**
- None (testing only)

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Test TUI end-to-end**

Run: `cargo run -- ui`
Test:
- Tab switches focus
- Arrow keys navigate
- Left/Right expand/collapse tree
- j/k scroll detail
- Space pages detail
- a/r/d actions work
- Flash messages appear
- q quits

**Step 4: Commit any fixes**

If any fixes needed:
```bash
git add -A
git commit -m "fix: address clippy warnings and test failures"
```

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete jjj ui TUI implementation"
```

---

## Summary

This plan implements the core `jjj ui` TUI in 14 tasks:

1. Add `Ui` command to CLI
2. Remove `Board` command
3. Create TUI module structure
4. Implement Next Actions pane
5. Add navigation in Next Actions
6. Implement Project Tree data structure
7. Render Project Tree
8. Add tree expand/collapse
9. Implement Detail pane
10. Add two-line footer
11. Implement one-way sync
12. Add basic actions
13. Add flash message feedback
14. Final integration test

Future enhancements (not in this plan):
- Command palette (`/`)
- Help overlay (`?`)
- Input mode for new problem/solution/critique
- Error modal dialogs
