//! Inspect and relocate automation rules.
//!
//! Automation rules are executable — `action = "shell"` runs `sh -c <command>`
//! — so they live in a machine-local `automation.toml` that push never copies
//! and fetch never writes. Before 0.5.1 they lived in `config.toml`, which
//! syncs through the shared `jjj` bookmark; a rule there gave every collaborator
//! code execution on every clone. Rules found in `config.toml` are now ignored,
//! and `migrate` moves them to the local file after showing exactly what will
//! start running.

use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{AutomationConfig, AutomationRule};
use crate::storage::AUTOMATION_FILE;

/// Show the active rules and where they came from.
pub fn list(ctx: &CommandContext, json: bool) -> Result<()> {
    let active = ctx.store.load_config()?.automation;
    let legacy = ctx.store.legacy_config_automation()?;
    let has_local_file = ctx.store.load_automation_config()?.is_some();

    if json {
        let payload = serde_json::json!({
            "source": if has_local_file { AUTOMATION_FILE } else { "global config.toml" },
            "active": active,
            "ignored_in_config_toml": legacy,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if active.is_empty() {
        println!("No automation rules are active.");
    } else {
        let source = if has_local_file {
            AUTOMATION_FILE.to_string()
        } else {
            "~/.config/jjj/config.toml".to_string()
        };
        println!("Active rules ({}) — from {}:", active.len(), source);
        for rule in &active {
            println!("{}", describe(rule));
        }
    }

    if !legacy.is_empty() {
        println!();
        println!(
            "Ignored ({}) — found in the synced config.toml:",
            legacy.len()
        );
        for rule in &legacy {
            println!("{}", describe(rule));
        }
        println!();
        println!("These do not run. config.toml is shared through the jjj bookmark,");
        println!("so a rule there would execute whatever a collaborator pushed.");
        println!("Move them to this machine with:  jjj automation migrate");
    }

    Ok(())
}

/// Move rules out of `config.toml` into the machine-local `automation.toml`.
pub fn migrate(ctx: &CommandContext, force: bool) -> Result<()> {
    let legacy = ctx.store.legacy_config_automation()?;

    if legacy.is_empty() {
        println!("Nothing to migrate — config.toml declares no automation rules.");
        return Ok(());
    }

    println!(
        "These {} rule(s) will become active on this machine:",
        legacy.len()
    );
    for rule in &legacy {
        println!("{}", describe(rule));
    }
    println!();

    if !force {
        println!("Review them first — a rule that arrived from a remote runs with your");
        println!("privileges. Re-run with --force to migrate.");
        return Ok(());
    }

    let mut local = ctx.store.load_automation_config()?.unwrap_or_default();
    local.automation.extend(legacy);
    ctx.store.save_automation_config(&local)?;
    ctx.store.strip_config_automation()?;

    println!("Migrated to {}.", AUTOMATION_FILE);
    println!("Removed the automation key from config.toml so it stops syncing.");
    Ok(())
}

/// One-line rendering of a rule, safe to print (the command is shown verbatim
/// so the reader can judge it — that is the entire point of the review step).
fn describe(rule: &AutomationRule) -> String {
    let state = if rule.enabled { "" } else { " [disabled]" };
    match &rule.command {
        Some(cmd) => format!(
            "  on {} → {:?}{}\n      {}",
            rule.on, rule.action, state, cmd
        ),
        None => format!("  on {} → {:?}{}", rule.on, rule.action, state),
    }
}

/// Convenience for tests and `doctor`: the rules that would actually run.
pub fn active_rules(ctx: &CommandContext) -> Result<Vec<AutomationRule>> {
    Ok(ctx.store.load_config()?.automation)
}

/// Convenience for `doctor`: whether a local automation file exists.
pub fn local_config(ctx: &CommandContext) -> Result<Option<AutomationConfig>> {
    ctx.store.load_automation_config()
}
