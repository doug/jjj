//! Config-driven automation: fires actions in response to jjj events.
//!
//! Rules are defined in `config.toml` under `[[automation]]`.
//! Each rule matches an `EventType` and dispatches to a built-in
//! action handler or shell command. Failures print warnings but never
//! block the primary operation.

use std::collections::HashMap;

use crate::models::{AutomationAction, AutomationRule, Event, EventType};
use crate::storage::MetadataStore;

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

/// Convert a template variable key (e.g. `problem.title`) into a shell-safe
/// environment variable name (e.g. `JJJ_VAR_PROBLEM_TITLE`).
fn env_var_name(key: &str) -> String {
    let mut name = String::from("JJJ_VAR_");
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_uppercase());
        } else {
            name.push('_');
        }
    }
    name
}

/// Expand `{{var}}` placeholders into references to environment variables that
/// carry the raw values, returning the rewritten command plus the
/// `(name, value)` pairs the caller must export before running `sh -c`.
///
/// Untrusted values (entity titles, bodies, etc. fetched from the shared
/// bookmark) are **never** interpolated into the command text — they travel
/// via the environment instead, so a value like `$(rm -rf /)` or `'; rm -rf /`
/// is inert no matter how the template quotes the placeholder. Each
/// placeholder expands to a double-quoted reference (`"${JJJ_VAR_X}"`) so the
/// value is also safe from word-splitting and globbing. Unknown variables are
/// left as-is.
fn expand_template(
    template: &str,
    vars: &HashMap<String, String>,
) -> (String, Vec<(String, String)>) {
    let mut result = template.to_string();
    let mut env: Vec<(String, String)> = Vec::new();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        if result.contains(&placeholder) {
            let name = env_var_name(key);
            result = result.replace(&placeholder, &format!("\"${{{}}}\"", name));
            env.push((name, value.clone()));
        }
    }
    (result, env)
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

    let (expanded, env) = expand_template(template, &auto_ctx.vars);

    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&expanded)
        .envs(env)
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
/// # Semantics
///
/// Automation has **at-most-once** semantics. The flow is:
///
/// 1. Caller wraps the metadata write in [`MetadataStore::with_metadata`].
///    The entity save and any pending events are flushed inside this block.
/// 2. After `with_metadata` returns `Ok`, the caller invokes [`run`].
/// 3. `run` enumerates matching rules and dispatches each.
///
/// If the process is killed between step 1 and 3, the event is durable but
/// the action does not fire. Recovery is manual: inspect the failure sidecar
/// (`.jj/jjj-meta/automation-failures.jsonl`) and re-trigger the action
/// (e.g., `jjj github push`).
///
/// Failures are appended to the sidecar and printed as warnings; they never
/// propagate to the caller.
pub fn run(store: &MetadataStore, event: &Event, entity_id: &str) {
    let config = match store.load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: automation disabled (config error: {})", e);
            return;
        }
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
    populate_entity_vars(store, &event.event_type, entity_id, &mut auto_ctx);

    for rule in matching_rules(&config.automation, &event.event_type) {
        let result = match rule.action {
            AutomationAction::Shell => execute_shell(rule, &auto_ctx),
            _ => execute_builtin(store, rule, entity_id),
        };

        match result {
            AutomationResult::Success(msg) => println!("  (auto: {})", msg),
            AutomationResult::Failure(msg) => {
                eprintln!("  Warning: automation '{:?}' failed: {}", rule.action, msg);
                record_failure(store, event, &rule.action, &msg);
            }
            AutomationResult::Skipped(_) => {}
        }
    }
}

/// Append a failed automation attempt to the sidecar log for manual replay.
///
/// The sidecar lives at `.jj/jjj-meta/automation-failures.jsonl`. Each line is
/// a JSON object: `{ "when", "event_type", "entity_id", "action", "message" }`.
/// Failures here are silently dropped — the caller already warned.
fn record_failure(store: &MetadataStore, event: &Event, action: &AutomationAction, message: &str) {
    use std::io::Write;

    let path = store.meta_path().join("automation-failures.jsonl");
    let record = serde_json::json!({
        "when": event.when.to_rfc3339(),
        "event_type": event.event_type.to_string(),
        "entity_id": event.entity,
        "action": format!("{:?}", action),
        "message": message,
    });

    if let Ok(line) = serde_json::to_string(&record) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{}", line);
        }
    }
}

/// Populate template variables from the entity that triggered the event.
fn populate_entity_vars(
    store: &MetadataStore,
    event_type: &EventType,
    entity_id: &str,
    auto_ctx: &mut AutomationContext,
) {
    match event_type {
        EventType::ProblemCreated
        | EventType::ProblemSolved
        | EventType::ProblemDissolved
        | EventType::ProblemReopened => {
            if let Ok(problem) = store.load_problem(entity_id) {
                auto_ctx.set("title", &problem.title);
                auto_ctx.set("type", "problem");
                if let Some(n) = problem.github_issue {
                    auto_ctx.set("issue_number", &n.to_string());
                }
            }
        }
        EventType::SolutionCreated
        | EventType::SolutionSubmitted
        | EventType::SolutionApproved
        | EventType::SolutionWithdrawn => {
            if let Ok(solution) = store.load_solution(entity_id) {
                auto_ctx.set("title", &solution.title);
                auto_ctx.set("type", "solution");
                if let Some(n) = solution.github_pr {
                    auto_ctx.set("pr_number", &n.to_string());
                }
                if let Ok(problem) = store.load_problem(&solution.problem_id) {
                    auto_ctx.set("problem.title", &problem.title);
                    if let Some(n) = problem.github_issue {
                        auto_ctx.set("issue_number", &n.to_string());
                    }
                }
            }
        }
        EventType::CritiqueRaised
        | EventType::CritiqueAddressed
        | EventType::CritiqueDismissed
        | EventType::CritiqueValidated
        | EventType::CritiqueReplied => {
            if let Ok(critique) = store.load_critique(entity_id) {
                auto_ctx.set("title", &critique.title);
                auto_ctx.set("type", "critique");
                if let Ok(solution) = store.load_solution(&critique.solution_id) {
                    auto_ctx.set("solution.title", &solution.title);
                    if let Some(n) = solution.github_pr {
                        auto_ctx.set("pr_number", &n.to_string());
                    }
                    if let Ok(problem) = store.load_problem(&solution.problem_id) {
                        auto_ctx.set("problem.title", &problem.title);
                    }
                }
            }
        }
        // Milestone and GitHub sync events: no entity-specific vars yet.
        // Listed explicitly so adding a new EventType variant produces a compile error.
        EventType::MilestoneCreated
        | EventType::MilestoneCompleted
        | EventType::GithubIssueCreated
        | EventType::GithubIssueImported
        | EventType::GithubIssueClosed
        | EventType::GithubPrCreated
        | EventType::GithubPrMerged
        | EventType::GithubReviewImported => {}
    }
}

/// Execute a built-in GitHub action.
fn execute_builtin(
    store: &MetadataStore,
    rule: &AutomationRule,
    entity_id: &str,
) -> AutomationResult {
    use crate::sync::hooks;

    match rule.action {
        AutomationAction::GithubIssue => {
            let mut problem = match store.load_problem(entity_id) {
                Ok(p) => p,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_create_issue(store, &mut problem) {
                Ok(()) => AutomationResult::Success("created GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubClose => {
            let problem = match store.load_problem(entity_id) {
                Ok(p) => p,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_close_issue(store, &problem) {
                Ok(()) => AutomationResult::Success("closed GitHub issue".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubPr => {
            let mut solution = match store.load_solution(entity_id) {
                Ok(s) => s,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_create_or_update_pr(store, &mut solution) {
                Ok(()) => AutomationResult::Success("created/updated GitHub PR".to_string()),
                Err(e) => AutomationResult::Failure(e.to_string()),
            }
        }
        AutomationAction::GithubMerge => {
            let solution = match store.load_solution(entity_id) {
                Ok(s) => s,
                Err(e) => return AutomationResult::Failure(e.to_string()),
            };
            match hooks::do_merge_pr(store, &solution) {
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
        let (result, env) = expand_template("New: {{title}} ({{id}})", &vars);
        // Placeholders become double-quoted env references; values go in env.
        assert_eq!(result, "New: \"${JJJ_VAR_TITLE}\" (\"${JJJ_VAR_ID}\")");
        assert!(env.contains(&("JJJ_VAR_TITLE".to_string(), "Fix auth bug".to_string())));
        assert!(env.contains(&("JJJ_VAR_ID".to_string(), "abc123".to_string())));
    }

    #[test]
    fn test_expand_template_unknown_var_kept() {
        let vars = HashMap::new();
        let (result, env) = expand_template("Hello {{unknown}}", &vars);
        assert_eq!(result, "Hello {{unknown}}");
        assert!(env.is_empty());
    }

    #[test]
    fn test_expand_template_no_vars() {
        let vars = HashMap::new();
        let (result, env) = expand_template("plain text", &vars);
        assert_eq!(result, "plain text");
        assert!(env.is_empty());
    }

    #[test]
    fn test_expand_template_dotted_vars() {
        let mut vars = HashMap::new();
        vars.insert("problem.title".to_string(), "Auth bug".to_string());
        let (result, env) = expand_template("On: {{problem.title}}", &vars);
        assert_eq!(result, "On: \"${JJJ_VAR_PROBLEM_TITLE}\"");
        assert_eq!(
            env,
            vec![("JJJ_VAR_PROBLEM_TITLE".to_string(), "Auth bug".to_string())]
        );
    }

    #[test]
    fn test_shell_injection_via_command_substitution_is_inert() {
        // A malicious title with a command substitution must NOT execute when
        // an automation rule expands it into a shell action.
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("pwned");
        let r = AutomationRule {
            on: EventType::ProblemCreated,
            action: AutomationAction::Shell,
            command: Some("echo {{title}}".to_string()),
            enabled: true,
        };
        let mut auto_ctx = AutomationContext::new("problem_created");
        auto_ctx.set("title", &format!("$(touch {})", marker.display()));
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Success(_)));
        assert!(
            !marker.exists(),
            "command substitution in an entity title must not execute"
        );
    }

    #[test]
    fn test_shell_injection_with_legacy_quoted_template_is_inert() {
        // Even the previously-documented `'{{title}}'` quoting (which used to
        // collapse the escaping) must now be inert — the value lives in the
        // environment, never in the command text.
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("pwned");
        let r = AutomationRule {
            on: EventType::ProblemCreated,
            action: AutomationAction::Shell,
            command: Some("echo '{{title}}'".to_string()),
            enabled: true,
        };
        let mut auto_ctx = AutomationContext::new("problem_created");
        auto_ctx.set("title", &format!("'; touch {}; '", marker.display()));
        let result = execute_rule(&r, &auto_ctx);
        assert!(matches!(result, AutomationResult::Success(_)));
        assert!(
            !marker.exists(),
            "shell metacharacters in an entity title must not execute"
        );
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
