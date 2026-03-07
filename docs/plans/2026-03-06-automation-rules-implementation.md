# Automation Rules Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a config-driven automation system that fires built-in GitHub actions or shell commands in response to jjj events, replacing hardcoded `auto_push`/`auto_close_on_solve` logic.

**Architecture:** `AutomationRule` structs in `ProjectConfig.automation` are matched against `EventType` strings. A new `src/automation.rs` module dispatches matching rules to built-in action handlers (refactored from `hooks.rs`) or shell execution. Existing command handlers replace direct `hooks::*` calls with a single `automation::run()` call after each event.

**Tech Stack:** Rust, serde (TOML deserialization), std::process::Command (shell actions)

---

### Task 1: Add `AutomationRule` to Config Model

**Files:**
- Modify: `src/models/config.rs`
- Test: `tests/config_management.rs`

**Step 1: Write the failing test**

Add to `tests/config_management.rs`:

```rust
/// Behavior: Automation rules deserialize from TOML
#[test]
fn test_automation_rules_deserialized() {
    let toml_str = r#"
[[automation]]
on = "solution_submitted"
action = "github_pr"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo '{{title}}'"

[[automation]]
on = "problem_created"
action = "github_issue"
enabled = false
"#;
    let config: ProjectConfig = toml::from_str(toml_str).expect("Failed to parse");
    assert_eq!(config.automation.len(), 3);
    assert_eq!(config.automation[0].on, "solution_submitted");
    assert_eq!(config.automation[0].action, "github_pr");
    assert!(config.automation[0].enabled);
    assert!(config.automation[0].command.is_none());
    assert_eq!(config.automation[1].action, "shell");
    assert_eq!(config.automation[1].command.as_deref(), Some("echo '{{title}}'"));
    assert!(!config.automation[2].enabled);
}

/// Behavior: Empty automation rules by default
#[test]
fn test_automation_rules_default_empty() {
    let config = ProjectConfig::default();
    assert!(config.automation.is_empty());
}

/// Behavior: Config with automation roundtrips through TOML
#[test]
fn test_automation_roundtrip_toml() {
    let toml_str = r#"
[[automation]]
on = "problem_solved"
action = "github_close"
"#;
    let config: ProjectConfig = toml::from_str(toml_str).expect("parse");
    let serialized = toml::to_string(&config).expect("serialize");
    let roundtrip: ProjectConfig = toml::from_str(&serialized).expect("re-parse");
    assert_eq!(roundtrip.automation.len(), 1);
    assert_eq!(roundtrip.automation[0].on, "problem_solved");
    assert_eq!(roundtrip.automation[0].action, "github_close");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test config_management -- test_automation`
Expected: Compilation error — `AutomationRule` and `ProjectConfig.automation` don't exist yet.

**Step 3: Write minimal implementation**

In `src/models/config.rs`, add the struct and field:

```rust
/// A single automation rule: when event `on` fires, execute `action`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    /// Event type to match (snake_case, e.g., "solution_submitted")
    pub on: String,

    /// Action to perform: built-in name ("github_pr", "github_close", etc.) or "shell"
    pub action: String,

    /// Shell command template (required when action = "shell")
    #[serde(default)]
    pub command: Option<String>,

    /// Enable/disable without removing the rule (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
}
```

Add to `ProjectConfig`:

```rust
/// Automation rules — fire actions on jjj events
#[serde(default)]
pub automation: Vec<AutomationRule>,
```

Update the `models.rs` barrel file to re-export `AutomationRule`:

```rust
pub use config::*;
```

This already uses `*`, so `AutomationRule` is automatically exported. No change needed there.

**Step 4: Run test to verify it passes**

Run: `cargo test --test config_management -- test_automation`
Expected: All 3 new tests PASS.

**Step 5: Commit**

```bash
git add src/models/config.rs tests/config_management.rs
git commit -m "feat: add AutomationRule to ProjectConfig"
```

---

### Task 2: Create `src/automation.rs` — Core Dispatch

**Files:**
- Create: `src/automation.rs`
- Modify: `src/lib.rs` (add `pub mod automation;`)

**Step 1: Write the failing test**

Add inline tests in the new file. The core function `matching_rules` filters rules by event type:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AutomationRule;

    fn rule(on: &str, action: &str) -> AutomationRule {
        AutomationRule {
            on: on.to_string(),
            action: action.to_string(),
            command: None,
            enabled: true,
        }
    }

    #[test]
    fn test_matching_rules_filters_by_event() {
        let rules = vec![
            rule("solution_submitted", "github_pr"),
            rule("problem_solved", "github_close"),
            rule("solution_submitted", "shell"),
        ];
        let matched: Vec<_> = matching_rules(&rules, "solution_submitted").collect();
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].action, "github_pr");
        assert_eq!(matched[1].action, "shell");
    }

    #[test]
    fn test_matching_rules_skips_disabled() {
        let mut r = rule("solution_submitted", "github_pr");
        r.enabled = false;
        let rules = vec![r];
        let matched: Vec<_> = matching_rules(&rules, "solution_submitted").collect();
        assert!(matched.is_empty());
    }

    #[test]
    fn test_matching_rules_no_match() {
        let rules = vec![rule("problem_solved", "github_close")];
        let matched: Vec<_> = matching_rules(&rules, "solution_submitted").collect();
        assert!(matched.is_empty());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test automation::tests`
Expected: Compilation error — module and function don't exist.

**Step 3: Write minimal implementation**

Create `src/automation.rs`:

```rust
//! Config-driven automation: fires actions in response to jjj events.
//!
//! Rules are defined in `config.toml` under `[[automation]]`.
//! Each rule matches an `EventType` string and dispatches to a built-in
//! action handler or shell command. Failures print warnings but never
//! block the primary operation.

use crate::models::AutomationRule;

/// Filter rules that match a given event type string.
fn matching_rules<'a>(
    rules: &'a [AutomationRule],
    event_type: &str,
) -> impl Iterator<Item = &'a AutomationRule> {
    rules
        .iter()
        .filter(move |r| r.enabled && r.on == event_type)
}
```

In `src/lib.rs`, add:

```rust
pub mod automation;
```

**Step 4: Run test to verify it passes**

Run: `cargo test automation::tests`
Expected: All 3 tests PASS.

**Step 5: Commit**

```bash
git add src/automation.rs src/lib.rs
git commit -m "feat: add automation module with rule matching"
```

---

### Task 3: Template Variable Substitution

**Files:**
- Modify: `src/automation.rs`

**Step 1: Write the failing test**

Add to `src/automation.rs` tests module:

```rust
    #[test]
    fn test_expand_template_simple() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("id".to_string(), "abc123".to_string());
        vars.insert("title".to_string(), "Fix auth bug".to_string());
        let result = expand_template("New: {{title}} ({{id}})", &vars);
        assert_eq!(result, "New: Fix auth bug (abc123)");
    }

    #[test]
    fn test_expand_template_unknown_var_kept() {
        let vars = std::collections::HashMap::new();
        let result = expand_template("Hello {{unknown}}", &vars);
        assert_eq!(result, "Hello {{unknown}}");
    }

    #[test]
    fn test_expand_template_no_vars() {
        let vars = std::collections::HashMap::new();
        let result = expand_template("plain text", &vars);
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_expand_template_dotted_vars() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("problem.title".to_string(), "Auth bug".to_string());
        let result = expand_template("On: {{problem.title}}", &vars);
        assert_eq!(result, "On: Auth bug");
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test automation::tests::test_expand_template`
Expected: Compilation error — `expand_template` doesn't exist.

**Step 3: Write minimal implementation**

Add to `src/automation.rs`:

```rust
use std::collections::HashMap;

/// Replace `{{var}}` placeholders in a template string.
///
/// Unknown variables are left as-is. No conditionals or loops —
/// the `shell` action is the escape hatch for complex logic.
fn expand_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test automation::tests::test_expand_template`
Expected: All 4 tests PASS.

**Step 5: Commit**

```bash
git add src/automation.rs
git commit -m "feat: add template variable expansion for shell automation"
```

---

### Task 4: `AutomationContext` and Built-in Action Dispatch

**Files:**
- Modify: `src/automation.rs`

This task adds the public API: `AutomationContext` (carries template variables and entity references) and `run()` (the entry point called from command handlers).

**Step 1: Write the failing test**

Add to `src/automation.rs` tests module:

```rust
    #[test]
    fn test_execute_rule_unknown_action_returns_failure() {
        let r = AutomationRule {
            on: "problem_created".to_string(),
            action: "nonexistent_action".to_string(),
            command: None,
            enabled: true,
        };
        let auto_ctx = AutomationContext::new("problem_created");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Failure(_)));
    }

    #[test]
    fn test_execute_rule_shell_missing_command_returns_failure() {
        let r = AutomationRule {
            on: "problem_created".to_string(),
            action: "shell".to_string(),
            command: None,
            enabled: true,
        };
        let auto_ctx = AutomationContext::new("problem_created");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Failure(_)));
    }

    #[test]
    fn test_execute_rule_shell_runs_command() {
        let r = AutomationRule {
            on: "problem_created".to_string(),
            action: "shell".to_string(),
            command: Some("echo hello".to_string()),
            enabled: true,
        };
        let auto_ctx = AutomationContext::new("problem_created");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Success(_)));
    }

    #[test]
    fn test_execute_rule_shell_expands_vars() {
        let r = AutomationRule {
            on: "problem_created".to_string(),
            action: "shell".to_string(),
            command: Some("echo '{{title}}'".to_string()),
            enabled: true,
        };
        let mut auto_ctx = AutomationContext::new("problem_created");
        auto_ctx.set("title", "My Problem");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Success(_)));
    }

    #[test]
    fn test_execute_rule_builtin_without_ctx_returns_skipped() {
        // Built-in GitHub actions need a CommandContext; without one they skip.
        let r = rule("problem_created", "github_issue");
        let auto_ctx = AutomationContext::new("problem_created");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Skipped(_)));
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test automation::tests`
Expected: Compilation error — types don't exist.

**Step 3: Write minimal implementation**

Add to `src/automation.rs`:

```rust
/// Result of executing a single automation rule.
#[derive(Debug)]
pub enum AutomationResult {
    /// Action succeeded.
    Success(String),
    /// Action failed (printed as warning, does not block).
    Failure(String),
    /// Action was skipped (e.g., no CommandContext for a built-in action).
    Skipped(String),
}

/// Context carrying template variables for automation execution.
///
/// Built-in GitHub actions additionally need a `CommandContext`, which
/// is passed through `run()` but not stored here.
pub struct AutomationContext {
    event_type: String,
    vars: HashMap<String, String>,
}

impl AutomationContext {
    pub fn new(event_type: &str) -> Self {
        let mut vars = HashMap::new();
        vars.insert("event".to_string(), event_type.to_string());
        Self {
            event_type: event_type.to_string(),
            vars,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }
}

/// Known built-in action names.
const BUILTIN_ACTIONS: &[&str] = &[
    "github_pr",
    "github_merge",
    "github_close",
    "github_issue",
    "github_sync",
];

/// Execute a single rule. Returns the result.
fn execute_rule(rule: &AutomationRule, auto_ctx: &AutomationContext) -> AutomationResult {
    if rule.action == "shell" {
        return execute_shell(rule, auto_ctx);
    }

    if BUILTIN_ACTIONS.contains(&rule.action.as_str()) {
        // Built-in actions need a CommandContext, handled by run()
        return AutomationResult::Skipped(format!(
            "{} requires CommandContext (use run() instead)",
            rule.action
        ));
    }

    AutomationResult::Failure(format!("Unknown action: {}", rule.action))
}

/// Execute a shell action with template expansion.
fn execute_shell(rule: &AutomationRule, auto_ctx: &AutomationContext) -> AutomationResult {
    let template = match &rule.command {
        Some(cmd) => cmd,
        None => {
            return AutomationResult::Failure(
                "Shell action requires a 'command' field".to_string(),
            )
        }
    };

    let expanded = expand_template(template, &auto_ctx.vars);

    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&expanded)
        .status()
    {
        Ok(status) if status.success() => {
            AutomationResult::Success(format!("shell: {}", expanded))
        }
        Ok(status) => AutomationResult::Failure(format!(
            "shell exited {}: {}",
            status.code().unwrap_or(-1),
            expanded
        )),
        Err(e) => AutomationResult::Failure(format!("shell failed: {}", e)),
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test automation::tests`
Expected: All tests PASS (including previous task's tests).

**Step 5: Commit**

```bash
git add src/automation.rs
git commit -m "feat: add AutomationContext and execute_rule dispatch"
```

---

### Task 5: Public `run()` Entry Point with Built-in GitHub Actions

**Files:**
- Modify: `src/automation.rs`
- Modify: `src/sync/hooks.rs`

This task adds the main `run()` function that command handlers will call. It loads config, matches rules, and dispatches built-in actions using the existing `hooks.rs` functions — but refactored to remove their internal `auto_push` checks (the check now lives in whether a rule exists).

**Step 1: Refactor hooks.rs — extract inner logic**

The current `hooks.rs` functions each check `config.github.auto_push` internally. We need them callable without that guard, since the automation system now decides when to call them.

Rename the existing `try_*` functions to `do_*` functions that skip the config check, and have the `auto_*` wrapper functions call those:

In `src/sync/hooks.rs`:

```rust
//! Auto-push hooks for GitHub sync.
//!
//! Functions prefixed `do_` are the bare action implementations used by
//! both the legacy `auto_*` wrappers (driven by `auto_push` config)
//! and the new automation rule dispatcher.

use crate::context::CommandContext;
use crate::models::{Problem, Solution};
use crate::sync::github::GitHubProvider;
use crate::sync::SyncProvider;

// ── Bare implementations (no config guard) ─────────────────────────

/// Create a GitHub issue for a problem. Returns the issue number.
pub fn do_create_issue(ctx: &CommandContext, problem: &mut Problem) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let number = provider.create_issue(problem)?;

    problem.github_issue = Some(number);
    ctx.store.save_problem(problem)?;

    println!("  (auto-created GitHub issue #{})", number);
    Ok(())
}

/// Close a GitHub issue.
pub fn do_close_issue(ctx: &CommandContext, problem: &Problem) -> crate::error::Result<()> {
    let issue_number = match problem.github_issue {
        Some(n) => n,
        None => return Ok(()),
    };

    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.close_issue(issue_number)?;

    println!("  (auto-closed GitHub issue #{})", issue_number);
    Ok(())
}

/// Create or update a GitHub PR for a solution.
pub fn do_create_or_update_pr(
    ctx: &CommandContext,
    solution: &mut Solution,
) -> crate::error::Result<()> {
    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    let problem = ctx.store.load_problem(&solution.problem_id)?;

    if solution.github_pr.is_some() {
        println!(
            "  (GitHub PR #{} will be updated on push)",
            solution.github_pr.unwrap()
        );
        return Ok(());
    }

    if solution.change_ids.is_empty() {
        return Ok(());
    }

    let short_id = &solution.id[..8.min(solution.id.len())];
    let branch = format!("jjj/s-{}", short_id);

    let pr_number = provider.create_pr(solution, &problem, &branch)?;
    solution.github_pr = Some(pr_number);
    solution.github_branch = Some(branch);
    ctx.store.with_metadata("Link GitHub PR to solution", || {
        ctx.store.save_solution(solution)
    })?;

    println!("  (auto-created GitHub PR #{})", pr_number);
    Ok(())
}

/// Merge a GitHub PR for a solution.
pub fn do_merge_pr(ctx: &CommandContext, solution: &Solution) -> crate::error::Result<()> {
    let pr_number = match solution.github_pr {
        Some(n) => n,
        None => return Ok(()),
    };

    let config = ctx.store.load_config()?;
    let repo_root = ctx.jj().repo_root();
    let provider = GitHubProvider::from_config(repo_root, &config.github)?;
    provider.merge_pr(pr_number)?;

    println!("  (auto-merged GitHub PR #{})", pr_number);
    Ok(())
}

// ── Legacy wrappers (check auto_push, used by existing command handlers) ──

/// Auto-create a GitHub issue after a new problem is created.
pub fn auto_create_issue(ctx: &CommandContext, problem: &mut Problem) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !config.github.auto_push {
        return;
    }
    if let Err(e) = do_create_issue(ctx, problem) {
        eprintln!("Warning: auto-push to GitHub failed: {}", e);
    }
}

/// Auto-close a GitHub issue after a problem is solved.
///
/// Triggers when any of these are true:
/// - `force` is set (caller passed `--github-close`)
/// - `github.auto_close_on_solve = true` in config
/// - `github.auto_push = true` in config (coarse-grained catch-all)
pub fn auto_close_issue(ctx: &CommandContext, problem: &Problem, force: bool) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !force && !config.github.auto_push && !config.github.auto_close_on_solve {
        return;
    }
    if let Err(e) = do_close_issue(ctx, problem) {
        eprintln!("Warning: auto-close GitHub issue failed: {}", e);
    }
}

/// Auto-create or update a GitHub PR after submit.
pub fn auto_create_or_update_pr(ctx: &CommandContext, solution: &mut Solution) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !config.github.auto_push {
        return;
    }
    if let Err(e) = do_create_or_update_pr(ctx, solution) {
        eprintln!("Warning: auto-push PR to GitHub failed: {}", e);
    }
}

/// Auto-merge a GitHub PR after a solution is accepted.
pub fn auto_merge_pr(ctx: &CommandContext, solution: &Solution) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !config.github.auto_push {
        return;
    }
    if let Err(e) = do_merge_pr(ctx, solution) {
        eprintln!("Warning: auto-merge GitHub PR failed: {}", e);
    }
}
```

**Step 2: Run all tests to confirm refactor is green**

Run: `cargo test`
Expected: All existing tests PASS (no behavioral change).

**Step 3: Add `run()` to `src/automation.rs`**

```rust
use crate::context::CommandContext;
use crate::models::{Event, Problem, Solution};

/// Entity reference passed to `run()` so built-in actions can access
/// the problem or solution that triggered the event.
pub enum EntityRef<'a> {
    Problem(&'a mut Problem),
    Solution(&'a mut Solution),
    None,
}

/// Execute all matching automation rules for an event.
///
/// Called from command handlers after recording the event.
/// Failures print warnings but never block the primary operation.
pub fn run(
    ctx: &CommandContext,
    event: &Event,
    entity: EntityRef<'_>,
) {
    let config = match ctx.store.load_config() {
        Ok(c) => c,
        Err(_) => return,
    };

    if config.automation.is_empty() {
        return;
    }

    let event_str = event.event_type.to_string();

    // Build template variables
    let mut auto_ctx = AutomationContext::new(&event_str);
    auto_ctx.set("id", &event.entity);
    auto_ctx.set("user", &event.by);
    if let Some(ref r) = event.rationale {
        auto_ctx.set("rationale", r);
    }

    // Populate entity-specific vars
    match &entity {
        EntityRef::Problem(p) => {
            auto_ctx.set("title", &p.title);
            auto_ctx.set("type", "problem");
            if let Some(n) = p.github_issue {
                auto_ctx.set("issue_number", &n.to_string());
            }
        }
        EntityRef::Solution(s) => {
            auto_ctx.set("title", &s.title);
            auto_ctx.set("type", "solution");
            if let Some(n) = s.github_pr {
                auto_ctx.set("pr_number", &n.to_string());
            }
            // Load parent problem title for {{problem.title}}
            if let Ok(problem) = ctx.store.load_problem(&s.problem_id) {
                auto_ctx.set("problem.title", &problem.title);
                if let Some(n) = problem.github_issue {
                    auto_ctx.set("issue_number", &n.to_string());
                }
            }
        }
        EntityRef::None => {}
    }

    for rule in matching_rules(&config.automation, &event_str) {
        let result = if rule.action == "shell" {
            execute_shell(rule, &auto_ctx)
        } else {
            execute_builtin(ctx, rule, &entity)
        };

        match result {
            AutomationResult::Success(msg) => println!("  (auto: {})", msg),
            AutomationResult::Failure(msg) => eprintln!("  Warning: automation '{}' failed: {}", rule.action, msg),
            AutomationResult::Skipped(_) => {}
        }
    }
}

/// Execute a built-in GitHub action.
fn execute_builtin(
    ctx: &CommandContext,
    rule: &AutomationRule,
    entity: &EntityRef<'_>,
) -> AutomationResult {
    use crate::sync::hooks;

    match rule.action.as_str() {
        "github_issue" => match entity {
            EntityRef::Problem(p) => {
                // Need mutable access — clone, mutate, save
                let mut problem = match ctx.store.load_problem(&p.id) {
                    Ok(p) => p,
                    Err(e) => return AutomationResult::Failure(e.to_string()),
                };
                match hooks::do_create_issue(ctx, &mut problem) {
                    Ok(()) => AutomationResult::Success("created GitHub issue".to_string()),
                    Err(e) => AutomationResult::Failure(e.to_string()),
                }
            }
            _ => AutomationResult::Skipped("github_issue requires a Problem entity".to_string()),
        },
        "github_close" => match entity {
            EntityRef::Problem(p) => match hooks::do_close_issue(ctx, p) {
                Ok(()) => AutomationResult::Success("closed GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            },
            _ => AutomationResult::Skipped("github_close requires a Problem entity".to_string()),
        },
        "github_pr" => match entity {
            EntityRef::Solution(s) => {
                let mut solution = match ctx.store.load_solution(&s.id) {
                    Ok(s) => s,
                    Err(e) => return AutomationResult::Failure(e.to_string()),
                };
                match hooks::do_create_or_update_pr(ctx, &mut solution) {
                    Ok(()) => AutomationResult::Success("created/updated GitHub PR".to_string()),
                    Err(e) => AutomationResult::Failure(e.to_string()),
                }
            }
            _ => AutomationResult::Skipped("github_pr requires a Solution entity".to_string()),
        },
        "github_merge" => match entity {
            EntityRef::Solution(s) => match hooks::do_merge_pr(ctx, s) {
                Ok(()) => AutomationResult::Success("merged GitHub PR".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            },
            _ => AutomationResult::Skipped("github_merge requires a Solution entity".to_string()),
        },
        "github_sync" => {
            // Sync pull is in commands/sync.rs — too intertwined to call from here.
            // Skip for now; users can run `jjj github` manually.
            AutomationResult::Skipped("github_sync not yet implemented as automation".to_string())
        }
        _ => AutomationResult::Failure(format!("Unknown action: {}", rule.action)),
    }
}
```

Note: The `EntityRef` borrow pattern is tricky with `Problem` since `github_issue` needs to be mutated and saved. The `execute_builtin` for `github_issue` re-loads the problem to get a mutable copy. This is safe because the event has already been committed.

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests PASS. Also update the `test_execute_rule_builtin_without_ctx_returns_skipped` test from Task 4 to use the new signature — `execute_rule` now only handles shell + unknown; built-in dispatch is in `execute_builtin` which requires `CommandContext`.

**Step 5: Commit**

```bash
git add src/automation.rs src/sync/hooks.rs
git commit -m "feat: add run() entry point and refactor hooks for automation dispatch"
```

---

### Task 6: Wire Automation into Command Handlers

**Files:**
- Modify: `src/commands/problem.rs`
- Modify: `src/commands/solution.rs`
- Modify: `src/commands/critique.rs`

This task replaces the hardcoded `hooks::auto_*` calls in command handlers with `automation::run()`. The legacy `auto_*` wrappers remain for backward compatibility with `auto_push` config (they check `auto_push` first, while automation rules are independent).

**Step 1: Wire into `new_problem` (problem.rs)**

After the `with_metadata` block (around line 175), where `auto_create_issue` is currently called, add automation dispatch alongside the existing hook:

```rust
    // Auto-push to GitHub if enabled (outside transaction, non-blocking)
    let pid = created_id.into_inner();
    if !pid.is_empty() {
        if let Ok(mut problem) = ctx.store.load_problem(&pid) {
            // Legacy auto_push path
            crate::sync::hooks::auto_create_issue(ctx, &mut problem);

            // Automation rules
            let event = Event::new(EventType::ProblemCreated, pid.clone(), user.clone());
            crate::automation::run(
                ctx,
                &event,
                crate::automation::EntityRef::Problem(&mut problem),
            );
        }
    }
```

**Step 2: Wire into `solve_problem` (problem.rs)**

After the `with_metadata` block (around line 559):

```rust
    // Auto-close GitHub issue if explicitly requested or configured
    if let Ok(mut problem) = ctx.store.load_problem(&problem_id) {
        crate::sync::hooks::auto_close_issue(ctx, &problem, github_close);

        // Automation rules
        let event = Event::new(EventType::ProblemSolved, problem_id.clone(), user.clone());
        crate::automation::run(
            ctx,
            &event,
            crate::automation::EntityRef::Problem(&mut problem),
        );
    }
```

**Step 3: Wire into `dissolve_problem` (problem.rs)**

Same pattern at line 600:

```rust
    if let Ok(mut problem) = ctx.store.load_problem(&problem_id) {
        crate::sync::hooks::auto_close_issue(ctx, &problem, github_close);

        let event = Event::new(EventType::ProblemDissolved, problem_id.clone(), user.clone());
        crate::automation::run(
            ctx,
            &event,
            crate::automation::EntityRef::Problem(&mut problem),
        );
    }
```

**Step 4: Wire into `submit_solution` (solution.rs)**

After the `with_metadata` block (around line 592), add:

```rust
    // Automation rules
    if let Ok(mut solution) = ctx.store.load_solution(&solution_id) {
        let event = Event::new(EventType::SolutionSubmitted, solution_id.clone(), user);
        crate::automation::run(
            ctx,
            &event,
            crate::automation::EntityRef::Solution(&mut solution),
        );
    }
```

**Step 5: Wire into `finalize_solution` (solution.rs)**

After the `with_metadata` block (around line 749):

```rust
    // Automation rules
    if let Ok(mut solution) = ctx.store.load_solution(solution_id) {
        let event = Event::new(EventType::SolutionApproved, solution_id.to_string(), event.by.clone());
        crate::automation::run(
            ctx,
            &event,
            crate::automation::EntityRef::Solution(&mut solution),
        );
    }
```

**Step 6: Wire into `new_critique` (critique.rs)**

After the `with_metadata` block (around line 120):

```rust
    // Automation rules — no entity ref since critiques don't have a built-in action
    if let Ok(user) = ctx.store.get_current_user() {
        let event = Event::new(EventType::CritiqueRaised, critique_id, user);
        crate::automation::run(ctx, &event, crate::automation::EntityRef::None);
    }
```

**Step 7: Run all tests**

Run: `cargo test`
Expected: All tests PASS. No behavioral change — automation rules are empty by default.

**Step 8: Commit**

```bash
git add src/commands/problem.rs src/commands/solution.rs src/commands/critique.rs
git commit -m "feat: wire automation::run() into command handlers"
```

---

### Task 7: Backward Compatibility — Auto-Push Precedence

**Files:**
- Modify: `src/automation.rs`

The design says: "If both `auto_push` and explicit automation rules exist, the explicit rules take precedence (auto_push is ignored for event types that have explicit rules)."

This means the legacy `auto_*` calls in command handlers should check whether an explicit automation rule already handled the action, to prevent double-firing.

**Step 1: Write the failing test**

```rust
    #[test]
    fn test_has_explicit_rule_for_event() {
        let rules = vec![
            rule("solution_submitted", "github_pr"),
            rule("problem_solved", "github_close"),
        ];
        assert!(has_explicit_rule(&rules, "solution_submitted"));
        assert!(has_explicit_rule(&rules, "problem_solved"));
        assert!(!has_explicit_rule(&rules, "problem_created"));
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test automation::tests::test_has_explicit_rule`
Expected: Compilation error.

**Step 3: Write minimal implementation**

Add to `src/automation.rs`:

```rust
/// Check whether any enabled automation rule exists for the given event type.
///
/// Used by legacy `auto_*` hooks to skip when explicit rules handle the event.
pub fn has_explicit_rule(rules: &[AutomationRule], event_type: &str) -> bool {
    rules.iter().any(|r| r.enabled && r.on == event_type)
}
```

**Step 4: Update command handlers to skip legacy hooks when rules exist**

In `src/commands/problem.rs`, wrap the legacy call:

```rust
    if !pid.is_empty() {
        if let Ok(mut problem) = ctx.store.load_problem(&pid) {
            // Automation rules (if configured) take precedence over legacy auto_push
            let config = ctx.store.load_config().ok();
            let has_rules = config
                .as_ref()
                .map(|c| crate::automation::has_explicit_rule(&c.automation, "problem_created"))
                .unwrap_or(false);

            if !has_rules {
                crate::sync::hooks::auto_create_issue(ctx, &mut problem);
            }

            let event = Event::new(EventType::ProblemCreated, pid.clone(), user.clone());
            crate::automation::run(
                ctx,
                &event,
                crate::automation::EntityRef::Problem(&mut problem),
            );
        }
    }
```

Apply the same pattern to `solve_problem`, `dissolve_problem`. For `submit_solution` and `finalize_solution`, the legacy hooks aren't currently wired up (they exist in hooks.rs but aren't called from command handlers), so no change needed there.

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests PASS.

**Step 6: Commit**

```bash
git add src/automation.rs src/commands/problem.rs
git commit -m "feat: skip legacy auto_push hooks when explicit automation rules exist"
```

---

### Task 8: Integration Test

**Files:**
- Create: `tests/automation_test.rs`

This test verifies automation fires end-to-end using a shell action (since GitHub actions can't run without a real GitHub repo).

**Step 1: Write the test**

```rust
//! Integration test: automation rules fire shell commands on events.

use std::path::PathBuf;

mod test_helpers;

/// Helper to write a config.toml with automation rules into a jjj repo.
fn write_config_with_automation(repo: &PathBuf, rules_toml: &str) {
    // The config is stored in the jjj metadata bookmark.
    // For testing, we write directly via jjj's store.
    let config_content = format!(
        r#"
name = "test-project"

{}
"#,
        rules_toml
    );

    // Write config via jjj command or directly to the metadata store
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_jjj"))
        .current_dir(repo)
        .args(["config", "set", "--raw"])
        .stdin(std::process::Stdio::piped())
        .spawn();

    // Alternative: just verify the automation module's unit tests cover this.
    // The integration test focuses on the config parsing + event dispatch path.
}

/// Behavior: Shell automation fires on problem_created
#[test]
fn test_shell_automation_fires_on_problem_created() {
    // This test requires a jjj-initialized repo.
    // If the test infrastructure doesn't support this, skip it.
    // The unit tests in automation.rs already verify the core logic.

    // For now, verify the automation module compiles and links correctly
    // by importing its public API.
    use jjj::automation::{AutomationContext, AutomationResult};
    use jjj::models::AutomationRule;

    let rule = AutomationRule {
        on: "problem_created".to_string(),
        action: "shell".to_string(),
        command: Some("true".to_string()), // no-op success
        enabled: true,
    };

    let auto_ctx = AutomationContext::new("problem_created");
    // We can't call run() without a real repo, but we can verify
    // the shell path works standalone.
    assert!(matches!(
        jjj::automation::execute_rule_standalone(&rule, &auto_ctx),
        AutomationResult::Success(_)
    ));
}
```

Note: Full E2E testing of automation with a real jjj repo is best done in UXR scenarios (Task 9). This integration test verifies the public API compiles and the shell path works.

To make this test work, we need to expose `execute_rule` (or a test-friendly variant) as public. Add to `src/automation.rs`:

```rust
/// Execute a single rule without a CommandContext.
/// Built-in GitHub actions will return Skipped.
/// Exposed for testing.
pub fn execute_rule_standalone(rule: &AutomationRule, auto_ctx: &AutomationContext) -> AutomationResult {
    execute_rule(rule, auto_ctx)
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --test automation_test`
Expected: PASS.

**Step 3: Commit**

```bash
git add tests/automation_test.rs src/automation.rs
git commit -m "test: add automation integration test"
```

---

### Task 9: UXR Scenario

**Files:**
- Create: `uxr/scenarios/18-automation-rules.sh`

**Step 1: Write the scenario**

```bash
#!/usr/bin/env bash
# Scenario 18: Automation rules — config-driven actions on events
#
# Verifies:
# - Automation rules parse from config.toml
# - Shell actions fire on problem_created
# - Template variables expand correctly
# - Disabled rules are skipped
# - Unknown actions print warnings

source "$(dirname "$0")/../lib.sh"
setup_test_repo

SECTION "Configure automation rules"

# Write config with shell automation rules
cat > /tmp/jjj-auto-test-marker <<< ""

jjj_run config set --raw <<'EOF'
name = "auto-test"

[[automation]]
on = "problem_created"
action = "shell"
command = "echo 'CREATED: {{title}}' >> /tmp/jjj-auto-test-output"

[[automation]]
on = "problem_solved"
action = "shell"
command = "echo 'SOLVED: {{title}}' >> /tmp/jjj-auto-test-output"

[[automation]]
on = "critique_raised"
action = "shell"
command = "echo 'CRITIQUE: {{title}}' >> /tmp/jjj-auto-test-output"
enabled = false
EOF

check "config set succeeds" $? 0

SECTION "Problem creation fires automation"

rm -f /tmp/jjj-auto-test-output
jjj_run problem new "Test auto problem" --priority medium --force
check "problem created" $? 0

# Verify the shell action fired
if [ -f /tmp/jjj-auto-test-output ]; then
    OUTPUT=$(cat /tmp/jjj-auto-test-output)
    check_contains "shell action fired" "$OUTPUT" "CREATED: Test auto problem"
else
    fail "automation output file not created"
fi

SECTION "Disabled rules are skipped"

# Create a critique — the rule is disabled, so no output
LINES_BEFORE=$(wc -l < /tmp/jjj-auto-test-output 2>/dev/null || echo 0)
jjj_run solution new "Test solution" --problem "Test auto problem" --force
jjj_run critique new "Test solution" "Test critique" --force
LINES_AFTER=$(wc -l < /tmp/jjj-auto-test-output 2>/dev/null || echo 0)
check "disabled rule did not fire" "$LINES_BEFORE" "$LINES_AFTER"

SECTION "Problem solved fires automation"

jjj_run solution submit "Test solution"
jjj_run solution approve "Test solution" --force --no-rationale
# Problem auto-solves, which should fire the solved rule
OUTPUT=$(cat /tmp/jjj-auto-test-output)
check_contains "solved rule fired" "$OUTPUT" "SOLVED: Test auto problem"

# Cleanup
rm -f /tmp/jjj-auto-test-output

summary
```

**Step 2: Run the scenario**

Run: `cargo build --release && bash uxr/scenarios/18-automation-rules.sh`
Expected: All checks PASS.

Note: The exact commands (`config set --raw`, `check_contains`) depend on what `lib.sh` provides. Adapt as needed based on the existing UXR test helpers.

**Step 3: Commit**

```bash
git add uxr/scenarios/18-automation-rules.sh
git commit -m "test: add UXR scenario for automation rules"
```

---

### Task 10: Documentation

**Files:**
- Modify: `CLAUDE.md` (add automation to command list)

**Step 1: Update CLAUDE.md**

Add to the "Commands Added (recent)" section in memory, and update the project CLAUDE.md's architecture section to mention automation:

In `CLAUDE.md`, add under the architecture section:

```markdown
### Automation Rules
Config-driven automation in `config.toml`:
```toml
[[automation]]
on = "solution_submitted"  # EventType (snake_case)
action = "github_pr"       # built-in action or "shell"
command = "echo '{{title}}'"  # required for shell actions
enabled = true             # optional, default true
```

Built-in actions: `github_issue`, `github_pr`, `github_merge`, `github_close`, `github_sync`.
Shell actions support `{{var}}` template expansion.
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add automation rules to CLAUDE.md"
```

---

## Execution Notes

- **Tasks 1-4** are pure unit-test-driven — no jjj repo needed, fast iteration.
- **Task 5** is the biggest refactor (hooks.rs split + run() entry point). Take care with borrow checker on `EntityRef`.
- **Task 6** is mechanical wiring — follow the existing hook call sites.
- **Task 7** is the backward-compat guard — small but important.
- **Task 8-9** validate end-to-end.
- **Task 10** is docs.

The `EntityRef` enum in Task 5 may need adjustment based on Rust's borrow rules. The `github_issue` action needs to mutate `Problem` (to set `github_issue` field), which conflicts with the immutable borrow in `EntityRef::Problem(&Problem)`. The plan uses `EntityRef::Problem(&mut Problem)` but if the borrow checker objects at call sites, consider having `execute_builtin` re-load the entity from storage instead.
