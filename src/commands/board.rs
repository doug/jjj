use crate::error::Result;
use crate::jj::JjClient;
use crate::models::SolutionStatus;
use crate::storage::MetadataStore;

pub fn execute(json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let solutions = store.list_solutions()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&solutions)?);
        return Ok(());
    }

    // Group solutions by status
    let proposed: Vec<_> = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Proposed)
        .collect();
    let testing: Vec<_> = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Testing)
        .collect();
    let accepted: Vec<_> = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Accepted)
        .collect();
    let refuted: Vec<_> = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Refuted)
        .collect();

    // Calculate column widths
    let col_width = 30;
    let separator = "+".to_string() + &"-".repeat(col_width + 2);

    // Print header
    println!(
        "{}{}{}{}+",
        separator, separator, separator, separator
    );
    println!(
        "| {:<width$} | {:<width$} | {:<width$} | {:<width$} |",
        format!("PROPOSED ({})", proposed.len()),
        format!("TESTING ({})", testing.len()),
        format!("ACCEPTED ({})", accepted.len()),
        format!("REFUTED ({})", refuted.len()),
        width = col_width
    );
    println!(
        "{}{}{}{}+",
        separator, separator, separator, separator
    );

    // Find max rows
    let max_rows = proposed
        .len()
        .max(testing.len())
        .max(accepted.len())
        .max(refuted.len());

    if max_rows == 0 {
        println!(
            "| {:<width$} | {:<width$} | {:<width$} | {:<width$} |",
            "(empty)",
            "(empty)",
            "(empty)",
            "(empty)",
            width = col_width
        );
    } else {
        for i in 0..max_rows {
            let proposed_str = proposed.get(i).map_or(String::new(), |s| {
                format_solution_cell(s, &store, col_width)
            });
            let testing_str = testing.get(i).map_or(String::new(), |s| {
                format_solution_cell(s, &store, col_width)
            });
            let accepted_str = accepted.get(i).map_or(String::new(), |s| {
                format_solution_cell(s, &store, col_width)
            });
            let refuted_str = refuted.get(i).map_or(String::new(), |s| {
                format_solution_cell(s, &store, col_width)
            });

            println!(
                "| {:<width$} | {:<width$} | {:<width$} | {:<width$} |",
                proposed_str,
                testing_str,
                accepted_str,
                refuted_str,
                width = col_width
            );
        }
    }

    println!(
        "{}{}{}{}+",
        separator, separator, separator, separator
    );

    // Show summary
    println!();
    println!("Total: {} solutions", solutions.len());

    // Show problems summary
    let problems = store.list_problems()?;
    let open_problems = problems.iter().filter(|p| p.is_open()).count();
    let solved_problems = problems.iter().filter(|p| p.is_resolved()).count();
    println!(
        "Problems: {} open, {} solved/dissolved",
        open_problems, solved_problems
    );

    Ok(())
}

fn format_solution_cell(
    solution: &crate::models::Solution,
    store: &MetadataStore,
    max_width: usize,
) -> String {
    // Get critique count
    let critiques = store
        .get_open_critiques_for_solution(&solution.id)
        .unwrap_or_default();
    let critique_indicator = if !critiques.is_empty() {
        format!(" [{}!]", critiques.len())
    } else {
        String::new()
    };

    let title_max = max_width - solution.id.len() - critique_indicator.len() - 2;
    let truncated_title = if solution.title.len() > title_max {
        format!("{}...", &solution.title[..title_max.saturating_sub(3)])
    } else {
        solution.title.clone()
    };

    format!("{} {}{}", solution.id, truncated_title, critique_indicator)
}
