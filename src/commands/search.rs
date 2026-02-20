use crate::context::CommandContext;
use crate::db::{self, search, Database};
use crate::embeddings::EmbeddingClient;
use crate::error::Result;
use crate::local_config::LocalConfig;
use crate::resolve::parse_entity_reference;

pub fn execute(
    ctx: &CommandContext,
    query: &str,
    entity_type: Option<&str>,
    text_only: bool,
    json: bool,
) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    // Always sync from markdown before searching to ensure results are fresh,
    // since entity creation commands write to markdown but not the DB.
    let db = Database::open(&db_path)?;
    db::load_from_markdown(&db, &ctx.store)?;

    // Load local config and try to get embedding client
    let local_config = LocalConfig::load(repo_root);
    let embedding_client = if text_only {
        None
    } else {
        EmbeddingClient::from_config(&local_config, local_config.embeddings_explicitly_enabled())
    };

    // Check if query is an entity reference (e.g., "p/01957d")
    if let Some((ref_type, ref_id)) = parse_entity_reference(query) {
        return execute_similarity_search(&db, ref_type, ref_id, entity_type, json);
    }

    // Hybrid text search
    execute_hybrid_search(&db, query, entity_type, embedding_client.as_ref(), json)
}

fn execute_similarity_search(
    db: &Database,
    entity_type: &str,
    entity_id_prefix: &str,
    filter_type: Option<&str>,
    json: bool,
) -> Result<()> {
    // Resolve the entity ID prefix to full ID
    let conn = db.conn();
    let full_id = resolve_entity_id(conn, entity_type, entity_id_prefix)?;

    let results = search::find_similar(conn, entity_type, &full_id, filter_type, 20)?;

    if json {
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "type": r.entity_type,
                    "id": r.entity_id,
                    "title": r.title,
                    "similarity": r.similarity,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else if results.is_empty() {
        let type_char = entity_type.chars().next().unwrap_or('?');
        println!(
            "No similar entities found for {}/{}",
            type_char, entity_id_prefix
        );
        println!("\nNote: Embeddings may not be computed. Run 'jjj db rebuild' with an embedding service running.");
    } else {
        let type_char = entity_type.chars().next().unwrap_or('?');
        println!(
            "Entities similar to {}/{}:\n",
            type_char,
            &full_id[..6.min(full_id.len())]
        );
        for result in results {
            let short_id = &result.entity_id[..6.min(result.entity_id.len())];
            let result_type_char = result.entity_type.chars().next().unwrap_or('?');
            println!(
                "  {}/{}  [{:.2}]  \"{}\"",
                result_type_char, short_id, result.similarity, result.title
            );
        }
    }

    Ok(())
}

fn execute_hybrid_search(
    db: &Database,
    query: &str,
    entity_type: Option<&str>,
    embedding_client: Option<&EmbeddingClient>,
    json: bool,
) -> Result<()> {
    let conn = db.conn();

    // Always do FTS search
    let fts_results = search::search(conn, query, entity_type)?;

    // Try semantic search if client available
    let final_results = if let Some(client) = embedding_client {
        if let Ok(query_embedding) = client.embed(query) {
            let semantic_results =
                search::similarity_search(conn, &query_embedding, entity_type, None, 50)?;

            if !semantic_results.is_empty() {
                // Merge with RRF
                search::merge_with_rrf(fts_results, semantic_results, 60)
            } else {
                fts_results
            }
        } else {
            fts_results
        }
    } else {
        fts_results
    };

    if json {
        let json_results: Vec<_> = final_results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "type": r.entity_type,
                    "id": r.entity_id,
                    "title": r.title,
                    "snippet": r.snippet,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else if final_results.is_empty() {
        println!("No results found for \"{}\"", query);
    } else {
        let hybrid_note = if embedding_client.is_some() {
            " (hybrid)"
        } else {
            ""
        };
        println!(
            "Found {} result(s) for \"{}\"{}:\n",
            final_results.len(),
            query,
            hybrid_note
        );
        for result in final_results {
            println!(
                "[{}] {} - {}",
                result.entity_type, result.entity_id, result.title
            );
            if !result.snippet.is_empty() {
                println!("    {}", result.snippet.replace('\n', " "));
            }
            println!();
        }
    }

    Ok(())
}

/// Resolve an entity ID prefix to the full ID.
fn resolve_entity_id(
    conn: &rusqlite::Connection,
    entity_type: &str,
    prefix: &str,
) -> Result<String> {
    let table = match entity_type {
        "problem" => "problems",
        "solution" => "solutions",
        "critique" => "critiques",
        "milestone" => "milestones",
        _ => return Err(crate::error::JjjError::EntityNotFound(prefix.to_string())),
    };

    let sql = format!("SELECT id FROM {} WHERE id LIKE ?1 || '%'", table);
    let pattern = prefix;

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([pattern])?;

    let mut matches = Vec::new();
    while let Some(row) = rows.next()? {
        matches.push(row.get::<_, String>(0)?);
    }

    match matches.len() {
        0 => Err(crate::error::JjjError::EntityNotFound(prefix.to_string())),
        1 => Ok(matches.remove(0)),
        _ => Err(crate::error::JjjError::AmbiguousId {
            prefix: prefix.to_string(),
            matches,
        }),
    }
}
