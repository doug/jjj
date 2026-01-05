use crate::cli::MilestoneAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Milestone, MilestoneStatus};
use crate::storage::MetadataStore;
use chrono::{NaiveDate, Utc};

pub fn execute(action: MilestoneAction) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    match action {
        MilestoneAction::New {
            title,
            date,
            description,
            tag,
        } => create_milestone(&store, title, date, description, tag),
        MilestoneAction::Edit {
            milestone_id,
            title,
            date,
            description,
            status,
            add_tag,
            remove_tag,
        } => edit_milestone(
            &store,
            milestone_id,
            title,
            date,
            description,
            status,
            add_tag,
            remove_tag,
        ),
        MilestoneAction::List { json } => list_milestones(&store, json),
        MilestoneAction::Show {
            milestone_id,
            json,
        } => show_milestone(&store, milestone_id, json),
        MilestoneAction::AddFeature {
            milestone_id,
            feature_id,
        } => add_feature(&store, milestone_id, feature_id),
        MilestoneAction::AddBug {
            milestone_id,
            bug_id,
        } => add_bug(&store, milestone_id, bug_id),
        MilestoneAction::Roadmap { json } => show_roadmap(&store, json),
        MilestoneAction::Assign { milestone_id, to } => assign_milestone(&store, milestone_id, to),
    }
}

fn create_milestone(
    store: &MetadataStore,
    title: String,
    date: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
) -> Result<()> {
    store.with_metadata(&format!("Create milestone {}", title), || {
        let mut config = store.load_config()?;
        let milestone_id = store.next_milestone_id()?;

        // Parse date if provided
        let target_date = if let Some(date_str) = date {
            let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date format: {}. Use YYYY-MM-DD", date_str))?;
            Some(naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc())
        } else {
            None
        };

        let mut milestone = Milestone {
            id: milestone_id.clone(),
            title: title.clone(),
            description,
            target_date,
            status: MilestoneStatus::Planning,
            feature_ids: Vec::new(),
            bug_ids: Vec::new(),
            tag_ids: std::collections::HashSet::new(),
            version: None,
            assignee: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Resolve and add tags
        for tag_input in tags {
            let tag_id = crate::utils::resolve_tag(&mut config, &tag_input);
            milestone.add_tag(tag_id);
        }

        store.save_config(&config)?;
        store.save_milestone(&milestone)?;
        println!("Created milestone {} ({})", milestone_id, title);
        Ok(())
    })
}

fn edit_milestone(
    store: &MetadataStore,
    milestone_id: String,
    title: Option<String>,
    date: Option<String>,
    description: Option<String>,
    status: Option<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
) -> Result<()> {
    store.with_metadata(&format!("Edit milestone {}", milestone_id), || {
        let mut milestone = store.load_milestone(&milestone_id)?;
        let mut config = store.load_config()?;

        if let Some(t) = title {
            milestone.title = t;
        }

        if let Some(d) = date {
            let naive_date = NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date format: {}. Use YYYY-MM-DD", d))?;
            milestone.target_date = Some(naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc());
        }

        if let Some(desc) = description {
            milestone.description = Some(desc);
        }

        if let Some(s) = status {
            milestone.status = match s.to_lowercase().as_str() {
                "planning" => MilestoneStatus::Planning,
                "active" => MilestoneStatus::Active,
                "released" => MilestoneStatus::Released,
                "cancelled" => MilestoneStatus::Cancelled,
                _ => return Err(format!("Invalid status: {}. Use planning, active, released, or cancelled", s).into()),
            };
        }

        for tag_input in add_tags {
            let tag_id = crate::utils::resolve_tag(&mut config, &tag_input);
            milestone.add_tag(tag_id);
        }

        for tag_input in remove_tags {
            let tag_id = if let Some(tag) = config.get_tag(&tag_input) {
                Some(tag.id.clone())
            } else if let Some(tag) = config.get_tag_by_name(&tag_input) {
                Some(tag.id.clone())
            } else {
                Some(tag_input)
            };
            
            if let Some(id) = tag_id {
                milestone.remove_tag(&id);
            }
        }

        store.save_config(&config)?;
        store.save_milestone(&milestone)?;
        println!("Updated milestone {}", milestone_id);
        Ok(())
    })
}

fn list_milestones(store: &MetadataStore, json: bool) -> Result<()> {
    let milestones = store.list_milestones()?;
    let config = store.load_config()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&milestones)?);
        return Ok(());
    }

    if milestones.is_empty() {
        println!("No milestones found.");
        return Ok(());
    }

    println!("Milestones:");
    for milestone in milestones {
        let date_str = if let Some(date) = milestone.target_date {
            format!(" ({})", date.format("%Y-%m-%d"))
        } else {
            String::new()
        };

        let status_str = format!("{:?}", milestone.status);

        let tag_names: Vec<String> = milestone
            .tag_ids
            .iter()
            .map(|id| {
                config
                    .get_tag(id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
        
        let tags_str = if tag_names.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tag_names.join(", "))
        };

        println!(
            "  {} - {} [{}]{}{} ({} features, {} bugs)",
            milestone.id,
            milestone.title,
            status_str,
            date_str,
            tags_str,
            milestone.feature_ids.len(),
            milestone.bug_ids.len()
        );
    }

    Ok(())
}

fn show_milestone(store: &MetadataStore, milestone_id: String, json: bool) -> Result<()> {
    let milestone = store.load_milestone(&milestone_id)?;
    let config = store.load_config()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&milestone)?);
        return Ok(());
    }

    println!("Milestone: {} - {}", milestone.id, milestone.title);
    println!("Status: {:?}", milestone.status);

    if let Some(desc) = &milestone.description {
        println!("Description: {}", desc);
    }

    if let Some(date) = milestone.target_date {
        println!("Target Date: {}", date.format("%Y-%m-%d"));
        if milestone.status != MilestoneStatus::Released
            && milestone.status != MilestoneStatus::Cancelled
        {
            if let Some(days) = milestone.days_until_target() {
                if days < 0 {
                    println!("  ⚠️  {} days overdue", -days);
                } else {
                    println!("  {} days remaining", days);
                }
            }
        }
    }

    if !milestone.tag_ids.is_empty() {
        let tag_names: Vec<String> = milestone
            .tag_ids
            .iter()
            .map(|id| {
                config
                    .get_tag(id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
        println!("Tags: {}", tag_names.join(", "));
    }

    println!("\nFeatures ({}):", milestone.feature_ids.len());
    for feature_id in &milestone.feature_ids {
        println!("  - {}", feature_id);
    }

    println!("\nBugs ({}):", milestone.bug_ids.len());
    for bug_id in &milestone.bug_ids {
        println!("  - {}", bug_id);
    }

    println!("\nCreated: {}", milestone.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", milestone.updated_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

fn add_feature(store: &MetadataStore, milestone_id: String, feature_id: String) -> Result<()> {
    store.with_metadata(
        &format!("Add feature {} to milestone {}", feature_id, milestone_id),
        || {
            let mut milestone = store.load_milestone(&milestone_id)?;
            let mut feature = store.load_feature(&feature_id)?;

            // Add feature to milestone
            if !milestone.feature_ids.contains(&feature_id) {
                milestone.feature_ids.push(feature_id.clone());
                milestone.updated_at = Utc::now();
                store.save_milestone(&milestone)?;
            }

            // Update feature's milestone
            feature.milestone_id = Some(milestone_id.clone());
            feature.updated_at = Utc::now();
            store.save_feature(&feature)?;

            Ok(())
        },
    )?;

    println!("Added feature {} to milestone {}", feature_id, milestone_id);
    Ok(())
}

fn add_bug(store: &MetadataStore, milestone_id: String, bug_id: String) -> Result<()> {
    store.with_metadata(
        &format!("Add bug {} to milestone {}", bug_id, milestone_id),
        || {
            let mut milestone = store.load_milestone(&milestone_id)?;
            let mut bug = store.load_bug(&bug_id)?;

            // Add bug to milestone
            if !milestone.bug_ids.contains(&bug_id) {
                milestone.bug_ids.push(bug_id.clone());
                milestone.updated_at = Utc::now();
                store.save_milestone(&milestone)?;
            }

            // Update bug's milestone
            bug.milestone_id = Some(milestone_id.clone());
            bug.updated_at = Utc::now();
            store.save_bug(&bug)?;

            Ok(())
        },
    )?;

    println!("Added bug {} to milestone {}", bug_id, milestone_id);
    Ok(())
}

fn show_roadmap(store: &MetadataStore, json: bool) -> Result<()> {
    let mut milestones = store.list_milestones()?;

    // Sort by target date (None dates go last)
    milestones.sort_by(|a, b| match (&a.target_date, &b.target_date) {
        (Some(date_a), Some(date_b)) => date_a.cmp(date_b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&milestones)?);
        return Ok(());
    }

    if milestones.is_empty() {
        println!("No milestones found.");
        return Ok(());
    }

    println!("Roadmap:\n");

    for milestone in milestones {
        let date_str = if let Some(date) = milestone.target_date {
            format!("{}", date.format("%Y-%m-%d"))
        } else {
            "No date".to_string()
        };

        let status_icon = match milestone.status {
            MilestoneStatus::Planning => "📋",
            MilestoneStatus::Active => "🚀",
            MilestoneStatus::Released => "✅",
            MilestoneStatus::Cancelled => "❌",
        };

        println!(
            "{} {} - {} [{}]",
            status_icon,
            milestone.id,
            milestone.title,
            date_str
        );

        if let Some(desc) = &milestone.description {
            println!("   {}", desc);
        }

        // Show overdue warning
        if milestone.is_overdue() {
            if let Some(days) = milestone.days_until_target() {
                println!("   ⚠️  {} days overdue", -days);
            }
        }

        println!(
            "   {} features, {} bugs",
            milestone.feature_ids.len(),
            milestone.bug_ids.len()
        );
        println!();
    }

    Ok(())
}

fn assign_milestone(store: &MetadataStore, milestone_id: String, assignee: Option<String>) -> Result<()> {
    // Determine assignee
    let assignee_name = if let Some(name) = assignee {
        name
    } else {
        // Default to current user
        store.jj_client.user_name()?
    };
    
    let assignee_clone = assignee_name.clone();

    store.with_metadata(&format!("Assign milestone {} to {}", milestone_id, assignee_name), || {
        let mut milestone = store.load_milestone(&milestone_id)?;
        milestone.assignee = Some(assignee_name.clone());
        milestone.updated_at = Utc::now();
        store.save_milestone(&milestone)?;
        Ok(())
    })?;

    println!("Assigned milestone {} to {}", milestone_id, assignee_clone);
    Ok(())
}
