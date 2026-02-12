# TUI Input and Help Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add help overlay, inline item creation, and $EDITOR integration to the jjj TUI.

**Architecture:** Extend UiState with InputMode enum (Normal/Help/Input). Key handlers route based on mode. UI renders overlays and input line conditionally. Item creation reuses existing storage methods.

**Tech Stack:** Rust, ratatui, crossterm

---

## Task 1: Add InputMode and InputAction Types

**Files:**
- Modify: `src/tui/app.rs:1-50`

**Step 1: Add the new enums after the existing imports**

Add these types after line 11 (after the existing use statements):

```rust
use super::next_actions::EntityType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Help,
    Input {
        prompt: String,
        buffer: String,
        action: InputAction,
    },
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Normal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    NewProblem { milestone_id: Option<String> },
    NewSolution { problem_id: String },
    NewCritique { solution_id: String },
    EditTitle { entity_type: EntityType, entity_id: String },
}
```

**Step 2: Add input_mode field to UiState**

In the `UiState` struct (around line 38), add:

```rust
pub input_mode: InputMode,
```

**Step 3: Initialize input_mode in UiState::new()**

In `UiState::new()` (around line 57), add to the Self initialization:

```rust
input_mode: InputMode::Normal,
```

**Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): add InputMode and InputAction types"
```

---

## Task 2: Export EntityType from tui module

**Files:**
- Modify: `src/tui/mod.rs`

**Step 1: Update the pub use statement for next_actions**

Change line 9 from:
```rust
pub use next_actions::{build_next_actions, NextAction};
```
To:
```rust
pub use next_actions::{build_next_actions, EntityType, NextAction};
```

**Step 2: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/tui/mod.rs
git commit -m "feat(tui): export EntityType from tui module"
```

---

## Task 3: Implement Help Mode Key Handler

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add ? key handler in handle_key**

In `handle_key` method (around line 171), add a new match arm after the existing ones:

```rust
KeyCode::Char('?') => self.toggle_help(),
```

**Step 2: Add toggle_help method**

Add this method after `toggle_related_panel` (around line 375):

```rust
fn toggle_help(&mut self) {
    self.ui.input_mode = match &self.ui.input_mode {
        InputMode::Help => InputMode::Normal,
        _ => InputMode::Help,
    };
}
```

**Step 3: Modify handle_key to check input mode first**

Wrap the existing match in `handle_key` to check input mode. Replace the entire `handle_key` method:

```rust
fn handle_key(&mut self, key: KeyCode) -> Result<()> {
    match &self.ui.input_mode {
        InputMode::Help => {
            // Any key exits help
            self.ui.input_mode = InputMode::Normal;
        }
        InputMode::Input { .. } => {
            self.handle_input_key(key)?;
        }
        InputMode::Normal => {
            self.handle_normal_key(key)?;
        }
    }
    Ok(())
}
```

**Step 4: Rename existing key handler to handle_normal_key**

Create a new method `handle_normal_key` with the original match contents:

```rust
fn handle_normal_key(&mut self, key: KeyCode) -> Result<()> {
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
        KeyCode::Char('R') => self.toggle_related_panel(),
        KeyCode::Char('?') => self.toggle_help(),
        _ => {}
    }
    Ok(())
}
```

**Step 5: Add stub for handle_input_key**

```rust
fn handle_input_key(&mut self, _key: KeyCode) -> Result<()> {
    // Will be implemented in Task 5
    Ok(())
}
```

**Step 6: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): add help mode key handling"
```

---

## Task 4: Render Help Overlay

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Add Clear widget import**

Update the ratatui imports at the top to include Clear:

```rust
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
```

**Step 2: Add InputMode import**

Add after the existing super imports:

```rust
use super::app::InputMode;
```

**Step 3: Call help overlay renderer in draw function**

At the end of the `draw` function (before the closing brace), add:

```rust
// Draw overlays last (on top)
if matches!(app.ui.input_mode, InputMode::Help) {
    draw_help_overlay(f, app);
}
```

**Step 4: Add the draw_help_overlay function**

Add at the end of the file:

```rust
fn draw_help_overlay(f: &mut Frame, app: &App) {
    use super::tree::TreeNode;

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
```

**Step 5: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 6: Manual test**

Run: `cargo run -- ui`
Press `?` - should show centered help popup
Press any key - should close

**Step 7: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): render context-sensitive help overlay"
```

---

## Task 5: Implement Input Mode Key Handler

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Implement handle_input_key**

Replace the stub `handle_input_key` with:

```rust
fn handle_input_key(&mut self, key: KeyCode) -> Result<()> {
    // Extract current input state
    let (prompt, buffer, action) = match &self.ui.input_mode {
        InputMode::Input { prompt, buffer, action } => {
            (prompt.clone(), buffer.clone(), action.clone())
        }
        _ => return Ok(()),
    };

    match key {
        KeyCode::Esc => {
            self.ui.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            if !buffer.is_empty() {
                self.execute_input_action(&action, &buffer)?;
            }
            self.ui.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            let mut new_buffer = buffer;
            new_buffer.pop();
            self.ui.input_mode = InputMode::Input {
                prompt,
                buffer: new_buffer,
                action,
            };
        }
        KeyCode::Char(c) => {
            let mut new_buffer = buffer;
            new_buffer.push(c);
            self.ui.input_mode = InputMode::Input {
                prompt,
                buffer: new_buffer,
                action,
            };
        }
        _ => {}
    }
    Ok(())
}
```

**Step 2: Add execute_input_action stub**

```rust
fn execute_input_action(&mut self, action: &InputAction, title: &str) -> Result<()> {
    match action {
        InputAction::NewProblem { milestone_id } => {
            self.create_problem(title, milestone_id.clone())?;
        }
        InputAction::NewSolution { problem_id } => {
            self.create_solution(title, problem_id)?;
        }
        InputAction::NewCritique { solution_id } => {
            self.create_critique(title, solution_id)?;
        }
        InputAction::EditTitle { entity_type, entity_id } => {
            self.update_title(entity_type, entity_id, title)?;
        }
    }
    Ok(())
}
```

**Step 3: Add placeholder create/update methods**

```rust
fn create_problem(&mut self, title: &str, milestone_id: Option<String>) -> Result<()> {
    use crate::models::Problem;

    let mut problem = Problem::new(title.to_string());
    problem.milestone_id = milestone_id;

    let id = problem.id.clone();
    self.store.with_metadata(&format!("Create problem: {}", title), || {
        self.store.save_problem(&problem)
    })?;

    self.show_flash(&format!("Created {}", id));
    self.refresh_data()?;
    Ok(())
}

fn create_solution(&mut self, title: &str, problem_id: &str) -> Result<()> {
    use crate::models::Solution;

    let mut solution = Solution::new(title.to_string(), problem_id.to_string());
    let id = solution.id.clone();

    self.store.with_metadata(&format!("Create solution: {}", title), || {
        self.store.save_solution(&solution)
    })?;

    self.show_flash(&format!("Created {}", id));
    self.refresh_data()?;
    Ok(())
}

fn create_critique(&mut self, title: &str, solution_id: &str) -> Result<()> {
    use crate::models::Critique;

    let critique = Critique::new(title.to_string(), solution_id.to_string());
    let id = critique.id.clone();

    self.store.with_metadata(&format!("Create critique: {}", title), || {
        self.store.save_critique(&critique)
    })?;

    self.show_flash(&format!("Created {}", id));
    self.refresh_data()?;
    Ok(())
}

fn update_title(&mut self, entity_type: &EntityType, entity_id: &str, new_title: &str) -> Result<()> {
    match entity_type {
        EntityType::Problem => {
            self.store.with_metadata(&format!("Update problem title: {}", new_title), || {
                let mut problem = self.store.load_problem(entity_id)?;
                problem.title = new_title.to_string();
                self.store.save_problem(&problem)
            })?;
        }
        EntityType::Solution => {
            self.store.with_metadata(&format!("Update solution title: {}", new_title), || {
                let mut solution = self.store.load_solution(entity_id)?;
                solution.title = new_title.to_string();
                self.store.save_solution(&solution)
            })?;
        }
        EntityType::Critique => {
            self.store.with_metadata(&format!("Update critique title: {}", new_title), || {
                let mut critique = self.store.load_critique(entity_id)?;
                critique.title = new_title.to_string();
                self.store.save_critique(&critique)
            })?;
        }
    }

    self.show_flash(&format!("Updated {}", entity_id));
    self.refresh_data()?;
    Ok(())
}
```

**Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): implement input mode key handling and CRUD operations"
```

---

## Task 6: Add n/e Key Handlers for Input Mode Entry

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add n and e handlers to handle_normal_key**

In `handle_normal_key`, add these match arms:

```rust
KeyCode::Char('n') => self.start_new_item()?,
KeyCode::Char('e') => self.start_edit_title()?,
KeyCode::Char('s') => self.handle_action_s()?,
KeyCode::Char('v') => self.handle_action_v()?,
```

**Step 2: Implement start_new_item**

```rust
fn start_new_item(&mut self) -> Result<()> {
    use super::tree::TreeNode;

    let (prompt, action) = match self.ui.focused_pane {
        FocusedPane::NextActions => {
            if let Some(na) = self.cache.next_actions.get(self.ui.next_actions_index) {
                match na.entity_type {
                    EntityType::Problem => (
                        "New solution title: ".to_string(),
                        InputAction::NewSolution { problem_id: na.entity_id.clone() },
                    ),
                    EntityType::Solution => (
                        "New critique title: ".to_string(),
                        InputAction::NewCritique { solution_id: na.entity_id.clone() },
                    ),
                    EntityType::Critique => return Ok(()), // Can't create child of critique
                }
            } else {
                return Ok(());
            }
        }
        FocusedPane::ProjectTree => {
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Milestone { id, .. } => (
                        "New problem title: ".to_string(),
                        InputAction::NewProblem { milestone_id: Some(id.clone()) },
                    ),
                    TreeNode::Backlog { .. } => (
                        "New problem title: ".to_string(),
                        InputAction::NewProblem { milestone_id: None },
                    ),
                    TreeNode::Problem { id, .. } => (
                        "New solution title: ".to_string(),
                        InputAction::NewSolution { problem_id: id.clone() },
                    ),
                    TreeNode::Solution { id, .. } => (
                        "New critique title: ".to_string(),
                        InputAction::NewCritique { solution_id: id.clone() },
                    ),
                    TreeNode::Critique { .. } => return Ok(()),
                }
            } else {
                return Ok(());
            }
        }
    };

    self.ui.input_mode = InputMode::Input {
        prompt,
        buffer: String::new(),
        action,
    };
    Ok(())
}
```

**Step 3: Implement start_edit_title**

```rust
fn start_edit_title(&mut self) -> Result<()> {
    use super::tree::TreeNode;

    let (prompt, action, current_title) = match self.ui.focused_pane {
        FocusedPane::NextActions => {
            if let Some(na) = self.cache.next_actions.get(self.ui.next_actions_index) {
                (
                    "Edit title: ".to_string(),
                    InputAction::EditTitle {
                        entity_type: na.entity_type,
                        entity_id: na.entity_id.clone(),
                    },
                    na.title.clone(),
                )
            } else {
                return Ok(());
            }
        }
        FocusedPane::ProjectTree => {
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Problem,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Solution { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Solution,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    TreeNode::Critique { id, title, .. } => (
                        "Edit title: ".to_string(),
                        InputAction::EditTitle {
                            entity_type: EntityType::Critique,
                            entity_id: id.clone(),
                        },
                        title.clone(),
                    ),
                    _ => return Ok(()),
                }
            } else {
                return Ok(());
            }
        }
    };

    self.ui.input_mode = InputMode::Input {
        prompt,
        buffer: current_title,
        action,
    };
    Ok(())
}
```

**Step 4: Add handle_action_s and handle_action_v**

```rust
fn handle_action_s(&mut self) -> Result<()> {
    use crate::models::ProblemStatus;

    if let Some((id, entity_type)) = self.get_selected_entity() {
        if entity_type == EntityType::Problem {
            let id_clone = id.clone();
            match self.store.with_metadata(&format!("Solve problem {}", id), || {
                let mut problem = self.store.load_problem(&id)?;
                problem.set_status(ProblemStatus::Solved);
                self.store.save_problem(&problem)
            }) {
                Ok(_) => {
                    self.show_flash(&format!("{} solved", id_clone));
                    self.refresh_data()?;
                }
                Err(e) => {
                    self.show_flash(&format!("Error: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_action_v(&mut self) -> Result<()> {
    if let Some((id, entity_type)) = self.get_selected_entity() {
        if entity_type == EntityType::Critique {
            let id_clone = id.clone();
            match self.store.with_metadata(&format!("Validate critique {}", id), || {
                let mut critique = self.store.load_critique(&id)?;
                critique.validate();
                self.store.save_critique(&critique)
            }) {
                Ok(_) => {
                    self.show_flash(&format!("{} validated", id_clone));
                    self.refresh_data()?;
                }
                Err(e) => {
                    self.show_flash(&format!("Error: {}", e));
                }
            }
        }
    }
    Ok(())
}
```

**Step 5: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): add n/e/s/v key handlers for item creation and editing"
```

---

## Task 7: Render Input Line

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Modify draw function to handle input mode**

In the `draw` function, after drawing the footer and before the help overlay check, add input line handling:

```rust
// Draw input line if in input mode (replaces footer)
if let InputMode::Input { prompt, buffer, .. } = &app.ui.input_mode {
    draw_input_line(f, prompt, buffer, vertical_chunks[1]);
} else {
    draw_footer(f, app, vertical_chunks[1]);
}
```

Also change the footer call to be conditional - wrap the existing `draw_footer` call inside the else branch above (remove the standalone call).

**Step 2: Add draw_input_line function**

```rust
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
```

**Step 3: Update draw function layout**

The draw function needs adjustment to conditionally show footer or input. Replace the footer rendering section:

```rust
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
```

**Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 5: Manual test**

Run: `cargo run -- ui`
Navigate to a problem, press `n` - should show input line
Type some text - should appear
Press Esc - should cancel
Press `n` again, type text, press Enter - should create solution

**Step 6: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): render input line for item creation"
```

---

## Task 8: Add $EDITOR Integration

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Add E key handler to handle_normal_key**

In `handle_normal_key`, add:

```rust
KeyCode::Char('E') => self.open_in_editor()?,
```

**Step 2: Add open_in_editor method**

```rust
fn open_in_editor(&mut self) -> Result<()> {
    use super::tree::TreeNode;
    use std::io::Write;
    use std::process::Command;

    // Get selected entity
    let (entity_type, entity_id) = match self.ui.focused_pane {
        FocusedPane::NextActions => {
            if let Some(na) = self.cache.next_actions.get(self.ui.next_actions_index) {
                (na.entity_type, na.entity_id.clone())
            } else {
                return Ok(());
            }
        }
        FocusedPane::ProjectTree => {
            if let Some(item) = self.cache.tree_items.get(self.ui.tree_index) {
                match &item.node {
                    TreeNode::Problem { id, .. } => (EntityType::Problem, id.clone()),
                    TreeNode::Solution { id, .. } => (EntityType::Solution, id.clone()),
                    TreeNode::Critique { id, .. } => (EntityType::Critique, id.clone()),
                    _ => return Ok(()),
                }
            } else {
                return Ok(());
            }
        }
    };

    // Serialize entity to temp file
    let temp_path = std::env::temp_dir().join(format!("jjj-edit-{}.md", &entity_id[..8.min(entity_id.len())]));
    let original_content = self.serialize_entity_for_edit(&entity_type, &entity_id)?;
    std::fs::write(&temp_path, &original_content)?;

    // Get editor
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    // Signal that we need to suspend
    self.editor_request = Some(EditorRequest {
        entity_type,
        entity_id,
        temp_path,
        original_content,
        editor,
    });

    Ok(())
}
```

**Step 3: Add EditorRequest struct and field**

Add after InputAction enum:

```rust
#[derive(Debug, Clone)]
pub struct EditorRequest {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub temp_path: std::path::PathBuf,
    pub original_content: String,
    pub editor: String,
}
```

Add to App struct:

```rust
pub editor_request: Option<EditorRequest>,
```

Initialize in App::new():

```rust
editor_request: None,
```

**Step 4: Add serialize_entity_for_edit method**

```rust
fn serialize_entity_for_edit(&self, entity_type: &EntityType, entity_id: &str) -> Result<String> {
    match entity_type {
        EntityType::Problem => {
            let problem = self.store.load_problem(entity_id)?;
            Ok(format!(
                "---\ntitle: {}\nstatus: {:?}\npriority: {}\n---\n\n## Description\n\n{}\n",
                problem.title,
                problem.status,
                problem.priority,
                problem.description.as_deref().unwrap_or("")
            ))
        }
        EntityType::Solution => {
            let solution = self.store.load_solution(entity_id)?;
            Ok(format!(
                "---\ntitle: {}\nstatus: {:?}\n---\n\n## Description\n\n{}\n",
                solution.title,
                solution.status,
                solution.description.as_deref().unwrap_or("")
            ))
        }
        EntityType::Critique => {
            let critique = self.store.load_critique(entity_id)?;
            Ok(format!(
                "---\ntitle: {}\nstatus: {:?}\nseverity: {}\n---\n\n## Description\n\n{}\n",
                critique.title,
                critique.status,
                critique.severity,
                critique.body.as_deref().unwrap_or("")
            ))
        }
    }
}
```

**Step 5: Modify run loop to handle editor request**

In the `run` method, modify the loop to check for editor requests:

```rust
pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
    while !self.should_quit {
        // Check for editor request
        if let Some(request) = self.editor_request.take() {
            self.run_editor(terminal, request)?;
            continue;
        }

        self.clear_expired_flash();
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
```

**Step 6: Add run_editor method**

```rust
fn run_editor<B: Backend>(&mut self, terminal: &mut Terminal<B>, request: EditorRequest) -> Result<()> {
    use crossterm::{execute, terminal::{LeaveAlternateScreen, EnterAlternateScreen}};
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use std::process::Command;

    // Leave alternate screen
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    // Run editor
    let status = Command::new(&request.editor)
        .arg(&request.temp_path)
        .status();

    // Re-enter alternate screen
    crossterm::terminal::enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.clear()?;

    // Process result
    match status {
        Ok(exit_status) if exit_status.success() => {
            let new_content = std::fs::read_to_string(&request.temp_path)?;
            if new_content == request.original_content {
                self.show_flash("No changes");
            } else {
                self.apply_edited_content(&request.entity_type, &request.entity_id, &new_content)?;
                self.show_flash(&format!("Updated {}", request.entity_id));
            }
        }
        Ok(_) => {
            self.show_flash("Edit cancelled");
        }
        Err(e) => {
            self.show_flash(&format!("Editor error: {}", e));
        }
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&request.temp_path);

    Ok(())
}
```

**Step 7: Add apply_edited_content method**

```rust
fn apply_edited_content(&mut self, entity_type: &EntityType, entity_id: &str, content: &str) -> Result<()> {
    // Simple parsing: extract title from frontmatter, description from body
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err("Invalid format".into());
    }

    let frontmatter = parts[1].trim();
    let body = parts[2].trim();

    // Extract title from frontmatter
    let title = frontmatter
        .lines()
        .find(|l| l.starts_with("title:"))
        .map(|l| l.trim_start_matches("title:").trim().to_string())
        .unwrap_or_default();

    // Extract description from body (after ## Description header)
    let description = body
        .strip_prefix("## Description")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match entity_type {
        EntityType::Problem => {
            self.store.with_metadata(&format!("Edit problem {}", entity_id), || {
                let mut problem = self.store.load_problem(entity_id)?;
                problem.title = title.clone();
                problem.description = description.clone();
                self.store.save_problem(&problem)
            })?;
        }
        EntityType::Solution => {
            self.store.with_metadata(&format!("Edit solution {}", entity_id), || {
                let mut solution = self.store.load_solution(entity_id)?;
                solution.title = title.clone();
                solution.description = description.clone();
                self.store.save_solution(&solution)
            })?;
        }
        EntityType::Critique => {
            self.store.with_metadata(&format!("Edit critique {}", entity_id), || {
                let mut critique = self.store.load_critique(entity_id)?;
                critique.title = title.clone();
                critique.body = description.clone();
                self.store.save_critique(&critique)
            })?;
        }
    }

    self.refresh_data()?;
    Ok(())
}
```

**Step 8: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully

**Step 9: Manual test**

Run: `cargo run -- ui`
Navigate to an item, press `E`
Editor should open with item content
Make a change, save and quit
TUI should resume with "Updated" message

**Step 10: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui): add \$EDITOR integration for full editing"
```

---

## Task 9: Final Integration Test

**Files:**
- None (manual testing)

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Manual workflow test**

Run: `cargo run -- ui`

Test sequence:
1. Press `?` - help appears, shows context actions
2. Press any key - help closes
3. Navigate to Backlog, press `n` - input line appears
4. Type "Test problem", press Enter - problem created
5. Navigate to new problem, press `n` - input line for solution
6. Type "Test solution", press Enter - solution created
7. Navigate to solution, press `e` - edit title input
8. Change title, press Enter - title updated
9. Press `E` - editor opens
10. Make change, save - content updated
11. Press `a` - solution accepted
12. Press `q` - exit

**Step 3: Commit final state**

```bash
git add -A
git commit -m "feat(tui): complete input and help implementation"
```

---

## Summary

This plan implements:
- Help overlay with `?` key (context-sensitive)
- Item creation with `n` key (contextual parent)
- Title editing with `e` key (inline input)
- Full editing with `E` key ($EDITOR integration)
- Additional actions: `s` (solve), `v` (validate)

Total: 9 tasks, ~8 commits
