use crate::cli::BugAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Bug, BugStatus, Severity};
use crate::storage::MetadataStore;
use chrono::Utc;

pub fn execute(action: BugAction) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    match action {
        BugAction::New {
            title,
            severity,
            description,
            repro,
        } => create_bug(&store, title, severity, description, repro),
        BugAction::List {
            severity,
            status,
            open,
            json,
        } => list_bugs(&store, severity, status, open, json),
        BugAction::Show { bug_id, json } => show_bug(&store, bug_id, json),
        BugAction::Link {
            bug_id,
            feature,
            milestone,
        } => link_bug(&store, bug_id, feature, milestone),
        BugAction::Status { bug_id, status } => update_status(&store, bug_id, status),
        BugAction::Triage { json } => show_triage(&store, json),
    }
}

fn create_bug(
    store: &MetadataStore,
    title: String,
    severity: Option<String>,
    description: Option<String>,
    repro_steps: Option<String>,
) -> Result<()> {
    let bug_id = store.next_bug_id()?;

    // Parse severity
    let severity = if let Some(s) = severity {
        parse_severity(&s)?
    } else {
        Severity::Medium
    };
    let severity_clone = severity.clone();

    let bug = Bug {
        id: bug_id.clone(),
        title: title.clone(),
        description,
        severity,
        status: BugStatus::New,
        feature_id: None,
        milestone_id: None,
        assignee: None,
        reporter: None,
        change_ids: Vec::new(),
        tags: std::collections::HashSet::new(),
        affected_version: None,
        fixed_version: None,
        repro_steps,
        expected_behavior: None,
        actual_behavior: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    store.with_metadata(&format!("Create bug {}", bug_id), || {
        store.save_bug(&bug)?;
        Ok(())
    })?;

    println!("Created bug {} ({}) [Severity: {:?}]", bug_id, title, severity_clone);
    Ok(())
}

fn list_bugs(
    store: &MetadataStore,
    severity_filter: Option<String>,
    status_filter: Option<String>,
    open_only: bool,
    json: bool,
) -> Result<()> {
    let mut bugs = store.list_bugs()?;

    // Apply filters
    if let Some(severity) = &severity_filter {
        let severity = parse_severity(severity)?;
        bugs.retain(|b| b.severity == severity);
    }

    if let Some(status) = &status_filter {
        let status = parse_bug_status(status)?;
        bugs.retain(|b| b.status == status);
    }

    if open_only {
        bugs.retain(|b| b.is_open());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&bugs)?);
        return Ok(());
    }

    if bugs.is_empty() {
        println!("No bugs found.");
        return Ok(());
    }

    println!("Bugs:");
    for bug in bugs {
        let severity_str = format!("{:?}", bug.severity);
        let status_str = format!("{:?}", bug.status);

        let links = {
            let mut parts = Vec::new();
            if let Some(f) = &bug.feature_id {
                parts.push(format!("F:{}", f));
            }
            if let Some(m) = &bug.milestone_id {
                parts.push(format!("M:{}", m));
            }
            if parts.is_empty() {
                String::new()
            } else {
                format!(" ({})", parts.join(", "))
            }
        };

        println!(
            "  {} - {} [{}] [{}]{}",
            bug.id, bug.title, severity_str, status_str, links
        );
    }

    Ok(())
}

fn show_bug(store: &MetadataStore, bug_id: String, json: bool) -> Result<()> {
    let bug = store.load_bug(&bug_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&bug)?);
        return Ok(());
    }

    println!("Bug: {} - {}", bug.id, bug.title);
    println!("Status: {:?}", bug.status);
    println!("Severity: {:?}", bug.severity);

    if let Some(desc) = &bug.description {
        println!("Description: {}", desc);
    }

    if let Some(feature) = &bug.feature_id {
        println!("Feature: {}", feature);
    }

    if let Some(milestone) = &bug.milestone_id {
        println!("Milestone: {}", milestone);
    }

    if let Some(assignee) = &bug.assignee {
        println!("Assignee: {}", assignee);
    }

    if let Some(reporter) = &bug.reporter {
        println!("Reporter: {}", reporter);
    }

    if let Some(repro) = &bug.repro_steps {
        println!("\nReproduction Steps:");
        println!("{}", repro);
    }

    if let Some(expected) = &bug.expected_behavior {
        println!("\nExpected Behavior:");
        println!("{}", expected);
    }

    if let Some(actual) = &bug.actual_behavior {
        println!("\nActual Behavior:");
        println!("{}", actual);
    }

    if let Some(version) = &bug.affected_version {
        println!("\nAffected Version: {}", version);
    }

    if let Some(version) = &bug.fixed_version {
        println!("Fixed Version: {}", version);
    }

    println!("\nCreated: {}", bug.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", bug.updated_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

fn link_bug(
    store: &MetadataStore,
    bug_id: String,
    feature: Option<String>,
    milestone: Option<String>,
) -> Result<()> {
    if feature.is_none() && milestone.is_none() {
        return Err("Must specify either --feature or --milestone".into());
    }

    store.with_metadata(&format!("Link bug {}", bug_id), || {
        let mut bug = store.load_bug(&bug_id)?;

        // Link to feature
        if let Some(feature_id) = &feature {
            let mut feature = store.load_feature(feature_id)?;

            // Update bug
            bug.feature_id = Some(feature_id.clone());

            // Update feature
            if !feature.bug_ids.contains(&bug_id) {
                feature.bug_ids.push(bug_id.clone());
                feature.updated_at = Utc::now();
                store.save_feature(&feature)?;
            }
        }

        // Link to milestone
        if let Some(milestone_id) = &milestone {
            let mut milestone = store.load_milestone(milestone_id)?;

            // Update bug
            bug.milestone_id = Some(milestone_id.clone());

            // Update milestone
            if !milestone.bug_ids.contains(&bug_id) {
                milestone.bug_ids.push(bug_id.clone());
                milestone.updated_at = Utc::now();
                store.save_milestone(&milestone)?;
            }
        }

        bug.updated_at = Utc::now();
        store.save_bug(&bug)?;

        Ok(())
    })?;

    let mut links = Vec::new();
    if let Some(f) = feature {
        links.push(format!("feature {}", f));
    }
    if let Some(m) = milestone {
        links.push(format!("milestone {}", m));
    }

    println!("Linked bug {} to {}", bug_id, links.join(" and "));
    Ok(())
}

fn update_status(store: &MetadataStore, bug_id: String, status: String) -> Result<()> {
    let new_status = parse_bug_status(&status)?;
    let new_status_clone = new_status.clone();

    store.with_metadata(&format!("Update bug {} status to {:?}", bug_id, new_status), || {
        let mut bug = store.load_bug(&bug_id)?;
        bug.status = new_status;
        bug.updated_at = Utc::now();

        // If marking as fixed, could set fixed_version
        // For now, just update status

        store.save_bug(&bug)?;
        Ok(())
    })?;

    println!("Updated bug {} status to {:?}", bug_id, new_status_clone);
    Ok(())
}

fn show_triage(store: &MetadataStore, json: bool) -> Result<()> {
    let bugs = store.list_bugs()?;

    // Group bugs by severity and status
    let critical: Vec<_> = bugs.iter().filter(|b| b.severity == Severity::Critical && b.is_open()).collect();
    let high: Vec<_> = bugs.iter().filter(|b| b.severity == Severity::High && b.is_open()).collect();
    let medium: Vec<_> = bugs.iter().filter(|b| b.severity == Severity::Medium && b.is_open()).collect();
    let low: Vec<_> = bugs.iter().filter(|b| b.severity == Severity::Low && b.is_open()).collect();

    let new_bugs: Vec<_> = bugs.iter().filter(|b| b.status == BugStatus::New).collect();
    let unassigned: Vec<_> = bugs.iter().filter(|b| b.is_open() && b.assignee.is_none()).collect();

    if json {
        let triage_data = serde_json::json!({
            "by_severity": {
                "critical": critical,
                "high": high,
                "medium": medium,
                "low": low,
            },
            "needs_attention": {
                "new": new_bugs,
                "unassigned": unassigned,
            }
        });
        println!("{}", serde_json::to_string_pretty(&triage_data)?);
        return Ok(());
    }

    println!("Bug Triage View\n");

    println!("🔴 Critical ({}):", critical.len());
    for bug in &critical {
        println!("  {} - {} [{:?}]", bug.id, bug.title, bug.status);
    }

    println!("\n🟠 High ({}):", high.len());
    for bug in &high {
        println!("  {} - {} [{:?}]", bug.id, bug.title, bug.status);
    }

    println!("\n🟡 Medium ({}):", medium.len());
    for bug in &medium {
        println!("  {} - {} [{:?}]", bug.id, bug.title, bug.status);
    }

    println!("\n🟢 Low ({}):", low.len());
    for bug in &low {
        println!("  {} - {} [{:?}]", bug.id, bug.title, bug.status);
    }

    println!("\n📋 Needs Attention:");
    println!("  New bugs: {}", new_bugs.len());
    println!("  Unassigned: {}", unassigned.len());

    let total_open = critical.len() + high.len() + medium.len() + low.len();
    let total_bugs = bugs.len();
    let resolved = total_bugs - total_open;

    println!("\n📊 Summary:");
    println!("  Total bugs: {}", total_bugs);
    println!("  Open: {}", total_open);
    println!("  Resolved: {}", resolved);

    Ok(())
}

// Helper functions

fn parse_severity(s: &str) -> Result<Severity> {
    match s.to_lowercase().as_str() {
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(format!("Invalid severity: {}. Use low, medium, high, or critical", s).into()),
    }
}

fn parse_bug_status(s: &str) -> Result<BugStatus> {
    match s.to_lowercase().as_str() {
        "new" => Ok(BugStatus::New),
        "confirmed" => Ok(BugStatus::Confirmed),
        "inprogress" | "in-progress" | "in_progress" => Ok(BugStatus::InProgress),
        "fixed" => Ok(BugStatus::Fixed),
        "closed" => Ok(BugStatus::Closed),
        "wontfix" | "wont-fix" | "wont_fix" => Ok(BugStatus::WontFix),
        "duplicate" => Ok(BugStatus::Duplicate),
        _ => Err(format!(
            "Invalid status: {}. Use new, confirmed, inprogress, fixed, closed, wontfix, or duplicate",
            s
        )
        .into()),
    }
}
