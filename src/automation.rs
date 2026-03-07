//! Config-driven automation: fires actions in response to jjj events.
//!
//! Rules are defined in `config.toml` under `[[automation]]`.
//! Each rule matches an `EventType` string and dispatches to a built-in
//! action handler or shell command. Failures print warnings but never
//! block the primary operation.

use std::collections::HashMap;

use crate::context::CommandContext;
use crate::models::AutomationRule;
use crate::models::Event;

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
pub struct AutomationContext {
    vars: HashMap<String, String>,
}

impl AutomationContext {
    pub fn new(event_type: &str) -> Self {
        let mut vars = HashMap::new();
        vars.insert("event".to_string(), event_type.to_string());
        Self { vars }
    }

    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }
}

/// Filter rules that match a given event type string.
fn matching_rules<'a>(
    rules: &'a [AutomationRule],
    event_type: &str,
) -> impl Iterator<Item = &'a AutomationRule> {
    let event_type = event_type.to_string();
    rules
        .iter()
        .filter(move |r| r.enabled && r.on == event_type)
}

/// Replace `{{var}}` placeholders in a template string.
///
/// Unknown variables are left as-is.
fn expand_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

/// Check whether any enabled automation rule exists for the given event type.
///
/// Used by legacy `auto_*` hooks to skip when explicit rules handle the event.
pub fn has_explicit_rule(rules: &[AutomationRule], event_type: &str) -> bool {
    rules.iter().any(|r| r.enabled && r.on == event_type)
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

/// Execute all matching automation rules for an event.
///
/// Called from command handlers after recording the event.
/// Failures print warnings but never block the primary operation.
pub fn run(ctx: &CommandContext, event: &Event, entity_id: &str) {
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

    // Try to populate entity-specific vars by loading from store
    populate_entity_vars(ctx, &event_str, entity_id, &mut auto_ctx);

    for rule in matching_rules(&config.automation, &event_str) {
        let result = if rule.action == "shell" {
            execute_shell(rule, &auto_ctx)
        } else {
            execute_builtin(ctx, rule, entity_id)
        };

        match result {
            AutomationResult::Success(msg) => println!("  (auto: {})", msg),
            AutomationResult::Failure(msg) => {
                eprintln!("  Warning: automation '{}' failed: {}", rule.action, msg)
            }
            AutomationResult::Skipped(_) => {}
        }
    }
}

/// Populate template variables from the entity that triggered the event.
fn populate_entity_vars(
    ctx: &CommandContext,
    event_str: &str,
    entity_id: &str,
    auto_ctx: &mut AutomationContext,
) {
    if event_str.starts_with("problem_") {
        if let Ok(problem) = ctx.store.load_problem(entity_id) {
            auto_ctx.set("title", &problem.title);
            auto_ctx.set("type", "problem");
            if let Some(n) = problem.github_issue {
                auto_ctx.set("issue_number", &n.to_string());
            }
        }
    } else if event_str.starts_with("solution_") {
        if let Ok(solution) = ctx.store.load_solution(entity_id) {
            auto_ctx.set("title", &solution.title);
            auto_ctx.set("type", "solution");
            if let Some(n) = solution.github_pr {
                auto_ctx.set("pr_number", &n.to_string());
            }
            if let Ok(problem) = ctx.store.load_problem(&solution.problem_id) {
                auto_ctx.set("problem.title", &problem.title);
                if let Some(n) = problem.github_issue {
                    auto_ctx.set("issue_number", &n.to_string());
                }
            }
        }
    } else if event_str.starts_with("critique_") {
        if let Ok(critique) = ctx.store.load_critique(entity_id) {
            auto_ctx.set("title", &critique.title);
            auto_ctx.set("type", "critique");
            if let Ok(solution) = ctx.store.load_solution(&critique.solution_id) {
                auto_ctx.set("solution.title", &solution.title);
                if let Some(n) = solution.github_pr {
                    auto_ctx.set("pr_number", &n.to_string());
                }
                if let Ok(problem) = ctx.store.load_problem(&solution.problem_id) {
                    auto_ctx.set("problem.title", &problem.title);
                }
            }
        }
    }
}

/// Execute a built-in GitHub action.
fn execute_builtin(
    ctx: &CommandContext,
    rule: &AutomationRule,
    entity_id: &str,
) -> AutomationResult {
    use crate::sync::hooks;

    match rule.action.as_str() {
        "github_issue" => {
            let mut problem = match ctx.store.load_problem(entity_id) {
                Ok(p) => p,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_create_issue(ctx, &mut problem) {
                Ok(()) => AutomationResult::Success("created GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        "github_close" => {
            let problem = match ctx.store.load_problem(entity_id) {
                Ok(p) => p,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_close_issue(ctx, &problem) {
                Ok(()) => AutomationResult::Success("closed GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        "github_pr" => {
            let mut solution = match ctx.store.load_solution(entity_id) {
                Ok(s) => s,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_create_or_update_pr(ctx, &mut solution) {
                Ok(()) => AutomationResult::Success("created/updated GitHub PR".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        "github_merge" => {
            let solution = match ctx.store.load_solution(entity_id) {
                Ok(s) => s,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_merge_pr(ctx, &solution) {
                Ok(()) => AutomationResult::Success("merged GitHub PR".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        "github_sync" => {
            AutomationResult::Skipped("github_sync not yet implemented as automation".to_string())
        }
        _ => AutomationResult::Failure(format!("Unknown action: {}", rule.action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AutomationRule;

    const BUILTIN_ACTIONS: &[&str] = &[
        "github_pr",
        "github_merge",
        "github_close",
        "github_issue",
        "github_sync",
    ];

    fn execute_rule(rule: &AutomationRule, auto_ctx: &AutomationContext) -> AutomationResult {
        if rule.action == "shell" {
            return execute_shell(rule, auto_ctx);
        }
        if BUILTIN_ACTIONS.contains(&rule.action.as_str()) {
            return AutomationResult::Skipped(format!(
                "{} requires CommandContext (use run() instead)",
                rule.action
            ));
        }
        AutomationResult::Failure(format!("Unknown action: {}", rule.action))
    }

    fn rule(on: &str, action: &str) -> AutomationRule {
        AutomationRule {
            on: on.to_string(),
            action: action.to_string(),
            command: None,
            enabled: true,
        }
    }

    // ── matching_rules ──

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

    // ── expand_template ──

    #[test]
    fn test_expand_template_simple() {
        let mut vars = HashMap::new();
        vars.insert("id".to_string(), "abc123".to_string());
        vars.insert("title".to_string(), "Fix auth bug".to_string());
        let result = expand_template("New: {{title}} ({{id}})", &vars);
        assert_eq!(result, "New: Fix auth bug (abc123)");
    }

    #[test]
    fn test_expand_template_unknown_var_kept() {
        let vars = HashMap::new();
        let result = expand_template("Hello {{unknown}}", &vars);
        assert_eq!(result, "Hello {{unknown}}");
    }

    #[test]
    fn test_expand_template_no_vars() {
        let vars = HashMap::new();
        let result = expand_template("plain text", &vars);
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_expand_template_dotted_vars() {
        let mut vars = HashMap::new();
        vars.insert("problem.title".to_string(), "Auth bug".to_string());
        let result = expand_template("On: {{problem.title}}", &vars);
        assert_eq!(result, "On: Auth bug");
    }

    // ── execute_rule ──

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
            command: Some("true".to_string()),
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
        let r = rule("problem_created", "github_issue");
        let auto_ctx = AutomationContext::new("problem_created");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Skipped(_)));
    }

    // ── has_explicit_rule ──

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
}
