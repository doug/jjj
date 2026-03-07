---
title: Config-Driven Automation Rules
description: Declarative automation that fires on jjj events — no daemon, no polling
---

# Config-Driven Automation Rules

## Problem

jjj requires manual commands for every state transition's side effects: creating PRs after submit, closing issues after solve, notifying teammates after critique. Users forget, and the GitHub state drifts from jjj state.

The existing `auto_push` flag is a blunt instrument — it's all-or-nothing. Users want fine-grained control over which events trigger which actions.

## Design

### Config Format

Automation rules live in `config.toml` (the per-project config in the `jjj` bookmark). Each rule matches an event type and triggers an action:

```toml
[[automation]]
on = "solution_submitted"
action = "github_pr"

[[automation]]
on = "solution_approved"
action = "github_merge"

[[automation]]
on = "problem_solved"
action = "github_close"

[[automation]]
on = "problem_created"
action = "github_issue"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo 'New critique: {{title}} on {{solution.title}}'"
enabled = true  # optional, default true
```

The `on` field matches existing `EventType` variants (snake_case): `problem_created`, `problem_solved`, `problem_dissolved`, `problem_reopened`, `solution_created`, `solution_submitted`, `solution_approved`, `solution_withdrawn`, `critique_raised`, `critique_addressed`, `critique_dismissed`, `critique_validated`, `critique_replied`, `milestone_created`, `milestone_completed`.

### Data Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    /// Event type to match (snake_case, e.g., "solution_submitted")
    pub on: String,

    /// Action to perform: built-in name or "shell"
    pub action: String,

    /// Shell command (required when action = "shell")
    #[serde(default)]
    pub command: Option<String>,

    /// Enable/disable without removing the rule
    #[serde(default = "default_true")]
    pub enabled: bool,
}
```

Added to `ProjectConfig`:

```rust
#[serde(default)]
pub automation: Vec<AutomationRule>,
```

### Built-in Actions

| Action | Typical trigger | Effect |
|--------|----------------|--------|
| `github_pr` | `solution_submitted` | Create/update a GitHub PR for the solution |
| `github_merge` | `solution_approved` | Squash-merge the linked PR |
| `github_close` | `problem_solved`, `problem_dissolved` | Close the linked GitHub issue |
| `github_issue` | `problem_created` | Create a GitHub issue from the problem |
| `github_sync` | any | Run a full sync pull (import reviews, refresh state) |
| `shell` | any | Execute an arbitrary shell command |

Built-in actions reuse the existing functions in `src/sync/hooks.rs`. This refactor extracts them from being hardcoded behind `auto_push` checks into being individually addressable by name.

### Template Variables for Shell Actions

Shell commands support `{{var}}` substitution:

| Variable | Description |
|----------|-------------|
| `{{id}}` | Entity ID that triggered the event |
| `{{title}}` | Entity title |
| `{{type}}` | Entity type (problem, solution, critique) |
| `{{user}}` | User who performed the action |
| `{{event}}` | Event type name |
| `{{problem.title}}` | Parent problem title (for solution/critique events) |
| `{{solution.title}}` | Parent solution title (for critique events) |
| `{{pr_number}}` | GitHub PR number if linked |
| `{{issue_number}}` | GitHub issue number if linked |

Simple string replacement only — no conditionals or loops. The `shell` action is the escape hatch for complex logic.

### Execution Model

Rules fire **synchronously, inline** after the event is recorded, within the same command invocation:

1. User runs `jjj solution submit "fix auth"`
2. jjj transitions solution to Submitted, records `SolutionSubmitted` event
3. jjj checks `config.automation` for rules matching `solution_submitted`
4. Matching rules execute in config file order
5. Command completes

**Error handling**: Automation failures print a warning (`eprintln!`) but never block the primary operation. The local state change always succeeds. This matches the existing `hooks.rs` pattern.

**Deduplication**: Built-in GitHub actions are naturally idempotent (e.g., `github_pr` checks if a PR already exists). Shell actions run every time.

**Ordering**: Multiple rules for the same event run in config file order. A failing rule does not prevent subsequent rules from running.

**Dry-run**: Rules respect the global `--dry-run` flag. `jjj status` could show configured automation rules.

### Backward Compatibility

The existing `auto_push`, `auto_close_on_solve` flags in `GitHubConfig` continue to work. They're equivalent to specific automation rules:

- `auto_push = true` ≈ github_issue on problem_created + github_pr on solution_submitted + github_merge on solution_approved
- `auto_close_on_solve = true` ≈ github_close on problem_solved

If both `auto_push` and explicit automation rules exist, the explicit rules take precedence (auto_push is ignored for event types that have explicit rules). This prevents duplicate actions.

### Implementation Sketch

New file: `src/automation.rs`

```rust
pub fn run_automation(
    ctx: &CommandContext,
    event: &Event,
    entity: &dyn AutomationEntity,
) -> Vec<AutomationResult> {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let results: Vec<_> = config.automation.iter()
        .filter(|rule| rule.enabled && rule.on == event.event_type_str())
        .map(|rule| execute_rule(ctx, rule, event, entity))
        .collect();

    for result in &results {
        match result {
            AutomationResult::Success(msg) => println!("  (auto: {})", msg),
            AutomationResult::Failure(msg) => eprintln!("  Warning: automation failed: {}", msg),
            AutomationResult::Skipped(msg) => {} // silent
        }
    }

    results
}
```

Called from each command handler after the event is created, e.g.:

```rust
// In submit_solution(), after creating the event:
let event = Event::new(EventType::SolutionSubmitted, ...);
automation::run_automation(ctx, &event, &solution);
```

### What This Replaces

The current hardcoded calls in command handlers:
- `hooks::auto_create_issue()` in `new_problem()`
- `hooks::auto_create_or_update_pr()` in `submit_solution()`
- `hooks::auto_close_issue()` in `solve_problem()` / `dissolve_problem()`
- `hooks::auto_merge_pr()` in `finalize_solution()`

These become automation rules. The `hooks.rs` functions remain as the implementation behind the built-in action names, but the dispatch logic moves to `automation.rs`.

## Testing

- Unit tests: rule matching, template variable substitution, config parsing
- Integration tests: verify automation fires on each command
- UXR scenario: end-to-end workflow with automation configured
- Backward compatibility: verify `auto_push = true` still works with no `[[automation]]` rules

## Non-Goals

- No daemon or watch mode (can add later as Approach B)
- No webhook receiver (jjj doesn't run a server)
- No conditional logic in rules (use `shell` for that)
- No jj hook integration (jj hooks are experimental)
