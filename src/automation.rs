//! Config-driven automation: fires actions in response to jjj events.
//!
//! Rules are defined in `config.toml` under `[[automation]]`.
//! Each rule matches an `EventType` and dispatches to a built-in
//! action handler or shell command. Failures print warnings but never
//! block the primary operation.

use std::collections::HashMap;

use crate::context::CommandContext;
use crate::models::{AutomationAction, AutomationRule, Event, EventType};

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
#[derive(Debug, Clone)]
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

/// Filter rules that match a given event type.
fn matching_rules<'a>(
    rules: &'a [AutomationRule],
    event_type: &EventType,
) -> Vec<&'a AutomationRule> {
    rules
        .iter()
        .filter(|r| r.enabled && r.on == *event_type)
        .collect()
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
pub fn has_explicit_rule(rules: &[AutomationRule], event_type: &EventType) -> bool {
    rules.iter().any(|r| r.enabled && r.on == *event_type)
}

/// Execute a shell action with template expansion.
fn execute_shell(rule: &AutomationRule, auto_ctx: &AutomationContext) -> AutomationResult {
    let template = match &rule.command {
        Some(cmd) => cmd,
        None => {
            return AutomationResult::Failure("Shell action requires a 'command' field".to_string())
        }
    };

    let expanded = expand_template(template, &auto_ctx.vars);

    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&expanded)
        .status()
    {
        Ok(status) if status.success() => AutomationResult::Success(format!("shell: {}", expanded)),
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
    populate_entity_vars(ctx, &event.event_type, entity_id, &mut auto_ctx);

    for rule in matching_rules(&config.automation, &event.event_type) {
        let result = match rule.action {
            AutomationAction::Shell => execute_shell(rule, &auto_ctx),
            _ => execute_builtin(ctx, rule, entity_id),
        };

        match result {
            AutomationResult::Success(msg) => println!("  (auto: {})", msg),
            AutomationResult::Failure(msg) => {
                eprintln!("  Warning: automation '{:?}' failed: {}", rule.action, msg)
            }
            AutomationResult::Skipped(_) => {}
        }
    }
}

/// Populate template variables from the entity that triggered the event.
fn populate_entity_vars(
    ctx: &CommandContext,
    event_type: &EventType,
    entity_id: &str,
    auto_ctx: &mut AutomationContext,
) {
    let event_str = event_type.to_string();
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

    match rule.action {
        AutomationAction::GithubIssue => {
            let mut problem = match ctx.store.load_problem(entity_id) {
                Ok(p) => p,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_create_issue(ctx, &mut problem) {
                Ok(()) => AutomationResult::Success("created GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubClose => {
            let problem = match ctx.store.load_problem(entity_id) {
                Ok(p) => p,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_close_issue(ctx, &problem) {
                Ok(()) => AutomationResult::Success("closed GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubPr => {
            let mut solution = match ctx.store.load_solution(entity_id) {
                Ok(s) => s,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_create_or_update_pr(ctx, &mut solution) {
                Ok(()) => AutomationResult::Success("created/updated GitHub PR".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubMerge => {
            let solution = match ctx.store.load_solution(entity_id) {
                Ok(s) => s,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_merge_pr(ctx, &solution) {
                Ok(()) => AutomationResult::Success("merged GitHub PR".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubSync => {
            AutomationResult::Skipped("github_sync not yet implemented as automation".to_string())
        }
        AutomationAction::Shell => {
            // Handled before dispatch; should never reach here
            execute_shell(rule, &AutomationContext::new(""))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AutomationAction, AutomationRule, EventType};

    fn execute_rule(rule: &AutomationRule, auto_ctx: &AutomationContext) -> AutomationResult {
        if rule.action == AutomationAction::Shell {
            return execute_shell(rule, auto_ctx);
        }
        if rule.action != AutomationAction::Shell {
            return AutomationResult::Skipped(format!(
                "{:?} requires CommandContext (use run() instead)",
                rule.action
            ));
        }
        unreachable!()
    }

    fn rule(on: EventType, action: AutomationAction) -> AutomationRule {
        AutomationRule {
            on,
            action,
            command: None,
            enabled: true,
        }
    }

    // ── matching_rules ──

    #[test]
    fn test_matching_rules_filters_by_event() {
        let rules = vec![
            rule(EventType::SolutionSubmitted, AutomationAction::GithubPr),
            rule(EventType::ProblemSolved, AutomationAction::GithubClose),
            rule(EventType::SolutionSubmitted, AutomationAction::Shell),
        ];
        let matched = matching_rules(&rules, &EventType::SolutionSubmitted);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].action, AutomationAction::GithubPr);
        assert_eq!(matched[1].action, AutomationAction::Shell);
    }

    #[test]
    fn test_matching_rules_skips_disabled() {
        let mut r = rule(EventType::SolutionSubmitted, AutomationAction::GithubPr);
        r.enabled = false;
        let rules = vec![r];
        let matched = matching_rules(&rules, &EventType::SolutionSubmitted);
        assert!(matched.is_empty());
    }

    #[test]
    fn test_matching_rules_no_match() {
        let rules = vec![rule(
            EventType::ProblemSolved,
            AutomationAction::GithubClose,
        )];
        let matched = matching_rules(&rules, &EventType::SolutionSubmitted);
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
    fn test_execute_rule_shell_missing_command_returns_failure() {
        let r = AutomationRule {
            on: EventType::ProblemCreated,
            action: AutomationAction::Shell,
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
            on: EventType::ProblemCreated,
            action: AutomationAction::Shell,
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
            on: EventType::ProblemCreated,
            action: AutomationAction::Shell,
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
        let r = rule(EventType::ProblemCreated, AutomationAction::GithubIssue);
        let auto_ctx = AutomationContext::new("problem_created");
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Skipped(_)));
    }

    // ── has_explicit_rule ──

    #[test]
    fn test_has_explicit_rule_for_event() {
        let rules = vec![
            rule(EventType::SolutionSubmitted, AutomationAction::GithubPr),
            rule(EventType::ProblemSolved, AutomationAction::GithubClose),
        ];
        assert!(has_explicit_rule(&rules, &EventType::SolutionSubmitted));
        assert!(has_explicit_rule(&rules, &EventType::ProblemSolved));
        assert!(!has_explicit_rule(&rules, &EventType::ProblemCreated));
    }
}
