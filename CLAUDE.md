# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**jjj** (Jujutsu Juggler) is a distributed project management and code review system built for [Jujutsu (jj)](https://github.com/jj-vcs/jj). It implements Popperian epistemology (Problems → Solutions → Critiques) as a Kanban-style workflow with no central server, no database, and offline-first operation.

**Key insight**: jj's stable Change IDs persist across rebases/squashes, allowing metadata to survive history rewrites that would orphan Git commit-based metadata.

## Build Commands

### Rust CLI
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test <test_name>         # Run specific test
cargo run -- <command>         # Execute jjj command
cargo fmt                      # Format code
cargo clippy                   # Lint
```

### VS Code Extension
```bash
cd vscode
npm install
npm run compile                # TypeScript compilation
npm run watch                  # Watch mode
npm run test                   # Run mocha tests
npm run lint                   # ESLint
npm run package                # Create .vsix
npm run install-ext            # Install locally
```

### Documentation
```bash
cd docs-site && npm run dev    # Serve locally
cd docs-site && npm run build  # Build docs
```

## Architecture

### Core Model
- **Problems**: Things to solve (can form DAG via parent_id). Support tags for categorization.
- **Solutions**: Conjectures attached to jj Change IDs (not commit hashes). Support tags for categorization.
- **Critiques**: Error-elimination feedback that blocks solution approval
- **Milestones**: Time-based goals grouping problems

### Storage: Shadow Graph
All metadata lives in an orphaned `jjj` bookmark, never touching the working copy. Sync via `jj git push -b jjj`.

```
problems/{uuid}.md
solutions/{uuid}.md
critiques/{uuid}.md
milestones/{uuid}.md
config.toml
events.jsonl
```

Entity files use YAML frontmatter + markdown body.

### Automation Rules
Config-driven automation in `config.toml` fires actions on jjj events:
```toml
[[automation]]
on = "solution_submitted"  # EventType (snake_case)
action = "github_pr"       # built-in action or "shell"
command = "echo '{{title}}'"  # required for shell actions
enabled = true             # optional, default true
```
Built-in actions: `github_issue`, `github_pr`, `github_merge`, `github_close`, `github_sync`.
Shell actions support `{{var}}` template expansion (`{{title}}`, `{{id}}`, `{{user}}`, `{{problem.title}}`, `{{pr_number}}`, etc.).

### Component Layers
```
CLI (src/commands/)           # Clap-based command handlers
    ↓
Automation (src/automation.rs) # Config-driven event→action dispatch
    ↓
Storage (src/storage.rs)      # MetadataStore: CRUD, YAML parsing (~30KB)
    ↓
JJ Integration (src/jj.rs)    # JjClient: subprocess wrapper
    ↓
TUI (src/tui/)               # Ratatui-based interactive UI
```

### Entity IDs
- All entities use UUID7 identifiers (e.g., "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a")
- UUIDs are time-ordered for natural chronological sorting
- Users can reference entities by:
  - Full UUID
  - Truncated hex prefix (minimum 6 chars, e.g., "01957d")
  - Fuzzy title match (e.g., "auth bug")
- Listings show short prefixes auto-extended for uniqueness
- Mixed-type listings use type prefixes: p/, s/, c/, m/
- Change IDs: jj-native opaque strings (e.g., "kpqxywon")

### State Machines
- **Problems**: Open → InProgress → Solved/Dissolved
- **Solutions**: Proposed → Submitted → Approved/Withdrawn
- **Critiques**: Open → Addressed/Valid/Dismissed

## Key Files

- `src/cli.rs` - CLI structure (all commands defined here)
- `src/storage/mod.rs` - Critical storage layer with YAML frontmatter parsing
- `src/commands/*.rs` - Individual command implementations
- `src/models/*.rs` - Data structures with serde derives
- `src/tui/app/mod.rs` - TUI state machine and key handlers
- `src/jj.rs` - Jujutsu subprocess integration

## Adding a New Command

1. Add enum variant in `src/cli.rs` (Commands enum)
2. Create handler in `src/commands/{command}.rs`
3. Add dispatch case in `src/commands/mod.rs::execute()`
4. Add tests in `tests/`

## TUI Navigation

- `Tab`: Switch between NextActions and ProjectTree panes
- Arrow keys: Navigate within pane
- `j/k`: Scroll detail pane
- `Left/Right`: Collapse/expand tree nodes
- `a/r/d`: Approve/Withdraw/Dismiss actions

### Events, Insights, and Timeline
```bash
jjj events                           # Recent events
jjj events --problem 01957d          # Events for a problem (by prefix)
jjj timeline "auth bug"              # Full timeline (by fuzzy title)
jjj insights                         # Aggregate stats (approval rate, cycle times)
jjj overlaps                         # Files touched by multiple solutions
jjj next --claim                     # Claim top work item (assign + display)
```
