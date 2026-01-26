use crate::cli::CritiqueAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Critique, CritiqueSeverity, CritiqueStatus, SolutionStatus};
use crate::storage::MetadataStore;

pub fn execute(action: CritiqueAction) -> Result<()> {
    match action {
        CritiqueAction::New {
            solution_id,
            title,
            severity,
            file,
            line,
        } => new_critique(solution_id, title, severity, file, line),
        CritiqueAction::List {
            solution,
            status,
            json,
        } => list_critiques(solution, status, json),
        CritiqueAction::Show { critique_id, json } => show_critique(critique_id, json),
        CritiqueAction::Edit {
            critique_id,
            title,
            severity,
            status,
        } => edit_critique(critique_id, title, severity, status),
        CritiqueAction::Address { critique_id } => address_critique(critique_id),
        CritiqueAction::Validate { critique_id } => validate_critique(critique_id),
        CritiqueAction::Dismiss { critique_id } => dismiss_critique(critique_id),
        CritiqueAction::Reply { critique_id, body } => reply_to_critique(critique_id, body),
    }
}

fn new_critique(
    solution_id: String,
    title: String,
    severity_str: String,
    file: Option<String>,
    line: Option<usize>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    // Parse severity
    let severity: CritiqueSeverity = severity_str.parse().map_err(|e: String| e)?;

    // Validate solution exists
    let solution = store.load_solution(&solution_id)?;

    // Warn if solution is already finalized
    if solution.is_finalized() {
        println!(
            "Warning: Solution {} is already {:?}. Creating critique anyway.",
            solution_id, solution.status
        );
    }

    store.with_metadata(&format!("Create critique on {}: {}", solution_id, title), || {
        let critique_id = store.next_critique_id()?;
        let mut critique = Critique::new(critique_id.clone(), title.clone(), solution_id.clone());
        critique.set_severity(severity.clone());

        // Set author to current user
        let author = store.jj_client.user_identity()?;
        critique.author = Some(author);

        // Set location if provided
        if let (Some(file_path), Some(line_num)) = (file.clone(), line) {
            // Try to read context from file
            let context = store.jj_client
                .file_at_revision("@", &file_path)
                .ok()
                .map(|content| {
                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                    let start = line_num.saturating_sub(2);
                    let end = (line_num + 2).min(lines.len());
                    lines[start..end].to_vec()
                })
                .unwrap_or_default();

            critique.set_location(file_path, line_num, None, context);
        }

        store.save_critique(&critique)?;

        // Update solution's critique_ids
        let mut solution = store.load_solution(&solution_id)?;
        solution.add_critique(critique_id.clone());
        store.save_solution(&solution)?;

        println!(
            "Created critique {} ({}) on solution {}",
            critique.id, critique.title, solution_id
        );
        println!("  Severity: {}", severity);

        if let Some(ref fp) = file {
            println!("  Location: {}:{}", fp, line.unwrap_or(0));
        }

        Ok(())
    })
}

fn list_critiques(
    solution_filter: Option<String>,
    status_filter: Option<String>,
    json: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let mut critiques = store.list_critiques()?;

    // Filter by solution
    if let Some(ref sid) = solution_filter {
        critiques.retain(|c| &c.solution_id == sid);
    }

    // Filter by status
    if let Some(status_str) = status_filter {
        let status: CritiqueStatus = status_str.parse().map_err(|e: String| e)?;
        critiques.retain(|c| c.status == status);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&critiques)?);
        return Ok(());
    }

    if critiques.is_empty() {
        println!("No critiques found.");
        return Ok(());
    }

    println!(
        "{:<8} {:<12} {:<10} {:<10} {}",
        "ID", "STATUS", "SEVERITY", "SOLUTION", "TITLE"
    );
    println!("{}", "-".repeat(80));

    for critique in &critiques {
        let status_icon = match critique.status {
            CritiqueStatus::Open => "?",
            CritiqueStatus::Addressed => "+",
            CritiqueStatus::Valid => "!",
            CritiqueStatus::Dismissed => "-",
        };

        println!(
            "{:<8} {}{:<11} {:<10} {:<10} {}",
            critique.id,
            status_icon,
            critique.status,
            critique.severity,
            critique.solution_id,
            critique.title
        );
    }

    Ok(())
}

fn show_critique(critique_id: String, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let critique = store.load_critique(&critique_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&critique)?);
        return Ok(());
    }

    println!("Critique: {} - {}", critique.id, critique.title);
    println!("Status: {}", critique.status);
    println!("Severity: {}", critique.severity);
    println!("Target Solution: {}", critique.solution_id);

    if let Some(ref author) = critique.author {
        println!("Author: {}", author);
    }

    // Show argument
    if !critique.argument.is_empty() {
        println!("\n## Argument\n{}", critique.argument);
    }

    // Show evidence
    if !critique.evidence.is_empty() {
        println!("\n## Evidence\n{}", critique.evidence);
    }

    // Show replies
    if !critique.replies.is_empty() {
        println!("\n## Discussion ({} replies)", critique.replies.len());
        for reply in &critique.replies {
            println!("\n### {} @ {}", reply.author, reply.created_at.format("%Y-%m-%d %H:%M"));
            println!("{}", reply.body);
        }
    }

    println!("\nCreated: {}", critique.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", critique.updated_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

fn edit_critique(
    critique_id: String,
    title: Option<String>,
    severity: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Edit critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;

        if let Some(new_title) = title {
            critique.title = new_title;
        }

        if let Some(severity_str) = severity {
            let new_severity: CritiqueSeverity = severity_str.parse().map_err(|e: String| e)?;
            critique.set_severity(new_severity);
        }

        if let Some(status_str) = status {
            let new_status: CritiqueStatus = status_str.parse().map_err(|e: String| e)?;
            critique.set_status(new_status);
        }

        store.save_critique(&critique)?;
        println!("Updated critique {}", critique_id);
        Ok(())
    })
}

fn address_critique(critique_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Address critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        critique.address();
        store.save_critique(&critique)?;
        println!(
            "Critique {} marked as addressed (solution was modified to address it)",
            critique_id
        );
        Ok(())
    })
}

fn validate_critique(critique_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Validate critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        let solution_id = critique.solution_id.clone();

        critique.validate();
        store.save_critique(&critique)?;

        println!(
            "Critique {} validated (it's correct - the solution has a flaw)",
            critique_id
        );

        // Optionally refute the solution
        let solution = store.load_solution(&solution_id)?;
        if solution.status != SolutionStatus::Refuted {
            println!(
                "\nThe target solution {} should likely be refuted.",
                solution_id
            );
            println!("Use 'jjj solution refute {}' to mark it as refuted.", solution_id);
        }

        Ok(())
    })
}

fn dismiss_critique(critique_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Dismiss critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        critique.dismiss();
        store.save_critique(&critique)?;
        println!(
            "Critique {} dismissed (shown to be incorrect or irrelevant)",
            critique_id
        );
        Ok(())
    })
}

fn reply_to_critique(critique_id: String, body: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Reply to critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        let author = store.jj_client.user_identity()?;

        critique.add_reply(author.clone(), body.clone());
        store.save_critique(&critique)?;

        let reply_id = &critique.replies.last().unwrap().id;
        println!("Added reply {} to critique {}", reply_id, critique_id);

        Ok(())
    })
}
